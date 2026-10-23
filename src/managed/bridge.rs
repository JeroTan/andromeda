//! Bounded, authenticated state replication. Phone events never widen project scope.
use super::{ProjectState, session::Session};
use crate::{
    actions::Controller,
    companion::Companion,
    errors::{Error, Result},
    projects::Registry,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize)]
pub struct BridgeStatus {
    pub serial: String,
    pub connected: bool,
    pub code: Option<String>,
    pub message: String,
}
#[derive(Clone)]
pub(super) struct Bridge {
    companion: Companion,
    registry: Registry,
    session: Arc<Session>,
    controller: Controller,
    epoch: Arc<Mutex<u64>>,
    host_id: String,
    status: Arc<Mutex<BTreeMap<String, BridgeStatus>>>,
    gate: Arc<tokio::sync::Mutex<()>>,
}
impl Bridge {
    pub fn new(
        companion: Companion,
        registry: Registry,
        session: Arc<Session>,
        controller: Controller,
        epoch: Arc<Mutex<u64>>,
        host_id: String,
    ) -> Self {
        Self {
            companion,
            registry,
            session,
            controller,
            epoch,
            host_id,
            status: Default::default(),
            gate: Default::default(),
        }
    }
    pub fn statuses(&self) -> Vec<BridgeStatus> {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }
    pub async fn run(self, cancel: CancellationToken) {
        loop {
            tokio::select! { biased; _ = cancel.cancelled() => break, _ = self.sync() => {} }
            tokio::select! { biased; _ = cancel.cancelled() => break, _ = tokio::time::sleep(Duration::from_secs(1)) => {} }
        }
    }
    pub async fn sync(&self) -> Result<()> {
        let _gate = self.gate.lock().await;
        let projects = self.registry.list()?;
        let epoch = *self.epoch.lock().unwrap_or_else(|e| e.into_inner());
        let mut devices: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for project in projects.iter().filter(|p| p.enabled) {
            if let Some(serial) = &project.serial {
                let state = self.session.project(&project.id);
                devices.entry(serial.clone()).or_default().push(json!({"id":project.id,"name":project.name,"state":state.state,"generation":state.generation,"session_id":state.session_id,"phone_ack":state.phone_ack,"resume_epoch":state.resume_epoch}));
            }
        }
        let mut workers = tokio::task::JoinSet::new();
        for (serial, states) in devices {
            let this = self.clone();
            workers.spawn(async move {
                let result = this.sync_device(&serial, &states, epoch).await;
                if let Err(error) = &result {
                    // Unpaired ADB-only use remains available; a previously paired bridge
                    // losing contact pauses its projects until the operator resumes them.
                    if error.code != "companion_not_paired" {
                        for state in &states {
                            let id = state["id"].as_str().unwrap_or_default();
                            if this.registry.get(id).is_ok_and(|project| {
                                project.enabled && project.serial.as_deref() == Some(&serial)
                            }) {
                                // Fence current state, including Start/Resume accepted
                                // while this request was in flight. Never revive Stop.
                                this.session.bridge_failed(id);
                            }
                        }
                    }
                    this.status
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(
                            serial.clone(),
                            BridgeStatus {
                                serial,
                                connected: false,
                                code: Some(error.code.clone()),
                                message: error.message.clone(),
                            },
                        );
                }
                result
            });
        }
        let mut error = None;
        while let Some(result) = workers.join_next().await {
            match result {
                Ok(Err(problem)) => error = Some(problem),
                Err(_) => {
                    error = Some(Error::new(
                        "bridge_sync",
                        "Phone status synchronization failed",
                    ))
                }
                _ => {}
            }
        }
        error.map_or(Ok(()), Err)
    }
    async fn sync_device(&self, serial: &str, states: &[Value], epoch: u64) -> Result<()> {
        let response = tokio::time::timeout(
            Duration::from_secs(2),
            self.companion.call(
                serial,
                "session_sync",
                json!({"host_id":self.host_id,"projects":states}),
            ),
        )
        .await
        .map_err(|_| {
            Error::new(
                "bridge_disconnected",
                "Phone bridge is not responding. Its projects are paused; reconnect and resume.",
            )
        })??;
        let events = response
            .get("events")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                Error::new(
                    "companion_update_required",
                    "Update the phone app to synchronize project sessions",
                )
            })?;
        if events.len() > 100 {
            return Err(Error::new(
                "companion_protocol",
                "Too many phone session events",
            ));
        }
        let global = self.epoch.lock().unwrap_or_else(|e| e.into_inner());
        if *global != epoch {
            return Ok(());
        }
        for event in events {
            let id = event["project_id"].as_str().unwrap_or_default();
            let Some(sent) = states.iter().find(|state| state["id"] == id) else {
                continue;
            };
            let Ok(current) = self.registry.get(id) else {
                continue;
            };
            if !current.enabled || current.serial.as_deref() != Some(serial) {
                continue;
            }
            let state: ProjectState = serde_json::from_value(event["state"].clone())
                .map_err(|_| Error::new("companion_protocol", "Invalid phone project state"))?;
            let Some(session_id) = event["session_id"].as_u64() else {
                continue;
            };
            if Some(session_id) != sent["session_id"].as_u64() {
                continue;
            }
            let Some(event_id) = event["event_id"].as_u64() else {
                continue;
            };
            // Acknowledge exact phone events, not just host generations: a later
            // phone Stop/Pause can arrive after an earlier Resume was applied.
            let Some(expected_resume_epoch) = event["resume_epoch"].as_u64() else {
                continue;
            };
            self.session
                .phone_event(id, state, session_id, event_id, expected_resume_epoch);
            if state == ProjectState::Running
                && self.session.project(id).state == ProjectState::Running
            {
                self.controller.resume();
            }
        }
        drop(global);
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(
                serial.into(),
                BridgeStatus {
                    serial: serial.into(),
                    connected: true,
                    code: None,
                    message: "Phone and desktop connected".into(),
                },
            );
        Ok(())
    }
}
