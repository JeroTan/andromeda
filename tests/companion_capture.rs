use andromeda::{
    actions::Controller,
    companion::{Companion, Store},
    device::Adb,
    mcp::Server,
};
use base64::Engine;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
mod support;

fn capture_response() -> Value {
    let pixels = image::RgbaImage::from_pixel(36, 80, image::Rgba([23, 45, 67, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    pixels.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let bytes = png.into_inner();
    json!({"success":true,"data":{"mime_type":"image/png","format":"png","width":36,"height":80,"native_width":108,"native_height":240,"bytes":bytes.len(),"image_base64":base64::engine::general_purpose::STANDARD.encode(bytes),"screen":{"width":108,"height":240,"rotation":0},"capture_ms":8,"processing_ms":3,"api_timestamp_age_ms":3}})
}
async fn fixture(
    response: Value,
) -> (
    tempfile::TempDir,
    tokio::task::JoinHandle<()>,
    Arc<Mutex<Vec<String>>>,
    Server,
) {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path().join("state")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let observed = requests.clone();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            assert_eq!(
                support::request(&mut socket).await,
                "host:transport:phone-A"
            );
            socket.write_all(b"OKAY").await.unwrap();
            let service = support::request(&mut socket).await;
            observed.lock().unwrap().push(service.clone());
            socket.write_all(b"OKAY").await.unwrap();
            if let Some(command) = service.strip_prefix("shell,v2,raw:") {
                assert_eq!(
                    response["error"]["code"], "unknown_method",
                    "Only unavailable old APIs may use the ADB fallback"
                );
                let output = if command == "screencap -p" {
                    let pixels =
                        image::RgbaImage::from_pixel(108, 240, image::Rgba([23, 45, 67, 255]));
                    let mut png = std::io::Cursor::new(Vec::new());
                    pixels.write_to(&mut png, image::ImageFormat::Png).unwrap();
                    png.into_inner()
                } else {
                    assert_eq!(command, "wm size && dumpsys input");
                    b"Physical size: 108x240\nSurfaceOrientation: 0\n".to_vec()
                };
                socket.write_u8(1).await.unwrap();
                socket.write_u32_le(output.len() as u32).await.unwrap();
                socket.write_all(&output).await.unwrap();
                socket.write_all(&[3, 1, 0, 0, 0, 0]).await.unwrap();
                continue;
            }
            assert_eq!(
                service, "localabstract:andromeda-v1",
                "A companion frame contains checked geometry; do not add ADB geometry round trips"
            );
            let size = socket.read_u32().await.unwrap() as usize;
            let mut bytes = vec![0; size];
            socket.read_exact(&mut bytes).await.unwrap();
            let request: Value = serde_json::from_slice(&bytes).unwrap();
            let reply = if request["method"] == "pair" {
                json!({"success":true,"data":{"paired":true}})
            } else {
                assert_eq!(request["method"], "screenshot");
                assert_eq!(request["max_width"], 36);
                assert_eq!(
                    request["format"],
                    response["data"]["format"].as_str().unwrap_or("png")
                );
                response.clone()
            };
            let bytes = serde_json::to_vec(&reply).unwrap();
            socket.write_u32(bytes.len() as u32).await.unwrap();
            socket.write_all(&bytes).await.unwrap();
        }
    });
    Companion::new(Adb::new(address), store.clone())
        .pair("phone-A")
        .await
        .unwrap();
    let server = Server {
        controller: Controller::new(Adb::new(address), Some("phone-A".into()), false)
            .with_companion(store),
    };
    (root, task, requests, server)
}
#[tokio::test]
async fn paired_capture_returns_exact_pixels_and_native_coordinate_scale_without_shell_hops() {
    let (_root, peer, requests, server) = fixture(capture_response()).await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"png","max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    let data = result.structured_content.unwrap();
    assert_eq!(data["success"], true, "{data}");
    assert_eq!(data["data"]["capture_backend"], "companion");
    assert_eq!(data["data"]["screen"]["width"], 108);
    assert_eq!(data["data"]["scale_to_screen"]["x"], 3.0);
    assert_eq!(data["data"]["scale_to_screen"]["y"], 3.0);
    let image = result.content.iter().find_map(|c| c.as_image()).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&image.data)
        .unwrap();
    let pixels = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(pixels.dimensions(), (36, 80));
    assert_eq!(pixels.get_pixel(12, 30).0, [23, 45, 67, 255]);
    assert_eq!(requests.lock().unwrap().len(), 2);
    peer.abort();
}

