//! User-initiated Claude setup via its supported project-local CLI scope.
use super::cli_process::run;
use andromeda::errors::{Error, Result};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(super) struct Plan {
    pub project_id: String,
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub connection: Value,
    cli: PathBuf,
    previous: Option<Value>,
}
fn home() -> Result<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| Error::new("claude_not_found", "Cannot locate your Claude installation"))
}
fn executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path
            .metadata()
            .is_ok_and(|m| m.permissions().mode() & 0o111 == 0)
        {
            return false;
        }
    }
    true
}
fn find_cli(root: &Path) -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    };
    let mut paths = Vec::new();
    if let Some(path) = std::env::var_os("ANDROMEDA_CLAUDE_BIN") {
        paths.push(PathBuf::from(path));
    } else {
        let home = home()?;
        paths.push(home.join(".local/bin").join(name));
        let mut extensions: Vec<_> = std::fs::read_dir(home.join(".vscode/extensions"))
            .into_iter()
            .flatten()
            .filter_map(std::result::Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let version = name
                    .strip_prefix("anthropic.claude-code-")?
                    .split('-')
                    .next()?;
                let parts: Vec<u64> = version
                    .split('.')
                    .map(str::parse)
                    .collect::<std::result::Result<_, _>>()
                    .ok()?;
                Some((parts, entry.path()))
            })
            .collect();
        extensions.sort_by(|a, b| b.0.cmp(&a.0));
        paths.extend(
            extensions
                .into_iter()
                .map(|(_, path)| path.join("resources/native-binary").join(name)),
        );
        paths.extend(
            std::env::var_os("PATH")
                .into_iter()
                .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
                .filter(|path| path.is_absolute())
                .map(|path| path.join(name)),
        );
    }
    paths.into_iter().filter(|path| executable(path)).filter_map(|path| path.canonicalize().ok())
        .find(|path| !path.starts_with(root))
        .ok_or_else(|| Error::new("claude_not_found", "Install Claude Code or its VS Code extension, then try Connect Claude again. Advanced: ANDROMEDA_CLAUDE_BIN can select its executable."))
}
fn project_key(root: &Path) -> String {
    let path = root.to_string_lossy();
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{path}")
    } else {
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
    }
}
fn existing(path: &Path, root: &Path) -> Result<Option<Value>> {
    if !path.exists() {
        return Ok(None);
    }
    if path.metadata()?.len() > 8 * 1024 * 1024 {
        return Err(Error::new(
            "invalid_config",
            "Claude configuration exceeds 8 MiB",
        ));
    }
    let document: Value = serde_json::from_slice(&std::fs::read(path)?).map_err(|_| {
        Error::new(
            "invalid_config",
            "Claude's local configuration is not valid JSON",
        )
    })?;
    Ok(
        document["projects"][project_key(root)]["mcpServers"]["andromeda"]
            .as_object()
            .map(|v| Value::Object(v.clone())),
    )
}
fn matches_connection(existing: &Value, expected: &Value) -> bool {
    existing["command"] == expected["command"]
        && existing["args"] == expected["args"]
        && existing.get("type").is_none_or(|v| v == "stdio")
        && existing
            .get("env")
            .is_none_or(|v| v.as_object().is_some_and(|v| v.is_empty()))
}
impl Plan {
    pub fn new(
        project_id: String,
        root: PathBuf,
        server: &Path,
        state_directory: &Path,
    ) -> Result<Self> {
        let cli = find_cli(&root)?;
        let config_path = match std::env::var_os("CLAUDE_CONFIG_DIR") {
            Some(directory) => PathBuf::from(directory).join(".claude.json"),
            None => home()?.join(".claude.json"),
        };
        let previous = existing(&config_path, &root)?;
        let connection = json!({"type":"stdio","command":server,"args":["--state-dir",state_directory,"managed","--project",project_id]});
        if previous
            .as_ref()
            .is_some_and(|old| !matches_connection(old, &connection))
        {
            return Err(Error::new(
                "claude_connection_conflict",
                "Claude already has a different local server named andromeda in this project. Review it in Claude before replacing it.",
            ));
        }
        Ok(Self {
            project_id,
            root,
            config_path,
            connection,
            cli,
            previous,
        })
    }
    pub async fn apply(self) -> Result<String> {
        if self.root.canonicalize()? != self.root
            || existing(&self.config_path, &self.root)? != self.previous
        {
            return Err(Error::new(
                "config_changed",
                "Claude or the project changed after preview; open Connect Claude again",
            ));
        }
        let added = self.previous.is_none();
        if added {
            let command = self.connection["command"]
                .as_str()
                .ok_or_else(|| Error::new("invalid_config", "Claude command is missing"))?;
            let mut arguments = vec![
                "mcp",
                "add",
                "andromeda",
                "--scope",
                "local",
                "--transport",
                "stdio",
                "--",
                command,
            ];
            let server_args = self.connection["args"].as_array().ok_or_else(|| {
                Error::new("invalid_config", "Claude server arguments are missing")
            })?;
            for argument in server_args {
                arguments.push(argument.as_str().ok_or_else(|| {
                    Error::new("invalid_config", "Claude server argument is not text")
                })?);
            }
            run(&self.cli, &self.root, &arguments, "Claude").await?;
        }
        let status = run(
            &self.cli,
            &self.root,
            &["mcp", "get", "andromeda"],
            "Claude",
        )
        .await?;
        if !status
            .lines()
            .any(|line| line.trim_start().starts_with("Status:") && line.contains("Connected"))
        {
            return Err(Error::new(
                "claude_not_connected",
                "Claude registration is saved, but its connection check did not succeed. Keep Andromeda open and check Claude's MCP status.",
            ));
        }
        Ok(if added {
            "Andromeda ran Claude's add command and verified the connection."
        } else {
            "Andromeda is already registered for this project. Claude's connection check passed."
        }
        .into())
    }
}
