use andromeda::{actions::Controller, device::Adb, mcp::Server};
use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientConfig, Implementation},
};
mod support;

#[tokio::test]
async fn mcp_cancellation_notification_closes_the_inflight_adb_stream() {
    use rmcp::{
        model::{CallToolRequestParams, ClientRequest, Request},
        service::PeerRequestOptions,
    };
    use tokio::io::AsyncReadExt;
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
    let pending = client
        .send_cancellable_request(
            ClientRequest::CallToolRequest(Request::new(CallToolRequestParams::new(
                "adm_list_devices",
            ))),
            PeerRequestOptions::no_options(),
        )
        .await
        .unwrap();
    let (mut socket, _) = listener.accept().await.unwrap();
    support::request(&mut socket).await;
    pending
        .cancel(Some("acceptance test cancellation".into()))
        .await
        .unwrap();
    let mut byte = [0; 1];
    let read = tokio::time::timeout(std::time::Duration::from_secs(3), socket.read(&mut byte))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(read, 0, "request cancellation must drop the ADB connection");
    let stopped = client
        .call_tool(CallToolRequestParams::new("adm_stop"))
        .await
        .unwrap();
    assert_eq!(
        stopped.structured_content.unwrap()["data"]["cancelled_operations"],
        0
    );
    assert!(!client.list_all_tools().await.unwrap().is_empty());
    client.cancel().await.unwrap();
    task.await.unwrap();
}

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

#[tokio::test]
async fn advertised_phone_control_examples_are_executable_through_the_public_mcp_interface() {
    use tokio_util::sync::CancellationToken;
    for name in [
        "adm_get_view",
        "adm_touch",
        "adm_swipe",
        "adm_wait_for",
        "adm_batch_actions",
    ] {
        let fixture = if name == "adm_get_view" {
            let mut bytes = Vec::new();
            image::DynamicImage::new_rgb8(64, 64)
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .unwrap();
            support::DeviceFixture::with_capture(bytes).await
        } else {
            support::DeviceFixture::start(0).await
        };
        let server = Server {
            controller: Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true),
        };
        let tool = andromeda::mcp::tools()
            .into_iter()
            .find(|t| t.name == name)
            .unwrap();
        let arguments = tool.input_schema["examples"][0].clone();
        assert!(
            arguments.is_object(),
            "{name} must advertise an argument object"
        );
        let result = server.call(name, arguments, CancellationToken::new()).await;
        assert_eq!(
            result.structured_content.as_ref().unwrap()["success"],
            true,
            "Advertised example for {name} failed: {:?}",
            result.structured_content
        );
    }
}
