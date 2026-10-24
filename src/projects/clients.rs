use super::Registry;
use super::{atomic_write, config_io, read_bounded};
use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Client {
    Claude,
    Codex,
    Opencode,
}

#[derive(Debug, Serialize)]
pub struct ConfigChange {
    pub path: PathBuf,
    pub client: Client,
    pub replaces_existing: bool,
    pub before: String,
    pub after: String,
    #[serde(skip)]
    project_id: String,
    #[serde(skip)]
    previous: Option<Vec<u8>>,
    #[serde(skip)]
    next: Vec<u8>,
}

impl Registry {
    pub fn preview_client_config(
        &self,
        id: &str,
        client: Client,
        server: &Path,
    ) -> Result<ConfigChange> {
        let project = self.get(id)?;
        let path = config_path(&project.root, client)?;
        let server = server.canonicalize()?;
        if !server.is_file() {
            return Err(Error::invalid("MCP server executable must be a file"));
        }
        let server = server
            .to_str()
            .ok_or_else(|| Error::invalid("Executable path must be UTF-8"))?;
        let directory = self
            .directory
            .to_str()
            .ok_or_else(|| Error::invalid("Application state path must be UTF-8"))?;
        let args = ["--state-dir", directory, "managed", "--project", id];
        let previous = if path.try_exists().map_err(config_io)? {
            Some(read_bounded(&path, 1024 * 1024)?)
        } else {
            None
        };
        let text = previous
            .as_deref()
            .map(std::str::from_utf8)
            .transpose()
            .map_err(|_| Error::new("invalid_config", "Client configuration must be UTF-8"))?
            .unwrap_or("");
        let (next, before, after, replaces_existing) = match client {
            Client::Opencode => super::opencode::edit(text, server, &args)?,
            Client::Claude => {
                let mut document: Value = if text.trim().is_empty() {
                    json!({})
                } else {
                    serde_json::from_str(text)
                        .map_err(|e| Error::new("invalid_config", e.to_string()))?
                };
                let root = document.as_object_mut().ok_or_else(|| {
                    Error::new(
                        "invalid_config",
                        "Claude configuration must be a JSON object",
                    )
                })?;
                let servers = root
                    .entry("mcpServers")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .ok_or_else(|| {
                        Error::new("invalid_config", "mcpServers must be a JSON object")
                    })?;
                let old = servers.get("andromeda").cloned();
                let owned = old
                    .as_ref()
                    .and_then(|v| v.get("args"))
                    .and_then(Value::as_array)
                    .is_some_and(|a| {
                        a.windows(2)
                            .any(|pair| pair[0] == "--project" && pair[1] == id)
                    });
                let connection = json!({"type":"stdio","command":server,"args":args});
                let before = old.as_ref().map(pretty_json).unwrap_or_default();
                let after = pretty_json(&connection);
                servers.insert("andromeda".into(), connection);
                (
                    format!("{}\n", pretty_json(&document)).into_bytes(),
                    before,
                    after,
                    old.is_some() && !owned,
                )
            }
            Client::Codex => {
                let mut document: toml_edit::DocumentMut =
                    text.parse().map_err(|e: toml_edit::TomlError| {
                        Error::new("invalid_config", e.to_string())
                    })?;
                if document.get("mcp_servers").is_none() {
                    document["mcp_servers"] = toml_edit::table();
                }
                let inline = document["mcp_servers"].is_inline_table();
                let servers = document["mcp_servers"].as_table_like_mut().ok_or_else(|| {
                    Error::new("invalid_config", "mcp_servers must be a TOML table")
                })?;
                let old = servers.get("andromeda").cloned();
                let owned = old
                    .as_ref()
                    .and_then(|i| i.as_table_like())
                    .and_then(|t| t.get("args"))
                    .and_then(toml_edit::Item::as_array)
                    .is_some_and(|a| {
                        let values: Vec<_> =
                            a.iter().filter_map(toml_edit::Value::as_str).collect();
                        values.windows(2).any(|pair| pair == ["--project", id])
                    });
                let mut connection = toml_edit::Table::new();
                connection["command"] = toml_edit::value(server);
                connection["args"] =
                    toml_edit::value(args.into_iter().collect::<toml_edit::Array>());
                connection["startup_timeout_sec"] = toml_edit::value(10);
                connection["tool_timeout_sec"] = toml_edit::value(130);
                let before = old.as_ref().map(ToString::to_string).unwrap_or_default();
                let after = connection.to_string();
                let entry = if inline {
                    toml_edit::Item::Value(connection.into_inline_table().into())
                } else {
                    toml_edit::Item::Table(connection)
                };
                servers.insert("andromeda", entry);
                (
                    document.to_string().into_bytes(),
                    before,
                    after,
                    old.is_some() && !owned,
                )
            }
        };
        Ok(ConfigChange {
            path,
            client,
            replaces_existing,
            before,
            after,
            project_id: id.into(),
            previous,
            next,
        })
    }
    pub fn apply_client_config(&self, change: ConfigChange, allow_replacement: bool) -> Result<()> {
        let project = self.get(&change.project_id)?;
        let path = config_path(&project.root, change.client)?;
        if path != change.path {
            return Err(Error::new(
                "config_changed",
                "Project configuration path changed; preview again",
            ));
        }
        if change.replaces_existing && !allow_replacement {
            return Err(Error::new(
                "replacement_required",
                "Review and confirm replacing the existing Andromeda connection",
            ));
        }
        let current = if path.try_exists().map_err(config_io)? {
            Some(read_bounded(&path, 1024 * 1024)?)
        } else {
            None
        };
        if current != change.previous {
            return Err(Error::new(
                "config_changed",
                "Client configuration changed after preview; review a fresh preview",
            ));
        }
        fs::create_dir_all(
            path.parent().ok_or_else(|| {
                Error::invalid("Client configuration requires a parent directory")
            })?,
        )?;
        config_path(&project.root, change.client)?;
        atomic_write(&path, &change.next)
    }
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".into())
}

fn config_path(root: &Path, client: Client) -> Result<PathBuf> {
    if root.canonicalize()? != root {
        return Err(Error::new(
            "project_scope_mismatch",
            "Registered project path changed",
        ));
    }
    let relative = match client {
        Client::Claude => ".mcp.json",
        Client::Codex => ".codex/config.toml",
        Client::Opencode => super::opencode::config_name(root)?,
    };
    let path = root.join(relative);
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::new(
            "project_scope_mismatch",
            "Client configuration cannot be a symbolic link",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| Error::invalid("Client configuration needs a parent"))?;
    if parent.try_exists().map_err(config_io)?
        && !parent.canonicalize().map_err(config_io)?.starts_with(root)
    {
        return Err(Error::new(
            "project_scope_mismatch",
            "Client configuration directory resolves outside this project",
        ));
    }
    Ok(path)
}
