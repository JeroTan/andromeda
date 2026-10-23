use super::{Controller, Output};
use crate::{
    capture,
    errors::{Error, Result},
    input::Action,
};
use image::GenericImageView;
use serde_json::json;
use std::time::Duration;

impl Controller {
    pub(super) async fn long_view(&self, serial: &str, options: capture::Scroll) -> Result<Output> {
        options.validate()?;
        let _input = self.inner.action_gate.try_lock().map_err(|_| {
            Error::new(
                "device_busy",
                "Another input sequence is running; wait or call adm_stop",
            )
        })?;
        let mut permit = self
            .inner
            .image_gate
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Error::new("shutdown", "Image processor closed"))?;
        let viewport = options.viewport;
        let tile_width = options.image.max_width.unwrap_or(720).min(viewport.width);
        let tile_height = options.image.max_height.unwrap_or(8192) / u32::from(options.max_pages);
        let mut pages: Vec<image::RgbaImage> = Vec::new();
        let mut stop_reason = "page_limit";
        let mut swipes = 0;
        for index in 0..options.max_pages {
            if self.is_paused() {
                return Err(Error::new(
                    "control_paused",
                    "Device input was paused by the operator",
                ));
            }
            self.inner.adb.ensure_screen(serial, options.screen).await?;
            if index > 0 {
                let action = Action::Swipe {
                    swipe: options.swipe.clone(),
                    screen: options.screen,
                };
                let command = action
                    .shell_command()?
                    .ok_or_else(|| Error::invalid("Missing scroll command"))?;
                self.inner
                    .adb
                    .shell(serial, &command)
                    .await
                    .map_err(|mut error| {
                        error.retryable = false;
                        error
                    })?;
                swipes += 1;
                tokio::time::sleep(Duration::from_millis(options.settle_ms)).await;
            }
            let png = self.inner.adb.shell(serial, "screencap -p").await?;
            self.inner.adb.ensure_screen(serial, options.screen).await?;
            let screen = options.screen;
            let (returned_permit, tile) = tokio::task::spawn_blocking(move || {
                let result = (|| {
                    let frame = capture::decode(&png)?;
                    if frame.dimensions() != (screen.width, screen.height) {
                        return Err(Error::new(
                            "stale_screen",
                            "Screenshot dimensions disagree with the supplied screen",
                        ));
                    }
                    Ok(frame
                        .crop_imm(viewport.x, viewport.y, viewport.width, viewport.height)
                        .resize(
                            tile_width,
                            tile_height,
                            image::imageops::FilterType::Triangle,
                        )
                        .to_rgba8())
                })();
                (permit, result)
            })
            .await
            .map_err(|e| Error::new("image_encoding", e.to_string()))?;
            permit = returned_permit;
            let tile = tile?;
            if pages.iter().any(|previous| previous == &tile) {
                stop_reason = "viewport_unchanged";
                break;
            }
            pages.push(tile);
        }
        let width = pages[0].width();
        let height: u32 = pages.iter().map(image::GenericImageView::height).sum();
        let mut offset = 0;
        let metadata: Vec<_> = pages.iter().enumerate().map(|(index, page)| {
            let data = json!({"index":index,"image_y":offset,"image_height":page.height(),"image_width":page.width(),"scale_to_screen":{"x":viewport.width as f64/page.width() as f64,"y":viewport.height as f64/page.height() as f64}});
            offset += page.height(); data
        }).collect();
        let image_options = options.image;
        let image = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let mut canvas = image::RgbaImage::new(width, height);
            let mut y = 0;
            for page in pages {
                image::imageops::replace(&mut canvas, &page, 0, i64::from(y));
                y += page.height();
            }
            let canvas = image::DynamicImage::ImageRgba8(canvas);
            let (bytes, mime_type) =
                capture::encode_pixels(&canvas, image_options.format, image_options.quality, None)?;
            Ok::<_, Error>(capture::Image {
                bytes,
                mime_type,
                width,
                height,
                native_width: width,
                native_height: height,
            })
        })
        .await
        .map_err(|e| Error::new("image_encoding", e.to_string()))??;
        Ok(Output {
            data: json!({"screen":options.screen,"viewport":{"x":viewport.x,"y":viewport.y,"width":viewport.width,"height":viewport.height},"pages":metadata,"completed_swipes":swipes,"stop_reason":stop_reason,"image_width":width,"image_height":height,"bytes":image.bytes.len(),"verified":false,"coverage_verified":false,"mime_type":image.mime_type,"note":"Pages are separate viewport observations with possible overlap, not a seamless screenshot. Scroll inertia can skip content; complete coverage is not verified. The list remains at its final position. Earlier pages are historical: reobserve before tapping."}),
            image: Some(image),
        })
    }
}
