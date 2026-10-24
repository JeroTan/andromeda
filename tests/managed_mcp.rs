use andromeda::{
    actions::Controller,
    device::Adb,
    managed::{Client, Host, Proxy},
    projects::Registry,
};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientConfig, Implementation},
};
use serde_json::json;
mod support;

#[tokio::test]
async fn desktop_reports_when_a_managed_mcp_client_connects_and_disconnects() {
    let workspace = tempfile::tempdir().unwrap();
    let directory = workspace.path().join("project");
    std::fs::create_dir(&directory).unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let project = registry.register(&directory).unwrap();
    let fixture = support::DeviceFixture::start(0).await;
    let host = Host::start(
        registry.clone(),
        Controller::new(Adb::new(fixture.address), None, true),
    )
    .await
    .unwrap();
    let proxy = Proxy::connect(
        Client::new(registry.clone(), project.id.clone(), directory.clone()).unwrap(),
    )
    .await
    .unwrap();
    assert!(
        host.connections().is_empty(),
        "A proxy waiting for initialize is not a connected MCP client"
    );
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = tokio::spawn(async move {
        proxy
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("connection-test", "1"),
    )
    .serve(client_io)
    .await
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while host.connections().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        host.connections()
            .iter()
            .find(|status| status.project_id == project.id)
            .unwrap()
            .clients,
        1
    );
    std::fs::write(directory.join("ANDROMEDA.md"), [0xff]).unwrap();
    let failed =
        Proxy::connect(Client::new(registry, project.id.clone(), directory).unwrap()).await;
    assert_eq!(failed.err().unwrap().code, "invalid_instructions");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    assert!(
        host.connections()
            .iter()
            .find(|status| status.project_id == project.id)
            .unwrap()
            .initialization_error
            .is_some(),
        "A healthy client's heartbeat must preserve another client's initialization failure"
    );
    client.cancel().await.unwrap();
    server.await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while host.connections().iter().any(|status| status.clients > 0) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn started_project_runs_allowed_input_without_per_action_approval() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    registry
        .update(
            &saved.id,
            Some("phone-A".into()),
            vec!["adm_press_key".into()],
            "ANDROMEDA.md".into(),
            true,
        )
        .unwrap();
    let fixture = support::DeviceFixture::start(0).await;
    let host = Host::start(
        registry.clone(),
        Controller::new(Adb::new(fixture.address), None, true),
    )
    .await
    .unwrap();
    host.set_project_state(
        &saved.id,
        andromeda::managed::ProjectState::Running,
        0,
        host.pause_generation(),
    )
    .unwrap();
    let client = Client::new(registry, saved.id.clone(), project).unwrap();
    let result = client
        .call(
            "adm_press_key".into(),
            json!({"key":"home", "timeout_ms":200}),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["success"],
        true,
        "Allowed input must execute without a human approving each request"
    );
    assert!(
        fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|command| command == "input keyevent 3")
    );
}

