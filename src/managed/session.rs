use crate::{
    errors::{Error, Result},
    projects::Project,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize)]
pub struct Approval {
    pub id: u64,
    pub project_id: String,
    pub project_name: String,
    pub project_root: std::path::PathBuf,
    pub serial: Option<String>,
    pub tool: String,
    pub arguments: Value,
    pub expires_at_ms: u64,
}

struct Active {
    project: String,
    cancel: CancellationToken,
    approval: Option<(Approval, oneshot::Sender<bool>)>,
}

#[derive(Default)]
pub(super) struct Session {
    next: AtomicU64,
    active: Mutex<BTreeMap<u64, Active>>,
    connections: Mutex<Connections>,
}

#[derive(Clone, Serialize)]
pub struct ConnectionStatus {
    pub project_id: String,
    pub clients: usize,
    pub initialization_error: Option<String>,
}

#[derive(Default)]
struct Connections {
    leases: BTreeMap<String, (String, Instant)>,
    errors: BTreeMap<String, String>,
}

impl Connections {
    fn expire(&mut self) {
        self.leases
            .retain(|_, (_, seen)| seen.elapsed() < Duration::from_secs(15));
    }
}

pub(super) struct Admission {
    session: Arc<Session>,
    id: u64,
}

impl Drop for Admission {
    fn drop(&mut self) {
        self.session
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.id);
    }
}

impl Session {
    pub fn connection_event(
        &self,
        project: &str,
        id: &str,
        event: super::ConnectionEvent,
    ) -> Result<()> {
        if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::invalid("Invalid managed connection ID"));
        }
        let mut connections = self.connections.lock().unwrap_or_else(|e| e.into_inner());
        connections.expire();
        if connections
            .leases
            .get(id)
            .is_some_and(|(owner, _)| owner != project)
        {
            return Err(Error::new(
                "connection_mismatch",
                "Connection belongs to another project",
            ));
        }
        if !matches!(event, super::ConnectionEvent::Closed) {
            if connections.leases.len() >= 128 && !connections.leases.contains_key(id) {
                return Err(Error::new(
                    "busy",
                    "At most 128 managed clients may connect",
                ));
            }
            connections
                .leases
                .insert(id.into(), (project.into(), Instant::now()));
            if matches!(event, super::ConnectionEvent::Initialized) {
                connections.errors.remove(project);
            }
        } else {
            connections.leases.remove(id);
        }
        Ok(())
    }

    pub fn initialization_error(&self, project: &str, error: &Error) {
        let mut connections = self.connections.lock().unwrap_or_else(|e| e.into_inner());
        if connections.errors.len() < 100 || connections.errors.contains_key(project) {
            connections
                .errors
                .insert(project.into(), error.message.clone());
        }
    }

    pub fn connections(&self) -> Vec<ConnectionStatus> {
        let mut connections = self.connections.lock().unwrap_or_else(|e| e.into_inner());
        connections.expire();
        let mut statuses = BTreeMap::<String, ConnectionStatus>::new();
        for (project, _) in connections.leases.values() {
            statuses
                .entry(project.clone())
                .or_insert(ConnectionStatus {
                    project_id: project.clone(),
                    clients: 0,
                    initialization_error: None,
                })
                .clients += 1;
        }
        for (project, error) in &connections.errors {
            statuses
                .entry(project.clone())
                .or_insert(ConnectionStatus {
                    project_id: project.clone(),
                    clients: 0,
                    initialization_error: None,
                })
                .initialization_error = Some(error.clone());
        }
        statuses.into_values().collect()
    }

    pub fn admit(self: &Arc<Self>, project: &str, cancel: CancellationToken) -> Result<Admission> {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if active.len() >= 32 {
            return Err(Error::new(
                "busy",
                "At most 32 managed operations may be active",
            ));
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        active.insert(
            id,
            Active {
                project: project.into(),
                cancel,
                approval: None,
            },
        );
        Ok(Admission {
            session: self.clone(),
            id,
        })
    }

    pub fn stop(&self, project: Option<&str>) -> usize {
        let active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        let mut count = 0;
        for operation in active
            .values()
            .filter(|operation| project.is_none_or(|id| operation.project == id))
        {
            operation.cancel.cancel();
            count += 1;
        }
        count
    }

    pub fn approvals(&self) -> Vec<Approval> {
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|active| !active.cancel.is_cancelled())
            .filter_map(|active| {
                active
                    .approval
                    .as_ref()
                    .map(|(approval, _)| approval.clone())
            })
            .collect()
    }

    pub fn decide(&self, id: u64, allow: bool) -> Result<()> {
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        let operation = active
            .get_mut(&id)
            .filter(|a| !a.cancel.is_cancelled())
            .ok_or_else(|| {
                Error::new(
                    "approval_expired",
                    "This request is no longer waiting for approval",
                )
            })?;
        let (approval, sender) = operation
            .approval
            .take()
            .ok_or_else(|| Error::new("approval_expired", "This request was already resolved"))?;
        if now_ms() >= approval.expires_at_ms {
            return Err(Error::new(
                "approval_expired",
                "The request deadline passed",
            ));
        }
        sender
            .send(allow)
            .map_err(|_| Error::new("approval_expired", "The requesting client disconnected"))
    }
}

impl Admission {
    pub async fn approve(
        &self,
        project: &Project,
        tool: &str,
        arguments: &Value,
        timeout: Duration,
    ) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        {
            let mut active = self
                .session
                .active
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let operation = active
                .get_mut(&self.id)
                .ok_or_else(|| Error::new("cancelled", "Request was cancelled"))?;
            operation.approval = Some((
                Approval {
                    id: self.id,
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    project_root: project.root.clone(),
                    serial: project.serial.clone(),
                    tool: tool.into(),
                    arguments: arguments.clone(),
                    expires_at_ms: now_ms().saturating_add(timeout.as_millis() as u64),
                },
                sender,
            ));
        }
        match receiver.await {
            Ok(true) => Ok(()),
            Ok(false) => Err(Error::new(
                "approval_denied",
                "The operator declined this action",
            )),
            Err(_) => Err(Error::new(
                "approval_expired",
                "Approval is no longer available",
            )),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