#[tokio::test]
async fn companion_capture_rejects_inconsistent_pixels_without_falling_back() {
    let mut response = capture_response();
    response["data"]["width"] = json!(35);
    let (_root, peer, requests, server) = fixture(response).await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"png","max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "companion_protocol"
    );
    assert!(result.content.iter().all(|c| c.as_image().is_none()));
    assert_eq!(requests.lock().unwrap().len(), 2);
    peer.abort();
}
#[tokio::test]
async fn already_encoded_jpeg_is_validated_and_returned_without_quality_loss_from_reencoding() {
    let mut response = capture_response();
    let pixels = image::RgbImage::from_pixel(36, 80, image::Rgb([23, 45, 67]));
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)
        .encode_image(&pixels)
        .unwrap();
    response["data"]["format"] = json!("jpeg");
    response["data"]["mime_type"] = json!("image/jpeg");
    response["data"]["bytes"] = json!(jpeg.len());
    response["data"]["image_base64"] =
        json!(base64::engine::general_purpose::STANDARD.encode(&jpeg));
    let (_root, peer, _requests, server) = fixture(response).await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"jpeg","quality":95,"max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result.structured_content.unwrap()["success"], true);
    let image = result.content.iter().find_map(|c| c.as_image()).unwrap();
    assert_eq!(image.mime_type, "image/jpeg");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(&image.data)
            .unwrap(),
        jpeg
    );
    peer.abort();
}
#[tokio::test]
async fn protected_screen_failure_does_not_attempt_another_capture_backend() {
    let (_root,peer,requests,server)=fixture(json!({"success":false,"error":{"code":"capture_protected","message":"Android protected this window"}})).await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "capture_protected"
    );
    assert_eq!(requests.lock().unwrap().len(), 2);
    peer.abort();
}

#[tokio::test]
async fn an_older_phone_app_falls_back_once_to_the_original_adb_capture() {
    let (_root, peer, requests, server) = fixture(
        json!({"success":false,"error":{"code":"unknown_method","message":"Old companion API"}}),
    )
    .await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    let data = result.structured_content.unwrap();
    assert_eq!(data["success"], true, "{data}");
    assert_eq!(data["data"]["capture_backend"], "adb");
    assert_eq!(data["data"]["capture_fallback_code"], "unknown_method");
    assert_eq!(requests.lock().unwrap().len(), 5);
    peer.abort();
}
#[tokio::test]
async fn explicit_companion_capture_reports_an_old_api_without_silently_using_adb() {
    let (_root, peer, requests, server) = fixture(
        json!({"success":false,"error":{"code":"unknown_method","message":"Old companion API"}}),
    )
    .await;
    let server = Server {
        controller: server
            .controller
            .with_capture_backend(andromeda::actions::CaptureBackend::Companion),
    };
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "unknown_method"
    );
    assert_eq!(requests.lock().unwrap().len(), 2);
    peer.abort();
}

#[tokio::test]
async fn a_lossy_source_cannot_be_relabelled_as_a_lossless_png_capture() {
    let mut response = capture_response();
    let pixels = image::RgbImage::from_pixel(36, 80, image::Rgb([23, 45, 67]));
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80)
        .encode_image(&pixels)
        .unwrap();
    response["data"]["mime_type"] = json!("image/jpeg");
    response["data"]["bytes"] = json!(jpeg.len());
    response["data"]["image_base64"] =
        json!(base64::engine::general_purpose::STANDARD.encode(jpeg));
    let (_root, peer, _requests, server) = fixture(response).await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"png","max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "companion_protocol"
    );
    assert!(result.content.iter().all(|c| c.as_image().is_none()));
    peer.abort();
}

#[tokio::test]
async fn a_lossy_webp_payload_cannot_claim_the_lossless_mode() {
    let mut response = capture_response();
    let pixels = image::RgbaImage::from_pixel(36, 80, image::Rgba([23, 45, 67, 255]));
    let webp = webp::Encoder::from_rgba(pixels.as_raw(), 36, 80)
        .encode(80.0)
        .to_vec();
    response["data"]["format"] = json!("webp_lossless");
    response["data"]["mime_type"] = json!("image/webp");
    response["data"]["bytes"] = json!(webp.len());
    response["data"]["image_base64"] =
        json!(base64::engine::general_purpose::STANDARD.encode(webp));
    let (_root, peer, _requests, server) = fixture(response).await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"webp_lossless","max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "companion_protocol"
    );
    peer.abort();
}

#[tokio::test]
async fn lossless_webp_preserves_exact_pixels_and_encoded_bytes() {
    let mut response = capture_response();
    let pixels = image::RgbaImage::from_fn(36, 80, |x, y| image::Rgba([x as u8, y as u8, 67, 255]));
    let webp = webp::Encoder::from_rgba(pixels.as_raw(), 36, 80)
        .encode_lossless()
        .to_vec();
    response["data"]["format"] = json!("webp_lossless");
    response["data"]["mime_type"] = json!("image/webp");
    response["data"]["bytes"] = json!(webp.len());
    response["data"]["image_base64"] =
        json!(base64::engine::general_purpose::STANDARD.encode(&webp));
    let (_root, peer, _requests, server) = fixture(response).await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"webp_lossless","max_width":36},"timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result.structured_content.unwrap()["success"], true);
    let image = result.content.iter().find_map(|c| c.as_image()).unwrap();
    let returned = base64::engine::general_purpose::STANDARD
        .decode(&image.data)
        .unwrap();
    assert_eq!(returned, webp);
    assert_eq!(
        image::load_from_memory(&returned).unwrap().to_rgba8(),
        pixels
    );
    peer.abort();
}
