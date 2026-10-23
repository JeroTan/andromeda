use andromeda::{actions::Controller, device::Adb, mcp::Server};
use serde_json::json;
use tokio_util::sync::CancellationToken;
mod support;

#[tokio::test]
async fn scrolling_capture_requires_input_authorization_before_contacting_the_device() {
    let fixture = support::DeviceFixture::start(0).await;
    let server = Server {
        controller: Controller::new(Adb::new(fixture.address), Some("phone-A".into()), false),
    };
    let result = server
        .call(
            "adm_get_long_view",
            json!({
                "screen":{"width":1080,"height":2400,"rotation":0},
                "viewport":{"x":0,"y":200,"width":1080,"height":1800},
                "swipe":{"from":{"x":500,"y":1700},"to":{"x":500,"y":500},"duration_ms":200},
                "max_pages":3,"settle_ms":0
            }),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "control_disabled"
    );
    assert!(fixture.commands.lock().unwrap().is_empty());
}

struct ScrollingDevice {
    address: std::net::SocketAddr,
    commands: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for ScrollingDevice {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl ScrollingDevice {
    async fn start() -> Self {
        use tokio::{io::AsyncWriteExt, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let commands = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed = commands.clone();
        let task = tokio::spawn(async move {
            let mut captures = 0;
            while let Ok((mut socket, _)) = listener.accept().await {
                assert_eq!(
                    support::request(&mut socket).await,
                    "host:transport:phone-A"
                );
                socket.write_all(b"OKAY").await.unwrap();
                let request = support::request(&mut socket).await;
                let command = request.strip_prefix("shell,v2,raw:").unwrap();
                observed.lock().unwrap().push(command.to_owned());
                socket.write_all(b"OKAY").await.unwrap();
                let data = if command == "wm size && dumpsys input" {
                    b"Physical size: 4x8\nSurfaceOrientation: 0\n".to_vec()
                } else if command == "screencap -p" {
                    let color = if captures == 0 {
                        [255, 0, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    };
                    captures += 1;
                    let image = image::RgbaImage::from_pixel(4, 8, image::Rgba(color));
                    let mut out = std::io::Cursor::new(Vec::new());
                    image.write_to(&mut out, image::ImageFormat::Png).unwrap();
                    out.into_inner()
                } else if command == "input swipe 2 6 2 2 5" || command == "input tap 2 2" {
                    Vec::new()
                } else {
                    panic!("Unexpected Android command: {command}");
                };
                socket.write_u8(1).await.unwrap();
                socket.write_u32_le(data.len() as u32).await.unwrap();
                socket.write_all(&data).await.unwrap();
                socket.write_all(&[3, 1, 0, 0, 0, 0]).await.unwrap();
            }
        });
        Self {
            address,
            commands,
            task,
        }
    }
}

fn scroll_request() -> serde_json::Value {
    json!({
        "screen":{"width":4,"height":8,"rotation":0},
        "viewport":{"x":0,"y":1,"width":4,"height":6},
        "swipe":{"from":{"x":2,"y":6},"to":{"x":2,"y":2},"duration_ms":5},
        "max_pages":4,"settle_ms":0
    })
}

#[tokio::test]
async fn scrolling_capture_returns_visible_pages_and_stops_when_the_viewport_repeats() {
    use base64::Engine;
    let fixture = ScrollingDevice::start().await;
    let server = Server {
        controller: Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true),
    };
    let result = server
        .call(
            "adm_get_long_view",
            scroll_request(),
            CancellationToken::new(),
        )
        .await;
    let content = result.structured_content.unwrap();
    assert_eq!(content["success"], true, "{content}");
    let data = &content["data"];
    assert_eq!(data["stop_reason"], "viewport_unchanged");
    assert_eq!(data["completed_swipes"], 2);
    assert_eq!(data["pages"].as_array().unwrap().len(), 2);
    assert_eq!(data["pages"][1]["image_y"], 6);
    let image = result
        .content
        .iter()
        .find_map(|block| block.as_image())
        .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&image.data)
        .unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (4, 12));
    assert_eq!(decoded.get_pixel(1, 1).0, [255, 0, 0, 255]);
    assert_eq!(decoded.get_pixel(1, 7).0, [0, 0, 255, 255]);
    assert_eq!(
        fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.starts_with("input"))
            .count(),
        2
    );
}

#[tokio::test]
async fn scrolling_capture_rejects_swipes_outside_the_list_before_any_device_work() {
    let fixture = ScrollingDevice::start().await;
    let server = Server {
        controller: Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true),
    };
    let mut request = scroll_request();
    request["swipe"]["to"]["y"] = json!(0);
    let result = server
        .call("adm_get_long_view", request, CancellationToken::new())
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "invalid_argument"
    );
    assert!(fixture.commands.lock().unwrap().is_empty());
}

#[tokio::test]
async fn stop_interrupts_a_local_scroll_loop_and_releases_the_input_gate() {
    let fixture = ScrollingDevice::start().await;
    let server = Server {
        controller: Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true),
    };
    let mut request = scroll_request();
    request["settle_ms"] = json!(3000);
    let worker = server.clone();
    let pending = tokio::spawn(async move {
        worker
            .call("adm_get_long_view", request, CancellationToken::new())
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if fixture
                .commands
                .lock()
                .unwrap()
                .iter()
                .any(|s| s.starts_with("input swipe"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let stopped = server
        .call("adm_stop", json!({}), CancellationToken::new())
        .await;
    assert_eq!(
        stopped.structured_content.unwrap()["data"]["cancelled_operations"],
        1
    );
    let result = pending.await.unwrap();
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "cancelled"
    );
    let next = server
        .call(
            "adm_touch",
            json!({"point":{"x":2,"y":2},"screen":{"width":4,"height":8,"rotation":0}}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(next.structured_content.unwrap()["success"], true);
    assert_eq!(
        fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .filter(|s| *s == "screencap -p")
            .count(),
        1
    );
}
