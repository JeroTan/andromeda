//! Real-duration acceptance probe. Only targets an explicitly paired disposable emulator.
use andromeda::{
    actions::Controller,
    companion::{Companion, Store},
    device::Adb,
    managed::{Client, Host, ProjectState},
    projects::Registry,
};
use serde_json::json;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

struct RegisteredProbe {
    registry: Registry,
    id: String,
}
impl Drop for RegisteredProbe {
    fn drop(&mut self) {
        if let Err(error) = self.registry.remove(&self.id) {
            eprintln!("Could not remove temporary timer project: {error}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 || !args[0].starts_with("emulator-") {
        return Err("Usage: verify_session_timer emulator-SERIAL PRIVATE_STATE NEW_REPORT.json; first select 15 minutes in the emulator app and pair it".into());
    }
    let serial = &args[0];
    let report = PathBuf::from(&args[2]);
    if report.exists() {
        return Err("Report already exists".into());
    }
    let state = PathBuf::from(&args[1]);
    let registry = Registry::open(state.clone())?;
    let store = Store::open(state)?;
    let adb = Adb::new("127.0.0.1:5037".parse()?);
    let companion = Companion::new(adb.clone(), store.clone());
    if !companion.is_paired(serial)? {
        return Err("Pair this disposable emulator first".into());
    }
    let workspace = tempfile::tempdir()?;
    let project = registry.register(workspace.path())?;
    let _registration = RegisteredProbe {
        registry: registry.clone(),
        id: project.id.clone(),
    };
    registry.update(
        &project.id,
        Some(serial.clone()),
        vec!["adm_press_key".into(), "adm_get_device_info".into()],
        "ANDROMEDA.md".into(),
        true,
    )?;
    let host = Host::start(
        registry.clone(),
        Controller::new(adb, Some(serial.clone()), true).with_companion(store),
    )
    .await?;
    let client = Client::new(registry, project.id.clone(), workspace.path().to_path_buf())?;
    host.set_project_state(
        &project.id,
        ProjectState::Running,
        0,
        host.pause_generation(),
    )?;
    let started = Instant::now();
    let mut initial_seconds = None;
    let mut samples = Vec::new();
    let mut last_minute = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let elapsed = started.elapsed().as_secs_f64();
        let phone = companion.status(serial).await?;
        let current = host.project_session(&project.id);
        let seconds = phone["session_seconds"]
            .as_i64()
            .ok_or("Missing phone timer")?;
        if initial_seconds.is_none() && phone["session_active"] == true {
            if !(895..=900).contains(&seconds) {
                return Err(format!("Expected a fresh 15-minute session, got {seconds}").into());
            }
            initial_seconds = Some(seconds);
            println!("15-minute timer started; authenticated desktop heartbeats active.");
        }
        samples.push(json!({"elapsed_seconds":elapsed,"phone_active":phone["session_active"],"phone_seconds":seconds,"desktop_state":current.state}));
        if current.state == ProjectState::Paused {
            return Err("Session paused before timer expiry (heartbeat/connection failure)".into());
        }
        if current.state == ProjectState::Stopped {
            if initial_seconds.is_none()
                || !(899.0..=910.0).contains(&elapsed)
                || phone["session_active"] != false
            {
                return Err(format!("Unexpected stop at {elapsed} seconds").into());
            }
            let result = client
                .call(
                    "adm_press_key".into(),
                    json!({"key":"home"}),
                    CancellationToken::new(),
                )
                .await;
            let status = result
                .structured_content
                .ok_or("Missing structured result")?;
            if status["error"]["code"] != "project_stopped" {
                return Err("Expired project admitted input".into());
            }
            let evidence = json!({"schema_version":1,"timer_minutes":15,"clock_accelerated":false,"environment":"disposable Android emulator","initial_seconds":initial_seconds,"elapsed_to_desktop_stop_seconds":elapsed,"input_after_expiry_error":status["error"]["code"],"samples":samples,"limitations":["One real elapsed-time run with authenticated desktop heartbeats; not physical-device power-management evidence.","Phone timer selected through Settings before starting this probe; no private timer state or clock is modified."]});
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&report)?;
            serde_json::to_writer_pretty(&mut file, &evidence)?;
            println!(
                "Verified expiry and desktop propagation after {elapsed:.3} seconds; later input rejected."
            );
            return Ok(());
        }
        if elapsed > 930.0 {
            return Err("Timer failed to stop within 930 seconds".into());
        }
        let minute = elapsed as u64 / 60;
        if minute > last_minute {
            println!(
                "Timer verification: {minute} elapsed minutes; phone remaining {seconds}s; desktop Running."
            );
            last_minute = minute;
        }
    }
}
