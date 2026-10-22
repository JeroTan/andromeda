use andromeda::{
    actions::{Controller, Operation, RunOptions},
    device::Adb,
    input::{Action, Key},
};
use tokio_util::sync::CancellationToken;
mod support;

#[tokio::test]
async fn rotation_change_rejects_tap_before_android_receives_input() {
    use andromeda::{device::Screen, input::Point};
    let fixture = support::DeviceFixture::start(1).await;
    let controller = Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true);
    let report = controller
        .run(
            Operation::Actions {
                actions: vec![Action::Touch {
                    point: Point { x: 500, y: 1000 },
                    screen: Screen {
                        width: 1080,
                        height: 2400,
                        rotation: 0,
                    },
                }],
                interval_ms: 0,
            },
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(report.result.err().unwrap().code, "stale_screen");
    assert!(
        !fixture
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|command| command.starts_with("input"))
    );
}

#[tokio::test]
async fn whole_batch_is_validated_before_any_input() {
    let fixture = support::DeviceFixture::start(0).await;
    let controller = Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true);
    let report = controller
        .run(
            Operation::Actions {
                actions: vec![
                    Action::PressKey { key: Key::Home },
                    Action::TypeText {
                        text: "unsupported 🦀".into(),
                    },
                ],
                interval_ms: 0,
            },
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(report.result.err().unwrap().code, "invalid_argument");
    assert!(fixture.commands.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ui_tree_preserves_hierarchy_and_redacts_password_values() {
    let fixture = support::DeviceFixture::start(0).await;
    let controller = Controller::new(Adb::new(fixture.address), Some("phone-A".into()), false);
    let report = controller
        .run(
            Operation::UiTree,
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await;
    let data = report.result.unwrap().data;
    assert_eq!(data["nodes"][1]["text"], "");
    assert_eq!(data["nodes"][2]["parent"], 0);
    assert_eq!(data["nodes"][2]["text"], "Display & brightness");
}

#[tokio::test]
async fn stop_interrupts_active_wait_and_next_request_still_works() {
    use tokio::{io::AsyncWriteExt, net::TcpListener};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let controller = Controller::new(Adb::new(listener.local_addr().unwrap()), None, false);
    let worker = controller.clone();
    let pending = tokio::spawn(async move {
        worker
            .run(
                Operation::ListDevices,
                RunOptions::default(),
                CancellationToken::new(),
            )
            .await
    });
    let (mut socket, _) = listener.accept().await.unwrap();
    assert_eq!(support::request(&mut socket).await, "host:devices-l");
    assert_eq!(controller.stop(), 1);
    let cancelled = tokio::time::timeout(std::time::Duration::from_secs(1), pending)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cancelled.result.err().unwrap().code, "cancelled");
    let worker = controller.clone();
    let next = tokio::spawn(async move {
        worker
            .run(
                Operation::ListDevices,
                RunOptions::default(),
                CancellationToken::new(),
            )
            .await
    });
    let (mut socket, _) = listener.accept().await.unwrap();
    support::request(&mut socket).await;
    socket.write_all(b"OKAY0000").await.unwrap();
    assert_eq!(
        next.await.unwrap().result.unwrap().data["devices"],
        serde_json::json!([])
    );
    assert_eq!(controller.stop(), 0);
}

#[tokio::test]
async fn timeout_bounds_a_stalled_adb_server() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let controller = Controller::new(Adb::new(listener.local_addr().unwrap()), None, false);
    let report = controller
        .run(
            Operation::ListDevices,
            RunOptions {
                serial: None,
                timeout_ms: Some(20),
            },
            CancellationToken::new(),
        )
        .await;
    assert_eq!(report.result.err().unwrap().code, "timeout");
    assert_eq!(controller.stop(), 0);
}

#[tokio::test]
async fn observation_only_session_rejects_input_before_connecting_to_device() {
    let controller = Controller::new(
        Adb::new("127.0.0.1:1".parse().unwrap()),
        Some("phone-A".into()),
        false,
    );
    let report = controller
        .run(
            Operation::Actions {
                actions: vec![Action::PressKey { key: Key::Home }],
                interval_ms: 0,
            },
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await;
    let error = report
        .result
        .err()
        .expect("input must require operator authorization");
    assert_eq!(error.code, "control_disabled");
}
