use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    Running,
    Paused,
    #[default]
    Stopped,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectSession {
    pub project_id: String,
    pub state: ProjectState,
    pub generation: u64,
}
impl ProjectSession {
    fn stopped(id: &str) -> Self {
        Self {
            project_id: id.into(),
            state: ProjectState::Stopped,
            generation: 0,
        }
    }
}
struct Active {
    project: String,
    cancel: CancellationToken,
}
#[derive(Default)]
struct Control {
    projects: BTreeMap<String, ProjectSession>,
    active: BTreeMap<u64, Active>,
}
#[derive(Default)]
pub(super) struct Session {
    next: AtomicU64,
    control: Mutex<Control>,
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
            .control
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active
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

    pub fn project(&self, project: &str) -> ProjectSession {
        self.control
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .get(project)
            .cloned()
            .unwrap_or_else(|| ProjectSession::stopped(project))
    }
    pub fn projects(&self) -> Vec<ProjectSession> {
        self.control
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projects
            .values()
            .cloned()
            .collect()
    }
    pub fn transition(
        &self,
        project: &str,
        state: ProjectState,
        expected: Option<u64>,
    ) -> Result<ProjectSession> {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        let current = control
            .projects
            .entry(project.into())
            .or_insert_with(|| ProjectSession::stopped(project));
        if expected.is_some_and(|generation| generation != current.generation) {
            return Err(Error::new(
                "session_changed",
                "Project session changed; use its current controls",
            ));
        }
        current.state = state;
        current.generation = current.generation.wrapping_add(1);
        let result = current.clone();
        if state != ProjectState::Running {
            for active in control
                .active
                .values()
                .filter(|active| active.project == project)
            {
                active.cancel.cancel();
            }
        }
        Ok(result)
    }
    pub fn admit(
        self: &Arc<Self>,
        project: &str,
        input: bool,
        cancel: CancellationToken,
    ) -> Result<Admission> {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        if input {
            match control
                .projects
                .get(project)
                .map(|p| p.state)
                .unwrap_or_default()
            {
                ProjectState::Running => {}
                ProjectState::Paused => {
                    return Err(Error::new(
                        "project_paused",
                        "This project is paused. Resume this project in Andromeda on the desktop or phone",
                    ));
                }
                ProjectState::Stopped => {
                    return Err(Error::new(
                        "project_stopped",
                        "This project is stopped. Start this project in Andromeda on the desktop",
                    ));
                }
            }
        }
        if control.active.len() >= 32 {
            return Err(Error::new(
                "busy",
                "At most 32 managed operations may be active",
            ));
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        control.active.insert(
            id,
            Active {
                project: project.into(),
                cancel,
            },
        );
        Ok(Admission {
            session: self.clone(),
            id,
        })
    }
    pub fn stop_all(&self) -> usize {
        let mut control = self.control.lock().unwrap_or_else(|e| e.into_inner());
        for project in control.projects.values_mut() {
            project.state = ProjectState::Stopped;
            project.generation = project.generation.wrapping_add(1);
        }
        for operation in control.active.values() {
            operation.cancel.cancel();
        }
        control.active.len()
    }
    pub fn stop(&self, project: &str) -> usize {
        let count = self
            .control
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active
            .values()
            .filter(|a| a.project == project)
            .count();
        let _ = self.transition(project, ProjectState::Stopped, None);
        count
    }
}
