use andromeda::{actions::Controller, device::Adb, mcp::Server};
use base64::Engine;
use serde_json::json;
use tokio_util::sync::CancellationToken;
mod support;

async fn server() -> (support::DeviceFixture, Server) {
    let pixels = image::RgbaImage::from_pixel(1080, 2400, image::Rgba([23, 45, 67, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    pixels.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let fixture = support::DeviceFixture::with_capture(png.into_inner()).await;
    let server = Server {
        controller: Controller::new(Adb::new(fixture.address), Some("phone-A".into()), false),
    };
    (fixture, server)
}

#[tokio::test]
async fn webp_capture_is_a_decodable_lossless_mcp_image_with_native_coordinates() {
    let (_fixture, server) = server().await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"webp_lossless"}}),
            CancellationToken::new(),
        )
        .await;
    let content = result.structured_content.unwrap();
    assert_eq!(content["success"], true, "{content}");
    let image = result
        .content
        .iter()
        .find_map(|block| block.as_image())
        .unwrap();
    assert_eq!(image.mime_type, "image/webp");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&image.data)
        .unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (1080, 2400));
    assert_eq!(decoded.get_pixel(500, 1000).0, [23, 45, 67, 255]);
    assert_eq!(content["data"]["scale_to_screen"]["x"], 1.0);
}

#[tokio::test]
async fn automatic_capture_chooses_the_smallest_compatible_encoding() {
    let (_fixture, server) = server().await;
    let mut candidates = Vec::new();
    for format in ["png", "jpeg", "webp", "webp_lossless"] {
        let result = server
            .call(
                "adm_get_view",
                json!({"image":{"format":format,"max_width":360,"quality":80}}),
                CancellationToken::new(),
            )
            .await;
        let image = result
            .content
            .iter()
            .find_map(|block| block.as_image())
            .unwrap();
        candidates.push((image.mime_type.clone(), image.data.clone()));
    }
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"auto","max_width":360,"quality":80}}),
            CancellationToken::new(),
        )
        .await;
    let content = result.structured_content.unwrap();
    assert_eq!(content["success"], true, "{content}");
    let image = result
        .content
        .iter()
        .find_map(|block| block.as_image())
        .unwrap();
    let smallest = candidates
        .into_iter()
        .min_by_key(|(_, bytes)| {
            base64::engine::general_purpose::STANDARD
                .decode(bytes)
                .unwrap()
                .len()
        })
        .unwrap();
    assert_eq!(image.mime_type, smallest.0);
    assert_eq!(image.data, smallest.1);
}

#[cfg(not(feature = "avif"))]
#[tokio::test]
async fn avif_without_the_optional_encoder_fails_before_device_capture() {
    let (_fixture, server) = server().await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"avif"}}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(
        result.structured_content.unwrap()["error"]["code"],
        "format_unavailable"
    );
    assert!(_fixture.commands.lock().unwrap().is_empty());
}

#[cfg(feature = "avif")]
#[tokio::test]
async fn optional_avif_export_has_the_correct_mime_and_avif_container() {
    let (_fixture, server) = server().await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"avif","max_width":32}}),
            CancellationToken::new(),
        )
        .await;
    let content = result.structured_content.unwrap();
    assert_eq!(content["success"], true, "{content}");
    let image = result
        .content
        .iter()
        .find_map(|block| block.as_image())
        .unwrap();
    assert_eq!(image.mime_type, "image/avif");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&image.data)
        .unwrap();
    assert_eq!(&bytes[4..8], b"ftyp");
    assert!(bytes[8..32].windows(4).any(|bytes| bytes == b"avif"));
}

#[tokio::test]
async fn lossy_webp_honors_the_requested_dimensions_and_returns_valid_pixels() {
    let (_fixture, server) = server().await;
    let result = server
        .call(
            "adm_get_view",
            json!({"image":{"format":"webp","max_width":360,"quality":80}}),
            CancellationToken::new(),
        )
        .await;
    let content = result.structured_content.unwrap();
    assert_eq!(content["success"], true, "{content}");
    let image = result
        .content
        .iter()
        .find_map(|block| block.as_image())
        .unwrap();
    assert_eq!(image.mime_type, "image/webp");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&image.data)
        .unwrap();
    let pixels = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(pixels.dimensions(), (360, 800));
    for (actual, expected) in pixels.get_pixel(180, 400).0.iter().zip([23u8, 45, 67, 255]) {
        assert!(actual.abs_diff(expected) <= 8);
    }
    assert_eq!(content["data"]["scale_to_screen"]["x"], 3.0);
}
