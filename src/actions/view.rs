use super::{Controller, Output};
use crate::{
    capture,
    errors::{Error, Result},
};
use serde_json::json;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum CaptureBackend {
    /// Prefer a paired Android screenshot API, falling back to ADB when unavailable.
    #[default]
    Auto,
    /// Use Android shell PNG capture; useful for matched benchmarks and older phones.
    Adb,
    /// Require the paired Android screenshot API; report errors without fallback.
    Companion,
}
impl Controller {
    pub(super) async fn view(&self, serial: &str, options: capture::Options) -> Result<Output> {
        options.validate()?;
        let permit = self
            .inner
            .image_gate
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Error::new("shutdown", "Image processor closed"))?;
        let mut fallback = None;
        if !matches!(self.capture_backend, CaptureBackend::Adb) {
            if let Some(companion) = &self.companion {
                if companion.is_paired(serial)? {
                    let format = match options.format {
                        capture::Format::Jpeg => "jpeg",
                        capture::Format::Webp => "webp",
                        capture::Format::WebpLossless => "webp_lossless",
                        _ => "png",
                    };
                    let started = Instant::now();
                    match companion.call(serial,"screenshot",json!({"format":format,"max_width":options.max_width.unwrap_or(8192),"max_height":options.max_height.unwrap_or(8192),"quality":options.quality.unwrap_or(80)})).await {
                        Ok(frame)=>{
                            let capture_ms=started.elapsed().as_secs_f64()*1000.0;
                            let (image,screen,device,processing_ms)=tokio::task::spawn_blocking(move||{
                                let _permit=permit;let started=Instant::now();
                                capture::from_companion(frame,options).map(|(image,screen,timings)|(image,screen,timings,started.elapsed().as_secs_f64()*1000.0))
                            }).await.map_err(|_|Error::new("image_encoding","Companion image processing failed"))??;
                            return Ok(Output {data:json!({"serial":serial,"screen":screen,"image_width":image.width,"image_height":image.height,"scale_to_screen":{"x":screen.width as f64/image.width as f64,"y":screen.height as f64/image.height as f64},"bytes":image.bytes.len(),"mime_type":image.mime_type,"capture_backend":"companion","timings":{"capture_ms":capture_ms,"image_processing_ms":processing_ms,"geometry_before_ms":0.0,"geometry_after_ms":0.0,"geometry_source":"companion_checked","device_capture_ms":device.capture_ms,"device_image_processing_ms":device.processing_ms,"api_timestamp_age_ms":device.api_timestamp_age_ms}}),image:Some(image)});
                        },
                        Err(error) if matches!(self.capture_backend,CaptureBackend::Auto) && matches!(error.code.as_str(),"unknown_method"|"companion_unavailable"|"session_inactive"|"capture_unavailable"|"capture_permission"|"capture_throttled"|"capture_busy"|"output_limit"|"timeout")=>fallback=Some(error.code),
                        Err(error)=>return Err(error),
                    }
                } else if matches!(self.capture_backend, CaptureBackend::Companion) {
                    return Err(Error::new(
                        "companion_not_paired",
                        "Pair the optional phone bridge before selecting companion capture",
                    ));
                }
            } else if matches!(self.capture_backend, CaptureBackend::Companion) {
                return Err(Error::new(
                    "companion_not_paired",
                    "Configure the optional phone bridge before selecting companion capture",
                ));
            }
        }
        let started = Instant::now();
        let screen = self.inner.adb.screen(serial).await?;
        let before = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let png = self.inner.adb.shell(serial, "screencap -p").await?;
        let capture_ms = started.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        self.inner.adb.ensure_screen(serial, screen).await?;
        let after = started.elapsed().as_secs_f64() * 1000.0;
        let (image, processing_ms) = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let started = Instant::now();
            capture::encode(png, options)
                .map(|image| (image, started.elapsed().as_secs_f64() * 1000.0))
        })
        .await
        .map_err(|_| Error::new("image_encoding", "Image processing failed"))??;
        if image.native_width != screen.width || image.native_height != screen.height {
            return Err(Error::new(
                "stale_screen",
                "Screenshot dimensions disagree with display geometry; observe again",
            ));
        }
        Ok(Output {
            data: json!({"serial":serial,"screen":screen,"image_width":image.width,"image_height":image.height,"scale_to_screen":{"x":screen.width as f64/image.width as f64,"y":screen.height as f64/image.height as f64},"bytes":image.bytes.len(),"mime_type":image.mime_type,"capture_backend":"adb","capture_fallback_code":fallback,"timings":{"geometry_before_ms":before,"capture_ms":capture_ms,"geometry_after_ms":after,"image_processing_ms":processing_ms}}),
            image: Some(image),
        })
    }
}
