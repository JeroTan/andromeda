use andromeda::projects::{Client, Registry};

#[test]
fn claude_project_setup_preserves_other_servers_and_keeps_credentials_out_of_git() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let path = project.join(".mcp.json");
    std::fs::write(&path,r#"{"mcpServers":{"existing":{"command":"existing-server","env":{"TOKEN":"keep-me"}}},"other":{"setting":true}}"#).unwrap();
    let server = workspace.path().join("andromeda");
    std::fs::write(&server, "executable fixture").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    let change = registry
        .preview_client_config(&saved.id, Client::Claude, &server)
        .unwrap();
    assert!(
        !change.after.contains("keep-me"),
        "Preview must show only the changed connection"
    );
    registry.apply_client_config(change, false).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(json["mcpServers"]["existing"]["env"]["TOKEN"], "keep-me");
    assert_eq!(json["other"]["setting"], true);
    assert!(
        json["mcpServers"]["andromeda"]["args"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(saved.id))
    );
    assert!(
        !json["mcpServers"]["andromeda"]
            .to_string()
            .contains("credential")
    );
}

#[test]
fn codex_setup_preserves_user_settings_and_adds_a_managed_connection() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir_all(project.join(".codex")).unwrap();
    let path = project.join(".codex/config.toml");
    std::fs::write(
        &path,
        "# Keep this comment\nmodel = 'custom-model'\n[mcp_servers.existing]\ncommand = 'other'\n",
    )
    .unwrap();
    let server = workspace.path().join("andromeda");
    std::fs::write(&server, "fixture").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    let change = registry
        .preview_client_config(&saved.id, Client::Codex, &server)
        .unwrap();
    registry.apply_client_config(change, false).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let config = text.parse::<toml_edit::DocumentMut>().unwrap();
    assert!(text.contains("# Keep this comment"));
    assert_eq!(config["model"].as_str(), Some("custom-model"));
    assert_eq!(
        config["mcp_servers"]["existing"]["command"].as_str(),
        Some("other")
    );
    assert!(
        config["mcp_servers"]["andromeda"]["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("managed"))
    );
    assert!(
        config["mcp_servers"]["andromeda"].get("cwd").is_none(),
        "The actual caller directory must remain available for scope validation"
    );
}

#[test]
fn configuration_changes_after_preview_are_not_overwritten() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let server = workspace.path().join("andromeda");
    std::fs::write(&server, "fixture").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    let change = registry
        .preview_client_config(&saved.id, Client::Claude, &server)
        .unwrap();
    let path = project.join(".mcp.json");
    std::fs::write(&path, "{\"concurrent_edit\":true}").unwrap();
    assert_eq!(
        registry
            .apply_client_config(change, false)
            .unwrap_err()
            .code,
        "config_changed"
    );
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "{\"concurrent_edit\":true}"
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_client_configuration_directory_cannot_write_outside_the_project() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    let outside = workspace.path().join("outside");
    std::fs::create_dir(&project).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, project.join(".codex")).unwrap();
    let server = workspace.path().join("andromeda");
    std::fs::write(&server, "fixture").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    assert_eq!(
        registry
            .preview_client_config(&saved.id, Client::Codex, &server)
            .unwrap_err()
            .code,
        "project_scope_mismatch"
    );
    assert!(!outside.join("config.toml").exists());
}
