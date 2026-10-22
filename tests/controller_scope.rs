use andromeda::{
    actions::{Controller, Operation, RunOptions},
    device::Adb,
};
use tokio_util::sync::CancellationToken;
mod support;

#[tokio::test]
async fn operator_pause_blocks_new_input_until_the_operator_resumes() {
    use andromeda::input::{Action, Key};
    let fixture = support::DeviceFixture::start(0).await;
    let operator = Controller::new(Adb::new(fixture.address), Some("phone-A".into()), true);
    let project = operator.scoped("project", Some("phone-A".into()), true);
    operator.pause();
    let denied = project
        .run(
            Operation::Actions {
                actions: vec![Action::PressKey { key: Key::Home }],
                interval_ms: 0,
            },
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(denied.result.err().unwrap().code, "control_paused");
    assert!(fixture.commands.lock().unwrap().is_empty());
    assert!(
        project
            .run(
                Operation::ListDevices,
                RunOptions::default(),
                CancellationToken::new()
            )
            .await
            .result
            .is_ok()
    );
    operator.resume();
    let accepted = project
        .run(
            Operation::Actions {
                actions: vec![Action::PressKey { key: Key::Home }],
                interval_ms: 0,
            },
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(accepted.result.unwrap().data["completed_actions"], 1);
}

#[tokio::test]
async fn a_project_stop_cancels_its_own_work_while_operator_stop_cancels_all() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let operator = Controller::new(Adb::new(listener.local_addr().unwrap()), None, false);
    let a = operator.scoped("project-a", None, false);
    let b = operator.scoped("project-b", None, false);
    let worker = a.clone();
    let pending_a = tokio::spawn(async move {
        worker
            .run(
                Operation::ListDevices,
                RunOptions::default(),
                CancellationToken::new(),
            )
            .await
    });
    let (mut socket_a, _) = listener.accept().await.unwrap();
    support::request(&mut socket_a).await;
    let pending_b = tokio::spawn(async move {
        b.run(
            Operation::ListDevices,
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await
    });
    let (mut socket_b, _) = listener.accept().await.unwrap();
    support::request(&mut socket_b).await;
    assert_eq!(
        a.stop(),
        1,
        "One project must not cancel another project's operation"
    );
    assert_eq!(
        pending_a.await.unwrap().result.err().unwrap().code,
        "cancelled"
    );
    assert!(!pending_b.is_finished());
    assert_eq!(operator.stop(), 1);
    assert_eq!(
        pending_b.await.unwrap().result.err().unwrap().code,
        "cancelled"
    );
    assert_eq!(operator.stop(), 0);
}
