use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
mod screen;
mod transport;
pub use screen::Screen;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceSummary {
    pub serial: String,
    pub state: String,
    pub details: String,
}

#[derive(Debug, Clone)]
pub struct Adb {
    pub address: SocketAddr,
}

impl Adb {
    pub async fn screen(&self, serial: &str) -> Result<Screen> {
        let bytes = self.shell(serial, "wm size && dumpsys input").await?;
        Screen::parse(&String::from_utf8_lossy(&bytes))
    }

    pub async fn ensure_screen(&self, serial: &str, expected: Screen) -> Result<()> {
        if self.screen(serial).await? != expected {
            return Err(crate::errors::Error::new(
                "stale_screen",
                "Screen dimensions or rotation changed. Observe again before acting.",
            ));
        }
        Ok(())
    }
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }

    pub async fn list_devices(&self) -> Result<Vec<DeviceSummary>> {
        let bytes = self.host_query("host:devices-l").await?;
        let text = String::from_utf8(bytes)
            .map_err(|_| crate::errors::Error::new("adb_protocol", "Device list is not UTF-8"))?;
        Ok(text
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                Some(DeviceSummary {
                    serial: fields.next()?.to_owned(),
                    state: fields.next()?.to_owned(),
                    details: fields.collect::<Vec<_>>().join(" "),
                })
            })
            .collect())
    }
}
