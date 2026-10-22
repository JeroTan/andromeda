use crate::errors::{Error, Result};
use image::{GenericImageView, ImageFormat, ImageReader, Limits};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    #[default]
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// Preserve aspect ratio; never upscale. Omit for native resolution.
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    #[serde(default)]
    pub format: Format,
    /// JPEG quality 1..=100. Defaults to 80.
    pub quality: Option<u8>,
}

impl Options {
    pub fn validate(&self) -> Result<()> {
        if [self.max_width, self.max_height]
            .iter()
            .flatten()
            .any(|n| !(1..=8192).contains(n))
            || self.quality.is_some_and(|q| q == 0 || q > 100)
        {
            return Err(Error::invalid(
                "Image dimensions must be 1..=8192 and JPEG quality 1..=100",
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
    let mut reader = ImageReader::with_format(Cursor::new(png), ImageFormat::Png);
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
    let (bytes, mime_type) = match options.format {
        Format::Png if width == native_width && height == native_height => (png, "image/png"),
        Format::Png => {
            let mut out = Cursor::new(Vec::new());
            resized
                .write_to(&mut out, ImageFormat::Png)
                .map_err(|e| Error::new("image_encoding", e.to_string()))?;
            (out.into_inner(), "image/png")
        }
        Format::Jpeg => {
            let mut out = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut out,
                options.quality.unwrap_or(80),
            )
            .encode_image(&resized.to_rgb8())
            .map_err(|e| Error::new("image_encoding", e.to_string()))?;
            (out, "image/jpeg")
        }
    };
    Ok(Image {
        bytes,
        mime_type,
        width,
        height,
        native_width,
        native_height,
    })
}
