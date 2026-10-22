use crate::errors::{Error, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Coordinates are pixels in the current display orientation, origin top-left.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Screen {
    pub width: u32,
    pub height: u32,
    /// Clockwise quarter turns from the device's natural orientation (0..=3).
    pub rotation: u8,
}

impl Screen {
    pub fn validate_point(&self, x: u32, y: u32) -> Result<()> {
        if self.width == 0
            || self.height == 0
            || self.rotation > 3
            || x >= self.width
            || y >= self.height
        {
            return Err(Error::invalid("Point is outside the observed screen"));
        }
        Ok(())
    }

    pub(super) fn parse(output: &str) -> Result<Self> {
        let size = output
            .lines()
            .filter_map(|line| {
                line.strip_prefix("Override size: ")
                    .or_else(|| line.strip_prefix("Physical size: "))
            })
            .next_back()
            .and_then(|size| size.trim().split_once('x'))
            .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
            .ok_or_else(|| {
                Error::new(
                    "display_unavailable",
                    "Android did not report a valid display size",
                )
            })?;
        let rotation = output
            .lines()
            .find_map(|line| {
                if line.contains("Viewport") && line.contains("displayId=0") {
                    line.split("orientation=")
                        .nth(1)?
                        .split(|c: char| !c.is_ascii_digit())
                        .next()?
                        .parse::<u8>()
                        .ok()
                } else {
                    line.trim()
                        .strip_prefix("SurfaceOrientation: ")?
                        .trim()
                        .parse::<u8>()
                        .ok()
                }
            })
            .filter(|r| *r <= 3)
            .ok_or_else(|| {
                Error::new(
                    "display_unavailable",
                    "Android did not report display rotation; refusing to guess coordinates",
                )
            })?;
        let (width, height) = if rotation % 2 == 1 {
            (size.1, size.0)
        } else {
            size
        };
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(Error::new(
                "display_unavailable",
                "Display dimensions outside supported bounds",
            ));
        }
        Ok(Self {
            width,
            height,
            rotation,
        })
    }
}
