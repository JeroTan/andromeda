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
    let proxy = Proxy::connect(Client::new(registry, project.id.clone(), directory).unwrap())
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
    drop(proxy);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while host.connections().iter().any(|status| status.clients > 0) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_later_operator_stop_invalidates_an_earlier_resume_request() {
    let workspace = tempfile::tempdir().unwrap();
    let registry = Registry::open(workspace.path().join("state")).unwrap();
    let fixture = support::DeviceFixture::start(0).await;
    let host = Host::start(
        registry,
        Controller::new(Adb::new(fixture.address), None, true),
    )
    .await
    .unwrap();
    let earlier = host.pause_generation();
    host.stop_all();
    assert_eq!(host.resume(earlier).unwrap_err().code, "session_changed");
    assert!(host.is_paused());
    host.resume(host.pause_generation()).unwrap();
    assert!(!host.is_paused());
}

#[tokio::test]
async fn managed_input_requires_operator_approval_and_stop_cancels_pending_approval() {
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
    host.resume(host.pause_generation()).unwrap();
    let client = Client::new(registry, saved.id, project).unwrap();
    let proposal = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .call(
                    "adm_press_key".into(),
                    json!({"key":"home"}),
                    tokio_util::sync::CancellationToken::new(),
                )
                .await
        }
    });
    let pending = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if let Some(pending) = host.approvals().into_iter().next() {
                break pending;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(pending.tool, "adm_press_key");
    assert!(
        fixture.commands.lock().unwrap().is_empty(),
        "Input must not reach ADB before approval"
    );
    host.decide(pending.id, true).unwrap();
    assert_eq!(
        proposal.await.unwrap().structured_content.unwrap()["success"],
        true
    );
    assert!(
        fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|command| command == "input keyevent 3")
    );

    let proposal = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .call(
                    "adm_press_key".into(),
                    json!({"key":"home"}),
                    tokio_util::sync::CancellationToken::new(),
                )
                .await
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while host.approvals().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let stopped = client
        .call(
            "adm_stop".into(),
            json!({}),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
    assert_eq!(stopped.structured_content.unwrap()["success"], true);
    assert_eq!(
        proposal.await.unwrap().structured_content.unwrap()["error"]["code"],
        "cancelled"
    );
    assert!(host.approvals().is_empty());
    host.stop_all();
    let blocked = client
        .call(
            "adm_press_key".into(),
            json!({"key":"home"}),
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
    assert_eq!(
        blocked.structured_content.unwrap()["error"]["code"],
        "control_paused"
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
    let proxy = Proxy::connect(Client::new(registry, saved.id, project).unwrap())
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
