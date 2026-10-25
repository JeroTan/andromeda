//! Bounded child-process execution for client setup and bundled-server checks.
use andromeda::errors::{Error, Result};
use std::{path::Path, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt;

pub(super) async fn run(cli: &Path, root: &Path, args: &[&str], client: &str) -> Result<String> {
    let mut child = tokio::process::Command::new(cli)
        .args(args)
        .current_dir(root)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| Error::new("client_start_failed", format!("Could not start {client}")))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::new("client_start_failed", "Client stdout unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::new("client_start_failed", "Client stderr unavailable"))?;
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
                "Client setup output exceeded its limit",
            ));
        }
        if !status.success() {
            return Err(Error::new(
                "client_setup_failed",
                format!("{client} could not complete setup: {status}"),
            ));
        }
        String::from_utf8(out)
            .map_err(|_| Error::new("client_setup_failed", "Client returned invalid text"))
    };
    tokio::time::timeout(Duration::from_secs(20), operation)
        .await
        .map_err(|_| {
            Error::new(
                "timeout",
                format!("{client} timed out; check its MCP status before retrying"),
            )
        })?
}

pub(super) async fn check_server(server: &Path, root: &Path) -> Result<()> {
    let output = run(server, root, &["--version"], "Andromeda MCP server")
        .await.map_err(|error| Error::new("mcp_start_failed", format!("The bundled MCP server cannot start ({}). Reinstall the app using the verified bundle installer before reconnecting.", error.message)))?;
    if output.trim() != concat!("andromeda-mcp ", env!("CARGO_PKG_VERSION")) {
        return Err(Error::new(
            "mcp_version_mismatch",
            "The app and bundled MCP server versions differ; reinstall the complete app bundle",
        ));
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[tokio::test]
    async fn a_killed_bundled_server_is_reported_before_client_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let server = directory.path().join("andromeda");
        std::fs::write(&server, "#!/bin/sh\nkill -9 $$\n").unwrap();
        std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).unwrap();
        let error = check_server(&server, directory.path()).await.unwrap_err();
        assert_eq!(error.code, "mcp_start_failed");
        assert!(error.message.contains("9"));
        assert!(error.message.contains("Reinstall"));
    }

    #[tokio::test]
    async fn a_broken_app_version_is_not_reported_as_a_healthy_mcp_installation() {
        let directory = tempfile::tempdir().unwrap();
        let server = directory.path().join("andromeda");
        std::fs::write(&server, "#!/bin/sh\necho andromeda-mcp 99.99.99\n").unwrap();
        std::fs::set_permissions(&server, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            check_server(&server, directory.path())
                .await
                .unwrap_err()
                .code,
            "mcp_version_mismatch"
        );
    }
}
