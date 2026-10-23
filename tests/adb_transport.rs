use andromeda::device::Adb;
use image::ImageEncoder;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
mod support;

fn test_png() -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            &[
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            2,
            2,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    bytes
}

async fn send_shell_chunk(socket: &mut tokio::net::TcpStream, bytes: &[u8]) {
    socket.write_u8(1).await.unwrap();
    socket.write_u32_le(bytes.len() as u32).await.unwrap();
    socket.write_all(bytes).await.unwrap();
}

#[tokio::test]
async fn launch_failure_preserves_android_stdout_diagnostics() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adb = Adb::new(listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        support::request(&mut socket).await;
        socket.write_all(b"OKAY").await.unwrap();
        support::request(&mut socket).await;
        socket.write_all(b"OKAY").await.unwrap();
        let message = b"Error: Activity not started, unable to resolve Intent for missing.example";
        socket.write_u8(1).await.unwrap();
        socket.write_u32_le(message.len() as u32).await.unwrap();
        socket.write_all(message).await.unwrap();
        socket.write_all(&[3, 1, 0, 0, 0, 1]).await.unwrap();
    });
    let error = adb
        .shell("phone-A", "am start -W -p missing.example")
        .await
        .unwrap_err();
    assert!(error.message.contains("unable to resolve"));
    server.await.unwrap();
}

#[tokio::test]
async fn shell_reports_device_exit_failure_instead_of_claiming_success() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adb = Adb::new(listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        for expected in ["host:transport:phone-A", "shell,v2,raw:input tap 10 20"] {
            let mut prefix = [0; 4];
            stream.read_exact(&mut prefix).await.unwrap();
            let length = usize::from_str_radix(std::str::from_utf8(&prefix).unwrap(), 16).unwrap();
            let mut request = vec![0; length];
            stream.read_exact(&mut request).await.unwrap();
            assert_eq!(request, expected.as_bytes());
            stream.write_all(b"OKAY").await.unwrap();
        }
        let message = b"Permission denied";
        stream.write_u8(2).await.unwrap();
        stream.write_u32_le(message.len() as u32).await.unwrap();
        stream.write_all(message).await.unwrap();
        stream.write_all(&[3, 1, 0, 0, 0, 1]).await.unwrap();
    });
    let error = adb.shell("phone-A", "input tap 10 20").await.unwrap_err();
    assert_eq!(error.code, "device_command_failed");
    assert!(error.message.contains("Permission denied"));
    server.await.unwrap();
}

#[tokio::test]
async fn discovery_preserves_authorization_state_and_device_identity() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adb = Adb::new(listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut prefix = [0; 4];
        stream.read_exact(&mut prefix).await.unwrap();
        let length = usize::from_str_radix(std::str::from_utf8(&prefix).unwrap(), 16).unwrap();
        let mut request = vec![0; length];
        stream.read_exact(&mut request).await.unwrap();
        assert_eq!(request, b"host:devices-l");
        let devices = b"phone-A\tdevice product:pixel model:Pixel_8 transport_id:1\nphone-B\tunauthorized transport_id:2\n";
        // TCP can split any field across packets.
        for byte in format!("OKAY{:04x}", devices.len())
            .bytes()
            .chain(devices.iter().copied())
        {
            stream.write_all(&[byte]).await.unwrap();
        }
    });
    let devices = adb.list_devices().await.unwrap();
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].serial, "phone-A");
    assert_eq!(devices[1].state, "unauthorized");
    server.await.unwrap();
}

#[tokio::test]
async fn persistent_stream_returns_complete_png_frames_across_chunks() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adb = Adb::new(listener.local_addr().unwrap());
    let png = test_png();
    let expected = png.clone();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        assert_eq!(
            support::request(&mut socket).await,
            "host:transport:phone-A"
        );
        socket.write_all(b"OKAY").await.unwrap();
        let service = support::request(&mut socket).await;
        assert!(service.starts_with("shell,v2,raw:while true; do screencap -p"));
        socket.write_all(b"OKAY").await.unwrap();
        send_shell_chunk(&mut socket, b"shell banner\n").await;
        for chunk in png.chunks(7) {
            send_shell_chunk(&mut socket, chunk).await;
        }
        for chunk in png.chunks(11) {
            send_shell_chunk(&mut socket, chunk).await;
        }
        socket.write_all(&[3, 1, 0, 0, 0, 0]).await.unwrap();
    });
    let mut stream = adb.stream("phone-A", 0).await.unwrap();
    assert_eq!(
        stream
            .next_frame(Duration::from_secs(1))
            .await
            .unwrap()
            .unwrap(),
        expected
    );
    assert_eq!(
        stream
            .next_frame(Duration::from_secs(1))
            .await
            .unwrap()
            .unwrap(),
        expected
    );
    assert!(
        stream
            .next_frame(Duration::from_secs(1))
            .await
            .unwrap()
            .is_none()
    );
    server.await.unwrap();
}

#[tokio::test]
async fn persistent_stream_rejects_an_oversized_png_chunk() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adb = Adb::new(listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        support::request(&mut socket).await;
        socket.write_all(b"OKAY").await.unwrap();
        support::request(&mut socket).await;
        socket.write_all(b"OKAY").await.unwrap();
        let mut frame = b"\x89PNG\r\n\x1a\n".to_vec();
        frame.extend_from_slice(&u32::MAX.to_be_bytes());
        frame.extend_from_slice(b"IHDR");
        send_shell_chunk(&mut socket, &frame).await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let mut stream = adb.stream("phone-A", 0).await.unwrap();
    let error = stream.next_frame(Duration::from_secs(1)).await.unwrap_err();
    assert_eq!(error.code, "invalid_stream_frame");
    server.abort();
}

#[tokio::test]
async fn persistent_stream_times_out_without_a_complete_frame() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adb = Adb::new(listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        support::request(&mut socket).await;
        socket.write_all(b"OKAY").await.unwrap();
        support::request(&mut socket).await;
        socket.write_all(b"OKAY").await.unwrap();
        send_shell_chunk(&mut socket, b"not a frame").await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    });
    let mut stream = adb.stream("phone-A", 0).await.unwrap();
    let error = stream
        .next_frame(Duration::from_millis(20))
        .await
        .unwrap_err();
    assert_eq!(error.code, "stream_timeout");
    server.abort();
}
