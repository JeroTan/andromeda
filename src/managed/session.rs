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
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Serialize)]
pub struct Approval {
    pub id: u64,
    pub project_id: String,
    pub project_name: String,
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