#[tokio::test]
async fn managed_mcp_enforces_tool_and_device_scope_on_the_server() {
    let workspace = tempfile::tempdir().unwrap();
    let project = workspace.path().join("project");
    std::fs::create_dir(&project).unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let saved = registry.register(&project).unwrap();
    registry
        .update(
            &saved.id,
            Some("phone-A".into()),
            saved.allowed_tools,
            "ANDROMEDA.md".into(),
            true,
        )
        .unwrap();
    let fixture = support::DeviceFixture::start(0).await;
    let _host = Host::start(
        registry.clone(),
        Controller::new(Adb::new(fixture.address), None, true),
    )
    .await
    .unwrap();
    let proxy = Proxy::connect(Client::new(registry, saved.id.clone(), project).unwrap())
        .await
        .unwrap();
    let (client_io, server_io) = tokio::io::duplex(65536);
    let task = tokio::spawn(async move {
        proxy
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("managed-test", "1"),
    )
    .serve(client_io)
    .await
    .unwrap();
    let tools = client.list_all_tools().await.unwrap();
    assert!(!tools.iter().any(|t| t.name == "adm_touch"));
    let denied = client
        .call_tool(CallToolRequestParams::new("adm_touch"))
        .await
        .unwrap();
    assert_eq!(
        denied.structured_content.unwrap()["error"]["code"],
        "tool_not_allowed"
    );
    let wrong_device = client
        .call_tool(
            CallToolRequestParams::new("adm_get_device_info")
                .with_arguments(json!({"serial":"phone-B"}).as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        wrong_device.structured_content.unwrap()["error"]["code"],
        "device_not_allowed"
    );
    assert!(
        fixture.commands.lock().unwrap().is_empty(),
        "Denied requests must not reach Android"
    );
    let listed = client
        .call_tool(CallToolRequestParams::new("adm_list_devices"))
        .await
        .unwrap();
    assert_eq!(
        listed.structured_content.unwrap()["data"]["devices"][0]["serial"],
        "phone-A"
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn project_pause_and_stop_are_isolated_and_stale_start_cannot_undo_stop() {
    use andromeda::managed::ProjectState;
    let workspace = tempfile::tempdir().unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let fixture = support::DeviceFixture::start(0).await;
    let host = Host::start(
        registry.clone(),
        Controller::new(Adb::new(fixture.address), None, true),
    )
    .await
    .unwrap();
    let mut projects = Vec::new();
    let mut clients = Vec::new();
    for name in ["A", "B"] {
        let root = workspace.path().join(name);
        std::fs::create_dir(&root).unwrap();
        let p = registry.register(&root).unwrap();
        registry
            .update(
                &p.id,
                Some("phone-A".into()),
                vec!["adm_press_key".into(), "adm_get_device_info".into()],
                "ANDROMEDA.md".into(),
                true,
            )
            .unwrap();
        clients.push(Client::new(registry.clone(), p.id.clone(), root).unwrap());
        host.set_project_state(&p.id, ProjectState::Running, 0, host.pause_generation())
            .unwrap();
        projects.push(p);
    }
    let a = host.project_session(&projects[0].id);
    host.set_project_state(
        &a.project_id,
        ProjectState::Paused,
        a.generation,
        host.pause_generation(),
    )
    .unwrap();
    let call = |client: Client, name: &str, args| {
        let name = name.to_owned();
        async move {
            let result = client
                .call(name, args, tokio_util::sync::CancellationToken::new())
                .await;
            let status = result.structured_content.unwrap();
            if status["success"] == true {
                let text = result.content.iter().find_map(|c| c.as_text()).unwrap();
                let text_status: serde_json::Value = serde_json::from_str(&text.text).unwrap();
                assert_eq!(text_status["data"], status["data"]);
            }
            status
        }
    };
    assert_eq!(
        call(clients[0].clone(), "adm_press_key", json!({"key":"home"})).await["error"]["code"],
        "project_paused"
    );
    assert_eq!(
        call(clients[1].clone(), "adm_press_key", json!({"key":"home"})).await["success"],
        true
    );
    let paused_info = call(clients[0].clone(), "adm_get_device_info", json!({})).await;
    assert_eq!(paused_info["success"], true);
    assert_eq!(paused_info["data"]["control_enabled"], false);
    assert_eq!(paused_info["data"]["project_state"], "paused");
    assert_eq!(
        call(clients[1].clone(), "adm_get_device_info", json!({})).await["data"]["control_enabled"],
        true
    );
    let earlier = host.project_session(&a.project_id);
    host.stop_project(&a.project_id);
    assert_eq!(
        host.set_project_state(
            &a.project_id,
            ProjectState::Running,
            earlier.generation,
            host.pause_generation()
        )
        .unwrap_err()
        .code,
        "session_changed"
    );
    assert_eq!(
        call(clients[0].clone(), "adm_press_key", json!({"key":"home"})).await["error"]["code"],
        "project_stopped"
    );
    assert_eq!(
        call(clients[0].clone(), "adm_get_device_info", json!({})).await["data"]["control_enabled"],
        false
    );
    let b = host.project_session(&projects[1].id);
    let epoch = host.pause_generation();
    host.stop_all();
    assert_eq!(
        host.set_project_state(&b.project_id, ProjectState::Running, b.generation, epoch)
            .unwrap_err()
            .code,
        "session_changed"
    );
    assert_eq!(
        call(clients[1].clone(), "adm_press_key", json!({"key":"home"})).await["error"]["code"],
        "project_stopped"
    );
}

#[tokio::test]
async fn pausing_an_active_project_cancels_its_batch_and_releases_other_projects() {
    use andromeda::managed::ProjectState;
    let workspace = tempfile::tempdir().unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let fixture = support::DeviceFixture::start(0).await;
    let host = Host::start(
        registry.clone(),
        Controller::new(Adb::new(fixture.address), None, true),
    )
    .await
    .unwrap();
    let mut projects = Vec::new();
    let mut clients = Vec::new();
    for name in ["A", "B"] {
        let root = workspace.path().join(name);
        std::fs::create_dir(&root).unwrap();
        let p = registry.register(&root).unwrap();
        registry
            .update(
                &p.id,
                Some("phone-A".into()),
                vec!["adm_batch_actions".into(), "adm_press_key".into()],
                "ANDROMEDA.md".into(),
                true,
            )
            .unwrap();
        clients.push(Client::new(registry.clone(), p.id.clone(), root).unwrap());
        host.set_project_state(&p.id, ProjectState::Running, 0, host.pause_generation())
            .unwrap();
        projects.push(p);
    }
    let batch = tokio::spawn({
        let client = clients[0].clone();
        async move {
            client.call("adm_batch_actions".into(),json!({"actions":[{"action":"press_key","key":"home"},{"action":"pause","duration_ms":10000},{"action":"press_key","key":"home"}]}),tokio_util::sync::CancellationToken::new()).await
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| c == "input keyevent 3")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let a = host.project_session(&projects[0].id);
    host.set_project_state(
        &a.project_id,
        ProjectState::Paused,
        a.generation,
        host.pause_generation(),
    )
    .unwrap();
    assert_eq!(
        batch.await.unwrap().structured_content.unwrap()["error"]["code"],
        "cancelled"
    );
    let b = clients[1]
        .call(
            "adm_press_key".into(),
            json!({"key":"home","timeout_ms":500}),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
    assert_eq!(b.structured_content.unwrap()["success"], true);
    assert_eq!(
        fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .filter(|c| *c == "input keyevent 3")
            .count(),
        2,
        "The cancelled batch must not dispatch its second key"
    );
}
