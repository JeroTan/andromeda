use super::Adb;
use crate::errors::{Error, Result};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

impl Adb {
    /// Execute an Android shell service without spawning a host process.
    /// Failure after dispatch is never retried: input may already have happened.
    pub async fn shell(&self, serial: &str, command: &str) -> Result<Vec<u8>> {
        tokio::time::timeout(Duration::from_secs(60), async {
            let socket = self
                .service(serial, &format!("shell,v2,raw:{command}"))
                .await?;
            let mut shell = ShellStream {
                socket,
                stderr: Vec::new(),
            };
            let mut output = Vec::new();
            loop {
                let chunk = match shell.next().await {
                    Ok(Some(chunk)) => chunk,
                    Ok(None) => break,
                    Err(mut error) => {
                        if error.code == "device_command_failed" && !output.is_empty() {
                            // am start -W writes useful failures to stdout. Keep
                            // a bounded excerpt in the caller's error, never logs.
                            let excerpt = &output[..output.len().min(4096)];
                            error
                                .message
                                .push_str(&format!("\n{}", String::from_utf8_lossy(excerpt)));
                        }
                        return Err(error);
                    }
                };
                if output.len() + chunk.len() > MAX_OUTPUT {
                    return Err(Error::new("output_limit", "Device output exceeded 32 MiB"));
                }
                output.extend_from_slice(&chunk);
            }
            Ok(output)
        })
        .await
        .map_err(|_| {
            Error::new(
                "timeout",
                "Device command exceeded 60 seconds; observe before retrying",
            )
        })?
    }

    pub async fn service(&self, serial: &str, service: &str) -> Result<TcpStream> {
        if serial.is_empty()
            || serial.len() > 256
            || !serial
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
        {
            return Err(Error::invalid("Invalid Android serial"));
        }
        let mut socket = self.connect().await?;
        request(&mut socket, &format!("host:transport:{serial}")).await?;
        request(&mut socket, service).await?;
        Ok(socket)
    }

    pub(super) async fn connect(&self) -> Result<TcpStream> {
        if !self.address.ip().is_loopback() {
            return Err(Error::invalid("ADB server must use a loopback address"));
        }
        let socket = TcpStream::connect(self.address).await.map_err(|e| {
            Error::new(
                "adb_unavailable",
                format!("Cannot reach local ADB server: {e}. Run `adb start-server`."),
            )
        })?;
        socket.set_nodelay(true)?;
        Ok(socket)
    }

    pub(super) async fn host_query(&self, command: &str) -> Result<Vec<u8>> {
        tokio::time::timeout(Duration::from_secs(10), async {
            let mut socket = self.connect().await?;
            request(&mut socket, command).await?;
            read_string(&mut socket).await
        })
        .await
        .map_err(|_| Error::new("timeout", "ADB discovery exceeded 10 seconds"))?
    }
}

const MAX_OUTPUT: usize = 32 * 1024 * 1024;

/// A bounded shell-v2 reader; dropping it closes the logical device connection.
pub(super) struct ShellStream {
    pub socket: TcpStream,
    pub stderr: Vec<u8>,
}

impl ShellStream {
    pub async fn next(&mut self) -> Result<Option<Vec<u8>>> {
        loop {
            let channel = self.socket.read_u8().await?;
            let length = self.socket.read_u32_le().await? as usize;
            if length > 1024 * 1024 {
                return Err(Error::new("adb_protocol", "Shell packet exceeds 1 MiB"));
            }
            let mut bytes = vec![0; length];
            self.socket.read_exact(&mut bytes).await?;
            match channel {
                1 => return Ok(Some(bytes)),
                2 => {
                    if self.stderr.len() + length > 65536 {
                        return Err(Error::new(
                            "output_limit",
                            "Device error output exceeded 64 KiB",
                        ));
                    }
                    self.stderr.extend_from_slice(&bytes);
                }
                3 => {
                    let code = match bytes.as_slice() {
                        [value] => *value as u32,
                        [a, b, c, d] => u32::from_le_bytes([*a, *b, *c, *d]),
                        _ => return Err(Error::new("adb_protocol", "Malformed shell exit status")),
                    };
                    if code != 0 {
                        return Err(Error::new(
                            "device_command_failed",
                            format!(
                                "Android command exited {code}: {}",
                                String::from_utf8_lossy(&self.stderr)
                            ),
                        ));
                    }
                    return Ok(None);
                }
                _ => return Err(Error::new("adb_protocol", "Unexpected shell channel")),
            }
        }
    }
}

pub(super) async fn request(socket: &mut TcpStream, command: &str) -> Result<()> {
    if command.len() > 65535 {
        return Err(Error::invalid("ADB request exceeds 65535 bytes"));
    }
    socket
        .write_all(format!("{:04x}", command.len()).as_bytes())
        .await?;
    socket.write_all(command.as_bytes()).await?;
    let mut status = [0; 4];
    socket.read_exact(&mut status).await?;
    match &status {
        b"OKAY" => Ok(()),
        b"FAIL" => {
            let reason = read_string(socket).await?;
            Err(Error::new("adb_rejected", String::from_utf8_lossy(&reason)))
        }
        _ => Err(Error::new("adb_protocol", "Invalid ADB status frame")),
    }
}

async fn read_string(socket: &mut TcpStream) -> Result<Vec<u8>> {
    let mut prefix = [0; 4];
    socket.read_exact(&mut prefix).await?;
    let length = std::str::from_utf8(&prefix)
        .ok()
        .and_then(|s| usize::from_str_radix(s, 16).ok())
        .ok_or_else(|| Error::new("adb_protocol", "Invalid ADB length prefix"))?;
    let mut bytes = vec![0; length];
    socket.read_exact(&mut bytes).await?;
    Ok(bytes)
}
