use andromeda::{
    actions::Controller,
    companion::{Companion, Store},
    device::Adb,
    managed::{Client, Host, ProjectState},
    projects::Registry,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use tokio_util::sync::CancellationToken;
mod support;

/// A phone protocol peer, with owner actions supplied by the test rather than
/// reimplementing host policy. All input still crosses the public MCP gateway.
#[tokio::test]
async fn phone_and_desktop_share_project_sessions_without_a_second_start() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::open(root.path().join("state")).unwrap();
    let store = Store::open(root.path().join("state")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let last_sync = Arc::new(Mutex::new(json!({})));
    let phone_events = Arc::new(Mutex::new(Vec::<Value>::new()));
    let offline = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let peer = tokio::spawn({
        let last_sync = last_sync.clone();
        let phone_events = phone_events.clone();
        let offline = offline.clone();
        async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let last_sync = last_sync.clone();
                let phone_events = phone_events.clone();
                let offline = offline.clone();
                tokio::spawn(async move {
                    assert_eq!(
                        support::request(&mut socket).await,
                        "host:transport:phone-A"
                    );
                    socket.write_all(b"OKAY").await.unwrap();
                    let service = support::request(&mut socket).await;
                    socket.write_all(b"OKAY").await.unwrap();
                    if service.starts_with("shell,v2,raw:") {
                        socket.write_all(&[3, 1, 0, 0, 0, 0]).await.unwrap();
                        return;
                    }
                    assert_eq!(service, "localabstract:andromeda-v1");
                    let length = socket.read_u32().await.unwrap() as usize;
                    let mut bytes = vec![0; length];
                    socket.read_exact(&mut bytes).await.unwrap();
                    let mut request: Value = serde_json::from_slice(&bytes).unwrap();
                    let response = match request["method"].as_str().unwrap() {
                        "pair" => json!({"success":true,"data":{"paired":true}}),
                        "session_sync" => {
                            assert_eq!(request["host_id"].as_str().unwrap().len(), 32);
                            request.as_object_mut().unwrap().remove("token");
                            *last_sync.lock().unwrap() = request;
                            if offline.load(std::sync::atomic::Ordering::SeqCst) {
                                return;
                            }
                            json!({"success":true,"data":{"events":phone_events.lock().unwrap().clone()}})
                        }
                        method => {
                            panic!("Unexpected phone method {method}; no separate start is allowed")
                        }
                    };
                    let bytes = serde_json::to_vec(&response).unwrap();
                    socket.write_u32(bytes.len() as u32).await.unwrap();
                    socket.write_all(&bytes).await.unwrap();
                });
            }
        }
    });
    Companion::new(Adb::new(address), store.clone())
        .pair("phone-A")
        .await
        .unwrap();
    let mut projects = Vec::new();
    let mut clients = Vec::new();
    for name in ["Project A", "Project B"] {
        let path = root.path().join(name);
        std::fs::create_dir(&path).unwrap();
        let project = registry.register(&path).unwrap();
        registry
            .update(
                &project.id,
                Some("phone-A".into()),
                vec!["adm_press_key".into()],
                "ANDROMEDA.md".into(),
                true,
            )
            .unwrap();
        clients.push(Client::new(registry.clone(), project.id.clone(), path).unwrap());
        projects.push(project);
    }
    let host = Host::start(
        registry,
        Controller::new(Adb::new(address), None, true).with_companion(store),
    )
    .await
    .unwrap();
    for project in &projects {
        host.set_project_state(
            &project.id,
            ProjectState::Running,
            0,
            host.pause_generation(),
        )
        .unwrap();
    }
    host.sync_phone_sessions().await.unwrap();
    assert!(
        last_sync.lock().unwrap()["projects"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["state"] == "running")
    );
    let a = host.project_session(&projects[0].id);
    *phone_events.lock().unwrap() = vec![
        json!({"project_id":a.project_id,"state":"paused","generation":a.generation,"session_id":a.session_id,"event_id":1}),
    ];
    host.sync_phone_sessions().await.unwrap();
    assert_eq!(
        host.project_session(&projects[0].id).state,
        ProjectState::Paused
    );
    assert_eq!(
        host.project_session(&projects[1].id).state,
        ProjectState::Running
    );
    let result = clients[1]
        .call(
            "adm_press_key".into(),
            json!({"key":"home","timeout_ms":500}),
            CancellationToken::new(),
        )
        .await;
    assert_eq!(result.structured_content.unwrap()["success"], true);
    // Phone Stop queued after a Resume response belongs to the same session;
    // it must still win when the desktop has already advanced its generation.
    let paused = host.project_session(&projects[0].id);
    host.set_project_state(
        &paused.project_id,
        ProjectState::Running,
        paused.generation,
        host.pause_generation(),
    )
    .unwrap();
    *phone_events.lock().unwrap() = vec![
        json!({"project_id":paused.project_id,"state":"stopped","generation":paused.generation,"session_id":paused.session_id,"event_id":2}),
    ];
    host.sync_phone_sessions().await.unwrap();
    assert_eq!(
        host.project_session(&projects[0].id).state,
        ProjectState::Stopped
    );
    // An old phone Resume must never resurrect a later desktop Stop.
    host.stop_project(&projects[0].id);
    *phone_events.lock().unwrap() = vec![
        json!({"project_id":a.project_id,"state":"running","generation":a.generation,"session_id":a.session_id,"event_id":1}),
    ];
    host.sync_phone_sessions().await.unwrap();
    assert_eq!(
        host.project_session(&projects[0].id).state,
        ProjectState::Stopped
    );
    offline.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(host.sync_phone_sessions().await.is_err());
    assert_eq!(
        host.project_session(&projects[1].id).state,
        ProjectState::Paused
    );
    offline.store(false, std::sync::atomic::Ordering::SeqCst);
    host.sync_phone_sessions().await.unwrap();
    assert_eq!(
        host.project_session(&projects[1].id).state,
        ProjectState::Paused,
        "Reconnecting never silently resumes input"
    );
    host.shutdown();
    peer.abort();
}
