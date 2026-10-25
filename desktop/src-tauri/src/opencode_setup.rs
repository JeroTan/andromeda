//! Verify the selected project's actual OpenCode connection; reconnect stale failure once.
use super::cli_process::run;
use andromeda::errors::{Error, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) fn find_cli(root: &Path) -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "opencode.exe"
    } else {
        "opencode"
    };
    let mut paths = Vec::new();
    if let Some(path) = std::env::var_os("ANDROMEDA_OPENCODE_BIN") {
        paths.push(PathBuf::from(path));
    } else {
        let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        if let Some(home) = std::env::var_os(variable) {
            let home = PathBuf::from(home);
            paths.push(home.join(".opencode/bin").join(name));
            paths.push(home.join(".local/bin").join(name));
        }
        paths.extend([
            PathBuf::from("/opt/homebrew/bin").join(name),
            PathBuf::from("/usr/local/bin").join(name),
        ]);
        paths.extend(
            std::env::var_os("PATH")
                .into_iter()
                .flat_map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
                .filter(|path| path.is_absolute())
                .map(|path| path.join(name)),
        );
    }
    paths.into_iter().filter_map(|p| p.canonicalize().ok())
        .find(|p| p.is_file() && !p.starts_with(root))
        .ok_or_else(|| Error::new("opencode_not_found", "Install OpenCode 2 before connecting. Advanced: ANDROMEDA_OPENCODE_BIN can select its executable."))
}

fn query(root: &Path) -> String {
    let path = root.to_string_lossy();
    let mut encoded = String::new();
    for byte in path.as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(byte) {
            encoded.push(*byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    format!("location%5Bdirectory%5D={encoded}")
}

async fn status(cli: &Path, root: &Path) -> Result<String> {
    let path = format!("/api/mcp?{}", query(root));
    let output = run(cli, root, &["api", "GET", &path], "OpenCode").await?;
    let response: Value = serde_json::from_str(&output).map_err(|_| {
        Error::new(
            "opencode_invalid_response",
            "OpenCode returned an invalid connection status",
        )
    })?;
    let actual = response["location"]["directory"].as_str().ok_or_else(|| {
        Error::new(
            "opencode_invalid_response",
            "OpenCode omitted the project directory",
        )
    })?;
    if Path::new(actual).canonicalize()? != root {
        return Err(Error::new(
            "project_scope_mismatch",
            "OpenCode checked a different project; no reconnect was attempted",
        ));
    }
    let servers = response["data"].as_array().ok_or_else(|| {
        Error::new(
            "opencode_invalid_response",
            "OpenCode omitted its MCP server statuses",
        )
    })?;
    let state = servers
        .iter()
        .find(|entry| entry["name"] == "andromeda")
        .and_then(|entry| entry["status"]["status"].as_str())
        .unwrap_or("missing");
    Ok(state.to_owned())
}

pub(super) async fn verify(
    cli: &Path,
    root: &Path,
    connection: &Value,
    changed: bool,
) -> Result<String> {
    let state = status(cli, root).await?;
    if state == "connected" && !changed {
        return Ok("OpenCode's connection check passed for this project.".into());
    }
    // Apply the exact reviewed connection only in this project. Do not reload
    // the shared server: that would interrupt unrelated projects and sessions.
    let path = format!("/api/experimental/mcp/andromeda?{}", query(root));
    let body = serde_json::json!({"config": connection}).to_string();
    run(
        cli,
        root,
        &["api", "PUT", &path, "--data", &body],
        "OpenCode connection",
    )
    .await?;
    if status(cli, root).await? != "connected" {
        return Err(Error::new(
            "opencode_not_connected",
            "Connection saved, but OpenCode could not start Andromeda. Check this project's MCP status.",
        ));
    }
    Ok("OpenCode reconnected Andromeda and verified this project's connection.".into())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Fixture {
        _directory: tempfile::TempDir,
        root: PathBuf,
        cli: PathBuf,
        log: PathBuf,
    }

    impl Fixture {
        fn new(initial: &str, wrong_root: bool) -> Self {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path().join("project space 雲");
            std::fs::create_dir(&root).unwrap();
            let root = root.canonicalize().unwrap();
            let cli = directory.path().join("opencode");
            let log = directory.path().join("commands.jsonl");
            let script = format!(
                r#"#!/usr/bin/env python3
import json,sys,pathlib,urllib.parse
root={root}
log=pathlib.Path({log})
state=log.with_suffix('.state')
args=sys.argv[1:]
with log.open('a') as file:file.write(json.dumps(args)+'\n')
assert args[0]=='api'
path=args[2]
query=urllib.parse.parse_qs(urllib.parse.urlsplit(path).query)
assert query['location[directory]']==[root]
if args[1]=='GET':
 status='connected' if state.exists() else {initial}
 data=[] if status=='missing' else [{{'name':'andromeda','status':{{'status':status}}}}]
 print(json.dumps({{'location':{{'directory':str(pathlib.Path(root).parent) if {wrong_root} else root}},'data':data}}))
elif args[1]=='PUT':
 assert path.startswith('/api/experimental/mcp/andromeda?')
 assert json.loads(args[args.index('--data')+1])=={{'config':{{'type':'local','command':['reviewed-server']}}}}
 state.write_text('connected')
else:raise Exception('Unapproved command')
"#,
                root = serde_json::to_string(root.to_str().unwrap()).unwrap(),
                log = serde_json::to_string(log.to_str().unwrap()).unwrap(),
                initial = serde_json::to_string(initial).unwrap(),
                wrong_root = if wrong_root { "True" } else { "False" }
            );
            std::fs::write(&cli, script).unwrap();
            std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
            Self {
                _directory: directory,
                root,
                cli,
                log,
            }
        }
        fn calls(&self) -> Vec<serde_json::Value> {
            std::fs::read_to_string(&self.log)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        }
    }

    #[tokio::test]
    async fn failed_and_missing_servers_are_connected_using_exact_reviewed_project_config() {
        for initial in ["failed", "missing"] {
            let f = Fixture::new(initial, false);
            let result = verify(
                &f.cli,
                &f.root,
                &serde_json::json!({"type":"local","command":["reviewed-server"]}),
                false,
            )
            .await
            .unwrap();
            assert!(result.contains("verified"));
            let calls = f.calls();
            assert_eq!(calls.len(), 3);
            assert_eq!(calls[0][1], "GET");
            assert_eq!(calls[1][1], "PUT");
            assert_eq!(calls[2][1], "GET");
            assert!(
                calls
                    .iter()
                    .all(|call| !call.to_string().contains("reload"))
            );
        }
    }

    #[tokio::test]
    async fn an_unchanged_healthy_connection_is_checked_without_restarting_it() {
        let f = Fixture::new("connected", false);
        verify(&f.cli, &f.root, &serde_json::json!({}), false)
            .await
            .unwrap();
        assert_eq!(f.calls().len(), 1);
    }

    #[tokio::test]
    async fn a_different_project_response_never_mutates_that_projects_mcp() {
        let f = Fixture::new("failed", true);
        let error = verify(&f.cli, &f.root, &serde_json::json!({}), false)
            .await
            .unwrap_err();
        assert_eq!(error.code, "project_scope_mismatch");
        assert_eq!(f.calls().len(), 1);
    }
}
