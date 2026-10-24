use super::{Format, Image, Options, decode, encode_pixels};
use crate::{
    device::Screen,
    errors::{Error, Result},
};
use base64::Engine;
use image::GenericImageView;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct DeviceTimings {
    pub capture_ms: f64,
    pub processing_ms: f64,
    pub api_timestamp_age_ms: f64,
}
#[derive(Deserialize)]
struct Frame {
    mime_type: String,
    format: String,
    width: u32,
    height: u32,
    native_width: u32,
    native_height: u32,
    bytes: usize,
    image_base64: String,
    screen: Screen,
    #[serde(flatten)]
    timings: DeviceTimings,
}
pub(crate) fn from_companion(
    value: serde_json::Value,
    options: Options,
) -> Result<(Image, Screen, DeviceTimings)> {
    let frame: Frame = serde_json::from_value(value)
        .map_err(|_| Error::new("companion_protocol", "Invalid screenshot response metadata"))?;
    let invalid = || {
        Error::new(
            "companion_protocol",
            "Screenshot pixels, encoding or geometry disagree with metadata",
        )
    };
    frame.screen.validate_point(0, 0).map_err(|_| invalid())?;
    if frame.native_width != frame.screen.width
        || frame.native_height != frame.screen.height
        || frame.native_width > 8192
        || frame.native_height > 8192
        || u64::from(frame.native_width) * u64::from(frame.native_height) > 8_388_608
        || frame.bytes == 0
        || frame.bytes > 3 * 1024 * 1024
        || frame.image_base64.len() > 4 * 1024 * 1024
    {
        return Err(invalid());
    }
    let ratio = 1.0f64
        .min(options.max_width.unwrap_or(8192) as f64 / frame.native_width as f64)
        .min(options.max_height.unwrap_or(8192) as f64 / frame.native_height as f64);
    if frame.width != (frame.native_width as f64 * ratio).floor().max(1.0) as u32
        || frame.height != (frame.native_height as f64 * ratio).floor().max(1.0) as u32
    {
        return Err(invalid());
    }
    let requested_format = match options.format {
        Format::Jpeg => "jpeg",
        Format::Webp => "webp",
        Format::WebpLossless => "webp_lossless",
        _ => "png",
    };
    let expected_mime = match requested_format {
        "jpeg" => "image/jpeg",
        "webp" | "webp_lossless" => "image/webp",
        _ => "image/png",
    };
    if frame.format != requested_format || frame.mime_type != expected_mime {
        return Err(invalid());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(frame.image_base64)
        .map_err(|_| invalid())?;
    if bytes.len() != frame.bytes {
        return Err(invalid());
    }
    let (format, mime) = match frame.mime_type.as_str() {
        "image/png" => (image::ImageFormat::Png, "image/png"),
        "image/jpeg" => (image::ImageFormat::Jpeg, "image/jpeg"),
        "image/webp" => (image::ImageFormat::WebP, "image/webp"),
        _ => return Err(invalid()),
    };
    if image::guess_format(&bytes).ok() != Some(format) {
        return Err(invalid());
    }
    let pixels = decode(&bytes)?;
    if pixels.dimensions() != (frame.width, frame.height) {
        return Err(invalid());
    }
    let direct = matches!(
        (options.format, format),
        (Format::Png, image::ImageFormat::Png)
            | (Format::Jpeg, image::ImageFormat::Jpeg)
            | (
                Format::Webp | Format::WebpLossless,
                image::ImageFormat::WebP
            )
    );
    let (bytes, mime_type) = if direct {
        (bytes, mime)
    } else {
        encode_pixels(
            &pixels,
            options.format,
            options.quality,
            (format == image::ImageFormat::Png).then_some(bytes),
        )?
    };
    if [
        frame.timings.capture_ms,
        frame.timings.processing_ms,
        frame.timings.api_timestamp_age_ms,
    ]
    .iter()
    .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(invalid());
    }
    Ok((
        Image {
            bytes,
            mime_type,
            width: frame.width,
            height: frame.height,
            native_width: frame.native_width,
            native_height: frame.native_height,
        },
        frame.screen,
        frame.timings,
    ))
}
