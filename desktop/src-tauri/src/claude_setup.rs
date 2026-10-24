//! User-initiated Claude setup via its supported project-local CLI scope.
use andromeda::errors::{Error, Result};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::io::AsyncReadExt;

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
    pub async fn apply(self) -> Result<()> {
        if self.root.canonicalize()? != self.root
            || existing(&self.config_path, &self.root)? != self.previous
        {
            return Err(Error::new(
                "config_changed",
                "Claude or the project changed after preview; open Connect Claude again",
            ));
        }
        if self.previous.is_none() {
            let configuration = serde_json::to_string(&self.connection)
                .map_err(|_| Error::new("invalid_config", "Cannot encode Claude connection"))?;
            run(
                &self.cli,
                &self.root,
                &[
                    "mcp",
                    "add-json",
                    "--scope",
                    "local",
                    "andromeda",
                    &configuration,
                ],
            )
            .await?;
        }
        let status = run(&self.cli, &self.root, &["mcp", "get", "andromeda"]).await?;
        if !status
            .lines()
            .any(|line| line.trim_start().starts_with("Status:") && line.contains("Connected"))
        {
            return Err(Error::new(
                "claude_not_connected",
                "Claude registration is saved, but its connection check did not succeed. Keep Andromeda open and check Claude's MCP status.",
            ));
        }
        Ok(())
    }
}
async fn run(cli: &Path, root: &Path, args: &[&str]) -> Result<String> {
    let mut child = tokio::process::Command::new(cli)
        .args(args)
        .current_dir(root)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            Error::new(
                "claude_start_failed",
                "Could not start the installed Claude Code executable",
            )
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::new("claude_start_failed", "Claude stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::new("claude_start_failed", "Claude stderr unavailable"))?;
    let operation = async {
        let read_out = async {
            let mut bytes = Vec::new();
            stdout.take(65537).read_to_end(&mut bytes).await?;
            Ok::<_, std::io::Error>(bytes)
        };
        let read_err = async {
            let mut bytes = Vec::new();
            stderr.take(65537).read_to_end(&mut bytes).await?;
            Ok::<_, std::io::Error>(bytes)
        };
        let (status, out, err) = tokio::try_join!(child.wait(), read_out, read_err)?;
        if out.len() > 65536 || err.len() > 65536 {
            return Err(Error::new(
                "output_limit",
                "Claude setup output exceeded its limit",
            ));
        }
        if !status.success() {
            return Err(Error::new(
                "claude_setup_failed",
                format!(
                    "Claude could not complete setup: {}",
                    String::from_utf8_lossy(&err)
                        .chars()
                        .take(400)
                        .collect::<String>()
                ),
            ));
        }
        String::from_utf8(out)
            .map_err(|_| Error::new("claude_setup_failed", "Claude returned invalid text"))
    };
    tokio::time::timeout(Duration::from_secs(20), operation)
        .await
        .map_err(|_| {
            Error::new(
                "timeout",
                "Claude setup timed out; check its MCP status before retrying",
            )
        })?
}
