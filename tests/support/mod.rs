#![allow(dead_code)]
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

pub struct DeviceFixture {
    pub address: SocketAddr,
    pub commands: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}
impl Drop for DeviceFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn request(socket: &mut TcpStream) -> String {
    let mut prefix = [0; 4];
    socket.read_exact(&mut prefix).await.unwrap();
    let length = usize::from_str_radix(std::str::from_utf8(&prefix).unwrap(), 16).unwrap();
    let mut request = vec![0; length];
    socket.read_exact(&mut request).await.unwrap();
    String::from_utf8(request).unwrap()
}

impl DeviceFixture {
    pub async fn start(rotation: u8) -> Self {
        Self::with_ui_rotation(rotation, rotation).await
    }
    pub async fn with_ui_rotation(rotation: u8, ui_rotation: u8) -> Self {
        Self::with_capture_output(rotation, ui_rotation, None).await
    }
    pub async fn with_capture(png: Vec<u8>) -> Self {
        Self::with_capture_output(0, 0, Some(png)).await
    }
    async fn with_capture_output(rotation: u8, ui_rotation: u8, png: Option<Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let commands = Arc::new(Mutex::new(Vec::new()));
        let observed = commands.clone();
        let task = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let service = request(&mut socket).await;
                if service == "host:devices-l" {
                    let data = b"phone-A\tdevice model:Test_Phone\n";
                    socket
                        .write_all(format!("OKAY{:04x}", data.len()).as_bytes())
                        .await
                        .unwrap();
                    socket.write_all(data).await.unwrap();
                    continue;
                }
                assert_eq!(service, "host:transport:phone-A");
                socket.write_all(b"OKAY").await.unwrap();
                let service = request(&mut socket).await;
                let command = service.strip_prefix("shell,v2,raw:").unwrap().to_owned();
                observed.lock().unwrap().push(command.clone());
                socket.write_all(b"OKAY").await.unwrap();
                let output=match command.as_str() {
                    "wm size && dumpsys input"=>format!("Physical size: 1080x2400\nSurfaceOrientation: {rotation}\n"),
                    command if command.contains("uiautomator dump")=>"<?xml version=\"1.0\"?><hierarchy rotation=\"0\"><node text=\"Settings\" enabled=\"true\" bounds=\"[0,0][1080,100]\"><node text=\"secret\" password=\"true\"/><node text=\"Display &amp; brightness\" resource-id=\"android:id/title\" enabled=\"true\"/></node></hierarchy>".into(),
                    _=>String::new(),
                };
                let output = output.replace(
                    "<hierarchy rotation=\"0\">",
                    &format!("<hierarchy rotation=\"{ui_rotation}\">"),
                );
                let output = if command == "screencap -p" {
                    png.clone().unwrap_or_default()
                } else {
                    output.into_bytes()
                };
                socket.write_u8(1).await.unwrap();
                socket.write_u32_le(output.len() as u32).await.unwrap();
                socket.write_all(&output).await.unwrap();
                socket.write_all(&[3, 1, 0, 0, 0, 0]).await.unwrap();
            }
        });
        Self {
            address,
            commands,
            task,
        }
    }
}
