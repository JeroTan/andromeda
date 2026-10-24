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

#[test]
fn opencode_setup_preserves_jsonc_settings_and_adds_a_scoped_v2_connection() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let path = project.join("opencode.jsonc");
    std::fs::write(
        &path,
        r#"{
  // Keep my chosen model and existing connection.
  "model": "custom/model",
  "mcp": {
    "timeout": { "execution": 42000 },
    "servers": {
      "existing": { "type": "local", "command": ["other"], },
    },
  },
}"#,
    )
    .unwrap();
    let server = workspace.path().join("Andromeda server");
    std::fs::write(&server, "fixture").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    // Serialized client names are the desktop's public command boundary.
    let client: Client = serde_json::from_str("\"opencode\"").unwrap();
    let change = registry
        .preview_client_config(&saved.id, client, &server)
        .unwrap();
    assert_eq!(change.path, path.canonicalize().unwrap());
    assert!(!change.replaces_existing);
    assert!(!change.after.contains("custom/model"));
    let entry: serde_json::Value = serde_json::from_str(&change.after).unwrap();
    assert_eq!(entry["type"], "local");
    assert_eq!(
        entry["command"][0],
        server.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(entry["command"][1], "--state-dir");
    assert_eq!(entry["command"][3], "managed");
    assert_eq!(entry["command"][4], "--project");
    assert_eq!(entry["command"][5], saved.id);
    assert_eq!(entry["timeout"]["execution"], 130000);
    assert!(entry.get("cwd").is_none());
    assert!(entry.get("environment").is_none());
    assert!(
        entry.get("enabled").is_none(),
        "OpenCode 2 uses disabled instead of enabled"
    );
    registry.apply_client_config(change, false).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("// Keep my chosen model and existing connection."));
    assert!(text.contains("\"model\": \"custom/model\""));
    assert!(text.contains("\"existing\": { \"type\": \"local\", \"command\": [\"other\"], }"));
    assert!(text.contains("\"execution\": 42000"));
}

#[test]
fn opencode_setup_selects_existing_config_and_fences_conflicts_and_concurrent_edits() {
    for name in [
        "opencode.json",
        "opencode.jsonc",
        ".opencode/opencode.json",
        ".opencode/opencode.jsonc",
    ] {
        let workspace = tempfile::tempdir().unwrap();
        let project = workspace.path().join("project");
        std::fs::create_dir_all(project.join(".opencode")).unwrap();
        let path = project.join(name);
        let original = r#"{"model":"keep/model","mcp":{"servers":{"andromeda":{"type":"local","command":["different"]}}}}"#;
        std::fs::write(&path, original).unwrap();
        let server = workspace.path().join("andromeda");
        std::fs::write(&server, "fixture").unwrap();
        let registry = Registry::open(workspace.path().join("state")).unwrap();
        let saved = registry.register(&project).unwrap();
        let change = registry
            .preview_client_config(&saved.id, Client::Opencode, &server)
            .unwrap();
        assert_eq!(change.path, path.canonicalize().unwrap());
        assert!(change.replaces_existing);
        assert_eq!(
            registry
                .apply_client_config(change, false)
                .unwrap_err()
                .code,
            "replacement_required"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let change = registry
            .preview_client_config(&saved.id, Client::Opencode, &server)
            .unwrap();
        std::fs::write(&path, "{\"concurrent\":true}").unwrap();
        assert_eq!(
            registry.apply_client_config(change, true).unwrap_err().code,
            "config_changed"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\"concurrent\":true}"
        );
        let change = registry
            .preview_client_config(&saved.id, Client::Opencode, &server)
            .unwrap();
        registry.apply_client_config(change, false).unwrap();
        let repeat = registry
            .preview_client_config(&saved.id, Client::Opencode, &server)
            .unwrap();
        assert!(
            !repeat.replaces_existing,
            "An identical connection needs no replacement grant"
        );
    }
}

#[test]
fn opencode_setup_creates_jsonc_but_rejects_invalid_or_legacy_config_without_writing() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let server = workspace.path().join("andromeda");
    std::fs::write(&server, "fixture").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    let path = project.join("opencode.jsonc");
    let change = registry
        .preview_client_config(&saved.id, Client::Opencode, &server)
        .unwrap();
    assert_eq!(change.path, saved.root.join("opencode.jsonc"));
    registry.apply_client_config(change, false).unwrap();
    let created: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(created["mcp"]["servers"]["andromeda"]["type"], "local");
    for (input, code) in [
        ("[]", "invalid_config"),
        ("{\"mcp\":[]}", "invalid_config"),
        ("{\"mcp\":{\"servers\":false}}", "invalid_config"),
        ("{model:'loose'}", "invalid_config"),
        ("{\"mcp\":{},\"mcp\":{}}", "invalid_config"),
        (
            "{\"mcp\":{\"servers\":{},\"servers\":{}}}",
            "invalid_config",
        ),
        (
            "{\"mcp\":{\"old\":{\"type\":\"local\",\"command\":[\"old\"]}}}",
            "opencode_version_mismatch",
        ),
    ] {
        std::fs::write(&path, input).unwrap();
        assert_eq!(
            registry
                .preview_client_config(&saved.id, Client::Opencode, &server)
                .unwrap_err()
                .code,
            code
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), input);
    }
}

#[test]
fn opencode_new_higher_precedence_config_invalidates_a_pending_preview() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let server = workspace.path().join("andromeda");
    std::fs::write(&server, "fixture").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    let change = registry
        .preview_client_config(&saved.id, Client::Opencode, &server)
        .unwrap();
    std::fs::create_dir(project.join(".opencode")).unwrap();
    std::fs::write(project.join(".opencode/opencode.jsonc"), "{\"keep\":true}").unwrap();
    assert_eq!(
        registry
            .apply_client_config(change, false)
            .unwrap_err()
            .code,
        "config_changed"
    );
    assert!(!project.join("opencode.jsonc").exists());
}

#[cfg(unix)]
#[test]
fn opencode_config_symlinks_cannot_escape_the_registered_project() {
    for relative in ["opencode.jsonc", ".opencode"] {
        let workspace = tempfile::tempdir().unwrap();
        let project = workspace.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let outside = workspace.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        let outside_file = outside.join("opencode.jsonc");
        std::fs::write(&outside_file, "{}").unwrap();
        std::os::unix::fs::symlink(
            if relative == ".opencode" {
                &outside
            } else {
                &outside_file
            },
            project.join(relative),
        )
        .unwrap();
        let server = workspace.path().join("andromeda");
        std::fs::write(&server, "fixture").unwrap();
        let registry = Registry::open(workspace.path().join("state")).unwrap();
        let saved = registry.register(&project).unwrap();
        assert_eq!(
            registry
                .preview_client_config(&saved.id, Client::Opencode, &server)
                .unwrap_err()
                .code,
            "project_scope_mismatch"
        );
        assert_eq!(std::fs::read_to_string(outside_file).unwrap(), "{}");
    }
}
