use andromeda::projects::Registry;

#[test]
fn instruction_edits_preserve_concurrent_changes_and_remain_within_the_project() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    registry
        .save_instructions(&saved.id, "Observe first.", None)
        .unwrap();
    assert_eq!(
        registry.instructions(&saved.id).unwrap().as_deref(),
        Some("Observe first.")
    );
    std::fs::write(project.join("ANDROMEDA.md"), "A human changed this.").unwrap();
    assert_eq!(
        registry
            .save_instructions(&saved.id, "Replacement", Some("Observe first."))
            .unwrap_err()
            .code,
        "instructions_changed"
    );
    assert_eq!(
        registry.instructions(&saved.id).unwrap().as_deref(),
        Some("A human changed this.")
    );
}

#[test]
fn removing_a_project_only_removes_its_registration() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    let other = workspace.path().join("other");
    std::fs::create_dir(&project).unwrap();
    std::fs::create_dir(&other).unwrap();
    let marker = project.join("keep.txt");
    std::fs::write(&marker, "keep this").unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    let remaining = registry.register(&other).unwrap();

    registry.remove(&saved.id).unwrap();

    assert!(project.is_dir());
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "keep this");
    assert!(registry.get(&saved.id).is_err());
    let projects = registry.list().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, remaining.id);
}

#[test]
fn a_registered_project_rejects_mcp_startup_from_an_unrelated_directory() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    let child = project.join("src");
    let unrelated = workspace.path().join("project-other");
    std::fs::create_dir_all(&child).unwrap();
    std::fs::create_dir_all(&unrelated).unwrap();
    let registry = Registry::open(workspace.path().join("private-state")).unwrap();
    let saved = registry.register(&project).unwrap();
    assert!(registry.authorize(&saved.id, &child).is_ok());
    let error = registry.authorize(&saved.id, &unrelated).unwrap_err();
    assert_eq!(error.code, "project_scope_mismatch");
}

#[test]
fn disabled_project_stays_disabled_when_the_registry_is_reopened() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let state = workspace.path().join("state");
    let registry = Registry::open(state.clone()).unwrap();
    let saved = registry.register(&project).unwrap();
    registry
        .update(
            &saved.id,
            None,
            vec!["adm_get_view".into()],
            "ANDROMEDA.md".into(),
            false,
        )
        .unwrap();
    let reopened = Registry::open(state).unwrap();
    assert_eq!(
        reopened.authorize(&saved.id, &project).unwrap_err().code,
        "project_disabled"
    );
}

#[cfg(unix)]
#[test]
fn instruction_symlinks_cannot_escape_the_registered_project() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let outside = workspace.path().join("private.txt");
    std::fs::write(&outside, "outside instructions").unwrap();
    std::os::unix::fs::symlink(&outside, project.join("ANDROMEDA.md")).unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    assert_eq!(
        registry.instructions(&saved.id).unwrap_err().code,
        "project_scope_mismatch"
    );
}
