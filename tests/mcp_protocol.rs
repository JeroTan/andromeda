use andromeda::{actions::Controller, device::Adb, mcp::Server};
use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientConfig, Implementation},
};
mod support;

#[tokio::test]
async fn stop_remains_available_while_another_mcp_call_waits_on_android() {
    use rmcp::model::CallToolRequestParams;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (client_io, server_io) = tokio::io::duplex(65536);
    let controller = Controller::new(Adb::new(listener.local_addr().unwrap()), None, false);
    let task = tokio::spawn(async move {
        Server { controller }
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("andromeda-test", "1"),
    )
    .serve(client_io)
    .await
    .unwrap();
    let peer = client.peer().clone();
    let pending = tokio::spawn(async move {
        peer.call_tool(CallToolRequestParams::new("adm_list_devices"))
            .await
            .unwrap()
    });
    let (mut socket, _) = listener.accept().await.unwrap();
    support::request(&mut socket).await;
    let stopped = client
        .call_tool(CallToolRequestParams::new("adm_stop"))
        .await
        .unwrap();
    assert_eq!(
        stopped.structured_content.unwrap()["data"]["cancelled_operations"],
        1
    );
    let cancelled = tokio::time::timeout(std::time::Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        cancelled.structured_content.unwrap()["error"]["code"],
        "cancelled"
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn malformed_arguments_are_machine_readable_tool_errors() {
    use rmcp::model::CallToolRequestParams;
    let (client_io, server_io) = tokio::io::duplex(65536);
    let controller = Controller::new(Adb::new("127.0.0.1:1".parse().unwrap()), None, false);
    let task = tokio::spawn(async move {
        Server { controller }
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("andromeda-test", "1"),
    )
    .serve(client_io)
    .await
    .unwrap();
    let result = client
        .call_tool(
            CallToolRequestParams::new("adm_touch").with_arguments(
                serde_json::json!({"x":3,"y":4})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(true));
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "invalid_argument"
    );
    client.cancel().await.unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn mcp_negotiates_and_advertises_android_tools_with_input_schemas() {
    let (client_io, server_io) = tokio::io::duplex(65536);
    let controller = Controller::new(Adb::new("127.0.0.1:1".parse().unwrap()), None, false);
    let task = tokio::spawn(async move {
        Server { controller }
            .serve(server_io)
            .await
            .unwrap()
            .waiting()
            .await
            .unwrap();
    });
    let client = ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("andromeda-test", "1"),
    )
    .serve(client_io)
    .await
    .unwrap();
    let tools = client.list_all_tools().await.unwrap();
    assert!(tools.iter().any(|tool| tool.name == "adm_get_view"));
    assert!(tools.iter().all(|tool| tool.name.starts_with("adm_")
        && tool.input_schema.get("type") == Some(&serde_json::json!("object"))));
    client.cancel().await.unwrap();
    task.await.unwrap();
}
