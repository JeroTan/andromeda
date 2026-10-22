use andromeda::device::Adb;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
mod support;

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
