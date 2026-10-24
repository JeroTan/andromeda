mod companion;
mod long;
use crate::errors::{Error, Result};
pub(crate) use companion::from_companion;
use image::{GenericImageView, ImageFormat, ImageReader, Limits};
pub use long::{Scroll, Viewport, default_pages, default_settle_ms};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    #[default]
    Png,
    Jpeg,
    /// Lossy WebP at quality (default 80).
    Webp,
    /// Exact pixels; quality does not alter this encoder.
    WebpLossless,
    /// Smallest byte count among PNG, JPEG, lossy WebP at quality, and lossless WebP.
    Auto,
    /// Opt-in AVIF encoder. Only use with clients/exporters that accept image/avif.
    Avif,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// Preserve aspect ratio; never upscale. Omit for native resolution.
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    #[serde(default)]
    pub format: Format,
    /// JPEG/WebP/AVIF quality 1..=100, default 80. Lossless formats ignore it.
    pub quality: Option<u8>,
}

impl Options {
    pub fn validate(&self) -> Result<()> {
        if matches!(self.format, Format::Avif) && !cfg!(feature = "avif") {
            return Err(Error::new(
                "format_unavailable",
                "This build omits AVIF. Build with --features avif for AVIF-capable clients/exports; use webp or auto for ordinary vision clients.",
            ));
        }
        if [self.max_width, self.max_height]
            .iter()
            .flatten()
            .any(|n| !(1..=8192).contains(n))
            || self.quality.is_some_and(|q| q == 0 || q > 100)
        {
            return Err(Error::invalid(
                "Image dimensions must be 1..=8192 and quality 1..=100",
            ));
        }
        Ok(())
    }
}

pub struct Image {
    pub bytes: Vec<u8>,
    pub mime_type: &'static str,
    pub width: u32,
    pub height: u32,
    pub native_width: u32,
    pub native_height: u32,
}

pub fn decode(png: &[u8]) -> Result<image::DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(png))
        .with_guessed_format()
        .map_err(|_| Error::new("invalid_image", "Cannot identify screenshot image format"))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|e| {
        Error::new(
            "invalid_image",
            format!("Cannot decode Android screenshot: {e}"),
        )
    })
}

pub fn encode(png: Vec<u8>, options: Options) -> Result<Image> {
    options.validate()?;
    let decoded = decode(&png)?;
    let (native_width, native_height) = decoded.dimensions();
    let max_width = options.max_width.unwrap_or(native_width).min(native_width);
    let max_height = options
        .max_height
        .unwrap_or(native_height)
        .min(native_height);
    let resized = if native_width > max_width || native_height > max_height {
        decoded.resize(max_width, max_height, image::imageops::FilterType::Triangle)
    } else {
        decoded
    };
    let (width, height) = resized.dimensions();
    let original = (width == native_width && height == native_height).then_some(png);
    let (bytes, mime_type) = encode_pixels(&resized, options.format, options.quality, original)?;
    Ok(Image {
        bytes,
        mime_type,
        width,
        height,
        native_width,
        native_height,
    })
}

pub(crate) fn encode_pixels(
    pixels: &image::DynamicImage,
    format: Format,
    quality: Option<u8>,
    native_png: Option<Vec<u8>>,
) -> Result<(Vec<u8>, &'static str)> {
    if matches!(format, Format::Png)
        && let Some(png) = native_png
    {
        return Ok((png, "image/png"));
    }
    match format {
        Format::Png | Format::WebpLossless => {
            let mut out = Cursor::new(Vec::new());
            let (encoding, mime) = if matches!(format, Format::WebpLossless) {
                (ImageFormat::WebP, "image/webp")
            } else {
                (ImageFormat::Png, "image/png")
            };
            pixels
                .write_to(&mut out, encoding)
                .map_err(|e| Error::new("image_encoding", e.to_string()))?;
            Ok((out.into_inner(), mime))
        }
        Format::Webp => {
            let rgba = pixels.to_rgba8();
            let encoded = webp::Encoder::from_rgba(&rgba, pixels.width(), pixels.height())
                .encode_simple(false, f32::from(quality.unwrap_or(80)))
                .map_err(|error| {
                    Error::new("image_encoding", format!("WebP encoding failed: {error:?}"))
                })?;
            Ok((encoded.to_vec(), "image/webp"))
        }
        Format::Avif => {
            #[cfg(feature = "avif")]
            {
                use image::ImageEncoder;
                let rgba = pixels.to_rgba8();
                let mut out = Vec::new();
                image::codecs::avif::AvifEncoder::new_with_speed_quality(
                    &mut out,
                    10,
                    quality.unwrap_or(80),
                )
                .write_image(
                    &rgba,
                    pixels.width(),
                    pixels.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| Error::new("image_encoding", e.to_string()))?;
                Ok((out, "image/avif"))
            }
            #[cfg(not(feature = "avif"))]
            {
                Err(Error::new(
                    "format_unavailable",
                    "AVIF requires a build with --features avif",
                ))
            }
        }
        Format::Auto => {
            let png = match native_png {
                Some(png) => (png, "image/png"),
                None => encode_pixels(pixels, Format::Png, quality, None)?,
            };
            let jpeg = encode_pixels(pixels, Format::Jpeg, quality, None)?;
            let webp = encode_pixels(pixels, Format::Webp, quality, None)?;
            let lossless = encode_pixels(pixels, Format::WebpLossless, quality, None)?;
            [png, jpeg, webp, lossless]
                .into_iter()
                .min_by_key(|(bytes, _)| bytes.len())
                .ok_or_else(|| Error::new("image_encoding", "No compatible image encoder"))
        }
        Format::Jpeg => {
            let mut out = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.unwrap_or(80))
                .encode_image(&pixels.to_rgb8())
                .map_err(|e| Error::new("image_encoding", e.to_string()))?;
            Ok((out, "image/jpeg"))
        }
    }
}
