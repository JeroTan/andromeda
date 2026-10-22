use andromeda::{
    companion::{Companion, Store},
    device::Adb,
};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
mod support;

#[tokio::test]
async fn a_paired_multi_touch_request_sends_both_strokes_in_one_device_dispatch() {
    use andromeda::{actions::Controller, mcp::Server};
    let workspace = tempfile::tempdir().unwrap();
    let store = Store::open(workspace.path().join("state")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let mut methods = Vec::new();
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert_eq!(
                support::request(&mut socket).await,
                "host:transport:phone-A"
            );
            socket.write_all(b"OKAY").await.unwrap();
            let service = support::request(&mut socket).await;
            socket.write_all(b"OKAY").await.unwrap();
            if let Some(command) = service.strip_prefix("shell,v2,raw:") {
                assert_eq!(command, "wm size && dumpsys input");
                let output = b"Physical size: 1080x2400\nSurfaceOrientation: 0\n";
                socket.write_u8(1).await.unwrap();
                socket.write_u32_le(output.len() as u32).await.unwrap();
                socket.write_all(output).await.unwrap();
                socket.write_all(&[3, 1, 0, 0, 0, 0]).await.unwrap();
                methods.push("screen".to_owned());
            } else {
                assert_eq!(service, "localabstract:andromeda-v1");
                let length = socket.read_u32().await.unwrap() as usize;
                let mut body = vec![0; length];
                socket.read_exact(&mut body).await.unwrap();
                let value: Value = serde_json::from_slice(&body).unwrap();
                let method = value["method"].as_str().unwrap().to_owned();
                if method == "gesture" {
                    assert_eq!(value["strokes"].as_array().unwrap().len(), 2);
                    assert_eq!(value["strokes"][0]["duration_ms"], 450);
                    assert_eq!(value["strokes"][1]["start_ms"], 0);
                }
                let response = if method == "pair" {
                    json!({"success":true,"data":{"paired":true}})
                } else {
                    json!({"success":true,"data":{"completed":true,"strokes":2}})
                };
                let bytes = serde_json::to_vec(&response).unwrap();
                socket.write_u32(bytes.len() as u32).await.unwrap();
                socket.write_all(&bytes).await.unwrap();
                methods.push(method);
            }
        }
        methods
    });
    Companion::new(Adb::new(address), store.clone())
        .pair("phone-A")
        .await
        .unwrap();
    let server = Server {
        controller: Controller::new(Adb::new(address), Some("phone-A".into()), true)
            .with_companion(store),
    };
    let args = json!({"screen":{"width":1080,"height":2400,"rotation":0},"strokes":[{"points":[{"x":300,"y":700},{"x":380,"y":1050}],"start_ms":0,"duration_ms":450},{"points":[{"x":760,"y":700},{"x":680,"y":1050}],"start_ms":0,"duration_ms":450}]});
    let result = server
        .call(
            "adm_multi_touch",
            args,
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["data"]["companion_results"][0]["data"]["strokes"],
        2
    );
    assert_eq!(peer.await.unwrap(), vec!["pair", "screen", "gesture"]);
}

#[tokio::test]
async fn multi_touch_is_exposed_and_requires_pairing_before_device_input() {
    use andromeda::{
        actions::Controller,
        mcp::{Server, tools},
    };
    let fixture = support::DeviceFixture::start(0).await;
    let workspace = tempfile::tempdir().unwrap();
    let server = Server {
        controller: Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true)
            .with_companion(Store::open(workspace.path().join("state")).unwrap()),
    };
    assert!(tools().iter().any(|tool| tool.name == "adm_multi_touch"));
    let args = json!({"screen":{"width":1080,"height":2400,"rotation":0},"strokes":[{"points":[{"x":100,"y":100},{"x":200,"y":200}],"start_ms":0,"duration_ms":300},{"points":[{"x":500,"y":500},{"x":400,"y":400}],"start_ms":0,"duration_ms":300}]});
    let result = server
        .call(
            "adm_multi_touch",
            args,
            tokio_util::sync::CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "companion_not_paired"
    );
    assert!(
        !fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|command| command.starts_with("input"))
    );
}

#[tokio::test]
async fn companion_pairing_stays_private_and_permission_errors_reach_the_caller() {
    let workspace = tempfile::tempdir().unwrap();
    let store = Store::open(workspace.path().join("state")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut paired = String::new();
        for expected in ["pair", "status"] {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert_eq!(
                support::request(&mut socket).await,
                "host:transport:phone-A"
            );
            socket.write_all(b"OKAY").await.unwrap();
            assert_eq!(
                support::request(&mut socket).await,
                "localabstract:andromeda-v1"
            );
            socket.write_all(b"OKAY").await.unwrap();
            let length = socket.read_u32().await.unwrap();
            assert!(length < 1024 * 1024);
            let mut data = vec![0; length as usize];
            socket.read_exact(&mut data).await.unwrap();
            let request: Value = serde_json::from_slice(&data).unwrap();
            assert_eq!(request["version"], 1);
            assert_eq!(request["method"], expected);
            let token = request["token"].as_str().unwrap();
            assert_eq!(token.len(), 64);
            let response = if expected == "pair" {
                paired = token.into();
                json!({"success":true,"data":{"paired":true}})
            } else {
                assert_eq!(token, paired);
                json!({"success":false,"error":{"code":"session_inactive","message":"Start a session on the phone"}})
            };
            let bytes = serde_json::to_vec(&response).unwrap();
            socket.write_u32(bytes.len() as u32).await.unwrap();
            socket.write_all(&bytes).await.unwrap();
        }
    });
    let companion = Companion::new(Adb::new(address), store.clone());
    assert!(!store.is_paired("phone-A").unwrap());
    companion.pair("phone-A").await.unwrap();
    assert!(store.is_paired("phone-A").unwrap());
    assert_eq!(
        companion.status("phone-A").await.unwrap_err().code,
        "session_inactive"
    );
    store.forget("phone-A").unwrap();
    assert!(!store.is_paired("phone-A").unwrap());
    task.await.unwrap();
}
