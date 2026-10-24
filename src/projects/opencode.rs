//! Project-local OpenCode 2 configuration edits that retain JSONC comments.
use crate::errors::{Error, Result};
use jsonc_parser::json;
use jsonc_parser::{ParseOptions, cst::CstRootNode};
use std::{fs, path::Path};

pub(super) fn config_name(root: &Path) -> Result<&'static str> {
    // OpenCode loads .opencode after direct project files, with JSONC last.
    for name in [
        ".opencode/opencode.jsonc",
        ".opencode/opencode.json",
        "opencode.jsonc",
        "opencode.json",
    ] {
        match fs::symlink_metadata(root.join(name)) {
            Ok(_) => return Ok(name),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok("opencode.jsonc")
}

pub(super) fn edit(
    text: &str,
    server: &str,
    args: &[&str],
) -> Result<(Vec<u8>, String, String, bool)> {
    let options = ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    };
    let document = CstRootNode::parse(text, &options).map_err(|_| {
        Error::new(
            "invalid_config",
            "OpenCode configuration is not valid JSONC",
        )
    })?;
    let root = document.object_value_or_create().ok_or_else(|| {
        Error::new(
            "invalid_config",
            "OpenCode configuration must be a JSON object",
        )
    })?;
    let mcp = root
        .object_value_or_create("mcp")
        .ok_or_else(|| Error::new("invalid_config", "OpenCode mcp must be a JSON object"))?;
    // A v1 map must not be silently converted into a mixed-version document.
    if mcp.properties().iter().any(|property| {
        property
            .decoded_name()
            .is_some_and(|name| !matches!(name.as_str(), "servers" | "timeout"))
    }) {
        return Err(Error::new(
            "opencode_version_mismatch",
            "This connector requires OpenCode 2. Migrate the existing v1 MCP configuration before connecting.",
        ));
    }
    let servers = mcp.object_value_or_create("servers").ok_or_else(|| {
        Error::new(
            "invalid_config",
            "OpenCode mcp.servers must be a JSON object",
        )
    })?;
    // Duplicate keys are ambiguous to readers; never edit one while another wins.
    for (object, name) in [(&root, "mcp"), (&mcp, "servers"), (&servers, "andromeda")] {
        if object
            .properties()
            .iter()
            .filter(|p| p.decoded_name().as_deref() == Some(name))
            .count()
            > 1
        {
            return Err(Error::new(
                "invalid_config",
                "OpenCode configuration has duplicate MCP keys",
            ));
        }
    }
    let old = servers.get("andromeda").and_then(|p| p.value());
    let command: Vec<_> = std::iter::once(server)
        .chain(args.iter().copied())
        .collect();
    let connection = serde_json::json!({
        "type": "local", "command": command,
        "timeout": {"startup": 10000, "catalog": 10000, "execution": 130000}
    });
    let previous = old.as_ref().and_then(|node| node.to_serde_value());
    let replaces = old.is_some() && previous.as_ref() != Some(&connection);
    let before = old.map(|node| node.to_string()).unwrap_or_default();
    let after = serde_json::to_string_pretty(&connection)
        .map_err(|_| Error::new("invalid_config", "Cannot encode OpenCode connection"))?;
    let value = jsonc_parser::json!({
        "type": "local", "command": command,
        "timeout": {"startup": 10000, "catalog": 10000, "execution": 130000}
    });
    if let Some(existing) = servers.get("andromeda") {
        existing.set_value(value);
    } else {
        servers.append("andromeda", value);
    }
    let mut next = document.to_string();
    if !next.ends_with('\n') {
        next.push('\n');
    }
    Ok((next.into_bytes(), before, after, replaces))
}
