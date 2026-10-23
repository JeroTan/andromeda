use super::{Adb, transport::ShellStream};
use crate::errors::{Error, Result};
use std::time::{Duration, Instant};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const MAX_FRAME: usize = 32 * 1024 * 1024;
const MAX_BUFFER: usize = MAX_FRAME + 1024 * 1024;

/// A persistent Android `screencap` loop over one shell-v2 connection.
///
/// The device writes native PNG frames back-to-back. Frames are separated by
/// their PNG chunk structure, so no delimiter can collide with image bytes.
pub struct ScreenStream {
    shell: ShellStream,
    buffer: Vec<u8>,
}

impl Adb {
    /// Open a persistent capture stream. Dropping the stream closes the device
    /// shell and stops the capture loop.
    pub async fn stream(&self, serial: &str, interval_ms: u64) -> Result<ScreenStream> {
        if interval_ms > 5000 {
            return Err(Error::invalid(
                "Stream interval must be between 0 and 5000 milliseconds",
            ));
        }
        let command = if interval_ms == 0 {
            "while true; do screencap -p || exit; done".to_owned()
        } else {
            format!(
                "while true; do screencap -p || exit; sleep {}; done",
                interval_ms as f64 / 1000.0
            )
        };
        let socket = self
            .service(serial, &format!("shell,v2,raw:{command}"))
            .await?;
        Ok(ScreenStream {
            shell: ShellStream {
                socket,
                stderr: Vec::new(),
            },
            buffer: Vec::new(),
        })
    }
}

impl ScreenStream {
    /// Read the next complete PNG frame, waiting at most `timeout` for it.
    pub async fn next_frame(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(frame) = take_frame(&mut self.buffer)? {
                return Ok(Some(frame));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::new(
                    "stream_timeout",
                    "The persistent capture stream did not produce a complete frame before the deadline",
                ));
            }
            let chunk = match tokio::time::timeout(remaining, self.shell.next()).await {
                Ok(Ok(Some(chunk))) => chunk,
                Ok(Ok(None)) => {
                    return if self.buffer.is_empty() {
                        Ok(None)
                    } else {
                        Err(Error::new(
                            "stream_ended",
                            "The persistent capture stream ended inside a frame",
                        ))
                    };
                }
                Ok(Err(error)) => return Err(error),
                Err(_) => {
                    return Err(Error::new(
                        "stream_timeout",
                        "The persistent capture stream did not produce a complete frame before the deadline",
                    ));
                }
            };
            if self.buffer.len() + chunk.len() > MAX_BUFFER {
                return Err(Error::new(
                    "output_limit",
                    "Persistent capture buffered more than 33 MiB without a complete frame",
                ));
            }
            self.buffer.extend_from_slice(&chunk);
        }
    }
}

fn take_frame(buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>> {
    let Some(start) = buffer
        .windows(PNG_SIGNATURE.len())
        .position(|window| window == PNG_SIGNATURE)
    else {
        // Keep a possible partial signature, but do not grow on unrelated output.
        if buffer.len() > PNG_SIGNATURE.len() - 1 {
            let keep = buffer.len() - (PNG_SIGNATURE.len() - 1);
            buffer.drain(..keep);
        }
        return Ok(None);
    };
    if start > 0 {
        buffer.drain(..start);
    }
    if buffer.len() < PNG_SIGNATURE.len() + 8 {
        return Ok(None);
    }
    let mut position = PNG_SIGNATURE.len();
    loop {
        if buffer.len() < position + 8 {
            return Ok(None);
        }
        let length = u32::from_be_bytes(
            buffer[position..position + 4]
                .try_into()
                .map_err(|_| Error::new("invalid_stream_frame", "Malformed PNG length"))?,
        ) as usize;
        if length > MAX_FRAME - 12 {
            return Err(Error::new(
                "invalid_stream_frame",
                "Persistent capture frame contains an oversized PNG chunk",
            ));
        }
        let end = position
            .checked_add(12 + length)
            .filter(|end| *end <= MAX_FRAME)
            .ok_or_else(|| {
                Error::new(
                    "invalid_stream_frame",
                    "Persistent capture frame exceeds 32 MiB",
                )
            })?;
        if buffer.len() < end {
            return Ok(None);
        }
        let kind = &buffer[position + 4..position + 8];
        if position == PNG_SIGNATURE.len() && kind != b"IHDR" {
            return Err(Error::new(
                "invalid_stream_frame",
                "Persistent capture frame does not start with IHDR",
            ));
        }
        position = end;
        if kind == b"IEND" {
            let frame = buffer[..position].to_vec();
            buffer.drain(..position);
            return Ok(Some(frame));
        }
    }
}
