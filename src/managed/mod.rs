//! Authenticated local gateway owned by the desktop application.
use crate::{
    actions::Controller,
    errors::{Error, Result},
    mcp::{Server, tools},
    projects::{Registry, atomic_write, random_hex, read_bounded},
};
use fs2::FileExt;
use rmcp::{
    ServerHandler,
    model::{CallToolResult, ServerConfig, Tool},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
mod proxy;
mod session;
mod wire;
pub use proxy::Proxy;
pub use session::{Approval, ConnectionStatus};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    version: u8,
    address: SocketAddr,
    key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    project_id: String,
    credential: String,
    working_directory: PathBuf,
    command: Command,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Tools,
    Info,
    Connection { id: String, connected: bool },
    Call { name: String, arguments: Value },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
enum Reply {
    Success(Value),
    Error(Error),
}

pub struct Host {
    cancel: CancellationToken,
    controller: Controller,
    session: Arc<session::Session>,
    pause_generation: Mutex<u64>,
    task: JoinHandle<()>,
    endpoint_path: PathBuf,
    key: String,
    _lock: File,
}

impl Host {
    pub async fn start(registry: Registry, controller: Controller) -> Result<Self> {
        controller.pause();
        let path = registry.directory().join("host.lock");
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::new(
                "config_error",
                "Host lock cannot be a symbolic link",
            ));
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(path)?;
        lock.try_lock_exclusive().map_err(|_| {
            Error::new(
                "desktop_already_running",
                "An Andromeda desktop host is already running",
            )
        })?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let key = random_hex(32)?;
        let endpoint_path = registry.directory().join("gateway.json");
        let endpoint = Endpoint {
            version: 1,
            address: listener.local_addr()?,
            key: key.clone(),
        };
        atomic_write(
            &endpoint_path,
            &serde_json::to_vec(&endpoint)
                .map_err(|_| Error::new("config_error", "Cannot write gateway metadata"))?,
        )?;
        let cancel = CancellationToken::new();
        let shutdown = cancel.clone();
        let worker = controller.clone();
        let secret = key.clone();
        let session = Arc::new(session::Session::default());
        let requests = session.clone();
        let task = tokio::spawn(async move {
            // Only 32 calls can be admitted; leave capacity for stop and setup requests.
            let slots = Arc::new(tokio::sync::Semaphore::new(64));
            let mut tasks = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    biased;
                    _=shutdown.cancelled()=>break,
                    Some(_)=tasks.join_next(),if !tasks.is_empty()=>{},
                    connection=listener.accept()=>{
                        let Ok((socket,_))=connection else{break};
                        let Ok(permit)=slots.clone().try_acquire_owned() else{continue};
                        let (registry,controller,key,cancel)=(registry.clone(),worker.clone(),secret.clone(),shutdown.child_token());
                        let session = requests.clone();
                        tasks.spawn(async move{let _permit=permit;let _=serve(socket,registry,controller,key,cancel,session).await;});
                    }
                }
            }
            worker.stop();
            tasks.abort_all();
        });
        Ok(Self {
            cancel,
            controller,
            session,
            pause_generation: Mutex::new(0),
            task,
            endpoint_path,
            key,
            _lock: lock,
        })
    }
    pub fn stop_all(&self) -> usize {
        let mut generation = self
            .pause_generation
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *generation = generation.wrapping_add(1);
        self.controller.pause();
        self.session.stop(None)
    }
    pub fn pause_generation(&self) -> u64 {
        *self
            .pause_generation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
    pub fn resume(&self, expected_generation: u64) -> Result<()> {
        let generation = self
            .pause_generation
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if *generation != expected_generation || self.cancel.is_cancelled() {
            return Err(Error::new(
                "session_changed",
                "The session was stopped after this Resume request; review its current state",
            ));
        }
        self.controller.resume();
        Ok(())
    }
    pub fn is_paused(&self) -> bool {
        self.controller.is_paused()
    }
    pub fn approvals(&self) -> Vec<Approval> {
        self.session.approvals()
    }
    pub fn connections(&self) -> Vec<ConnectionStatus> {
        self.session.connections()
    }
    pub fn decide(&self, id: u64, allow: bool) -> Result<()> {
        if allow && self.is_paused() {
            return Err(Error::new(
                "control_paused",
                "Resume the session before approving input",
            ));
        }
        self.session.decide(id, allow)
    }

    pub fn shutdown(&self) {
        self.cancel.cancel();
        self.stop_all();
        self.task.abort();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.stop_all();
        self.task.abort();
        let ours = read_bounded(&self.endpoint_path, 4096)
            .ok()
            .and_then(|b| serde_json::from_slice::<Endpoint>(&b).ok())
            .is_some_and(|e| e.key == self.key);
        if ours {
            let _ = fs::remove_file(&self.endpoint_path);
        }
    }
}

async fn serve(
    mut socket: TcpStream,
    registry: Registry,
    controller: Controller,
    key: String,
    cancel: CancellationToken,
    session: Arc<session::Session>,
) -> Result<()> {
    socket.set_nodelay(true)?;
    let request = tokio::time::timeout(Duration::from_secs(5), async {
        let hello: wire::Hello = wire::read(&mut socket, 1024).await?;
        let nonce = random_hex(32)?;
        let proof = wire::proof(&key, "server", &hello.nonce, &nonce, &())?;
        wire::write(
            &mut socket,
            &wire::Challenge {
                nonce: nonce.clone(),
                proof,
            },
            1024,
        )
        .await?;
        let signed: wire::Signed<Request> = wire::read(&mut socket, wire::REQUEST_LIMIT).await?;
        wire::verify(
            &key,
            "client",
            &hello.nonce,
            &nonce,
            &signed.value,
            &signed.proof,
        )?;
        Ok::<_, Error>(signed.value)
    })
    .await
    .map_err(|_| Error::new("timeout", "Gateway authentication deadline exceeded"))??;
    let (mut reader, mut writer) = socket.into_split();
    let operation_cancel = cancel.child_token();
    let response = tokio::select! {
        biased;
        _=cancel.cancelled()=>return Err(Error::new("cancelled","Desktop host is closing")),
        _=reader.read_u8()=>{operation_cancel.cancel();return Err(Error::new("cancelled","MCP client disconnected"));},
        result=dispatch(&registry,&controller,request,operation_cancel.clone(),session)=>match result {Ok(value)=>Reply::Success(value),Err(error)=>Reply::Error(error)},
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        wire::write(&mut writer, &response, wire::RESPONSE_LIMIT),
    )
    .await
    .map_err(|_| Error::new("timeout", "Gateway response deadline exceeded"))?
}

async fn dispatch(
    registry: &Registry,
    controller: &Controller,
    request: Request,
    cancel: CancellationToken,
    session: Arc<session::Session>,
) -> Result<Value> {
    let project = registry.authorize(&request.project_id, &request.working_directory)?;
    // Compare a MAC keyed by the credential, avoiding a variable-time secret comparison.
    let credential = registry.credential(&request.project_id)?;
    let nonce = "0".repeat(64);
    let expected = wire::proof(&credential, "project", &nonce, &nonce, &request.project_id)?;
    wire::verify(
        &request.credential,
        "project",
        &nonce,
        &nonce,
        &request.project_id,
        &expected,
    )?;
    let catalog: Vec<Tool> = tools()
        .into_iter()
        .filter(|tool| {
            project
                .allowed_tools
                .iter()
                .any(|name| name == tool.name.as_ref())
        })
        .collect();
    let allow_control = catalog.iter().any(|tool| {
        tool.name != "adm_stop"
            && tool
                .annotations
                .as_ref()
                .is_some_and(|a| a.read_only_hint != Some(true))
    });
    let server = Server {
        controller: controller.scoped(&project.id, project.serial.clone(), allow_control),
    };
    match request.command {
        Command::Connection { id, connected } => {
            session.heartbeat(&project.id, &id, connected)?;
            Ok(json!({"connected":connected}))
        }
        Command::Tools => encode(catalog),
        Command::Info => {
            let mut info = server.get_info();
            let instructions = registry
                .instructions(&project.id)
                .inspect_err(|error| session.initialization_error(&project.id, error))?;
            if let Some(instructions) = instructions {
                let base = info.instructions.unwrap_or_default();
                info.instructions = Some(format!(
                    "{base}\n\nInstructions selected for this project:\n{instructions}"
                ));
            }
            encode(info)
        }
        Command::Call {
            name,
            mut arguments,
        } => {
            if !project.allowed_tools.contains(&name) {
                return Err(Error::new(
                    "tool_not_allowed",
                    "This tool is not enabled for this project",
                ));
            }
            if name != "adm_stop" && name != "adm_list_devices" {
                let serial = project.serial.clone().ok_or_else(|| {
                    Error::new(
                        "device_required",
                        "Select this project's device in the desktop app",
                    )
                })?;
                let object = arguments
                    .as_object_mut()
                    .ok_or_else(|| Error::invalid("Tool arguments must be an object"))?;
                if object
                    .get("serial")
                    .is_some_and(|value| value.as_str() != Some(&serial))
                {
                    return Err(Error::new(
                        "device_not_allowed",
                        "This project is registered for a different device",
                    ));
                }
                object.insert("serial".into(), Value::String(serial));
            }
            if name == "adm_stop" {
                if !arguments
                    .as_object()
                    .is_some_and(|object| object.is_empty())
                {
                    return Err(Error::invalid("adm_stop takes an empty object"));
                }
                let count = session.stop(Some(&project.id));
                server.controller.stop();
                return encode(CallToolResult::structured(
                    json!({"success":true,"elapsed_ms":0.0,"data":{"cancelled_operations":count,"note":"Queued work and approval requests are cancelled. Input already accepted by Android may finish."}}),
                ));
            }
            crate::mcp::validate(&name, &arguments)?;
            let is_input = catalog
                .iter()
                .find(|tool| tool.name == name)
                .is_some_and(|tool| {
                    tool.annotations
                        .as_ref()
                        .is_some_and(|a| a.read_only_hint != Some(true))
                });
            if is_input && controller.is_paused() {
                return Err(Error::new(
                    "control_paused",
                    "Resume the session in the desktop app before requesting input",
                ));
            }
            let timeout_ms = arguments
                .get("timeout_ms")
                .and_then(Value::as_u64)
                .unwrap_or(15000);
            crate::input::duration(timeout_ms, 1, 120000)?;
            let timeout = Duration::from_millis(timeout_ms);
            let started = Instant::now();
            let admission = session.admit(&project.id, cancel.clone())?;
            let execute = async {
                if is_input {
                    if controller.is_paused() {
                        return Err(Error::new("control_paused", "Device input is paused"));
                    }
                    admission
                        .approve(&project, &name, &arguments, timeout)
                        .await?;
                    // Recheck policy after human review: a changed device or allowlist invalidates approval.
                    let current = registry.authorize(&project.id, &request.working_directory)?;
                    if current.serial != project.serial
                        || current.allowed_tools != project.allowed_tools
                    {
                        return Err(Error::new(
                            "policy_changed",
                            "Project access changed; request the action again",
                        ));
                    }
                }
                let remaining = timeout.saturating_sub(started.elapsed()).as_millis().max(1) as u64;
                arguments["timeout_ms"] = json!(remaining);
                encode(server.call(&name, arguments, cancel.clone()).await)
            };
            tokio::select! {
                biased;
                _ = cancel.cancelled() => Err(Error::new("cancelled", "Managed request cancelled; observe before retrying input")),
                result = tokio::time::timeout(timeout, execute) => result.map_err(|_| Error::new("timeout", "Managed request deadline expired, including approval time; observe before retrying input"))?,
            }
        }
    }
}

fn encode<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value)
        .map_err(|_| Error::new("managed_protocol", "Cannot encode the managed response"))
}

#[derive(Clone)]
pub struct Client {
    registry: Registry,
    project_id: String,
    working_directory: PathBuf,
}

impl Client {
    async fn track_connection(&self) -> Result<ConnectionLease> {
        let id = random_hex(16)?;
        let cancel = CancellationToken::new();
        self.rpc(
            Command::Connection {
                id: id.clone(),
                connected: true,
            },
            cancel.clone(),
        )
        .await?;
        let client = self.clone();
        let connection = id.clone();
        let stop = cancel.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = stop.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(4)) => {}
                }
                let _ = client
                    .rpc(
                        Command::Connection {
                            id: connection.clone(),
                            connected: true,
                        },
                        stop.clone(),
                    )
                    .await;
            }
            // A clean shutdown removes the lease immediately. A crashed process expires after 15s.
            let _ = client
                .rpc(
                    Command::Connection {
                        id: connection,
                        connected: false,
                    },
                    CancellationToken::new(),
                )
                .await;
        });
        Ok(ConnectionLease { cancel })
    }
    pub fn new(registry: Registry, project_id: String, working_directory: PathBuf) -> Result<Self> {
        registry.authorize(&project_id, &working_directory)?;
        Ok(Self {
            registry,
            project_id,
            working_directory,
        })
    }
    pub async fn tools(&self) -> Result<Vec<Tool>> {
        decode(self.rpc(Command::Tools, CancellationToken::new()).await?)
    }
    pub async fn info(&self) -> Result<ServerConfig> {
        decode(self.rpc(Command::Info, CancellationToken::new()).await?)
    }
    pub async fn call(
        &self,
        name: String,
        arguments: Value,
        cancel: CancellationToken,
    ) -> CallToolResult {
        let started = Instant::now();
        match self
            .rpc(Command::Call { name, arguments }, cancel)
            .await
            .and_then(decode::<CallToolResult>)
        {
            Ok(mut result) => {
                if let Some(content) = &mut result.structured_content {
                    content["elapsed_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
                    if let Some(text) = result.content.first_mut() {
                        *text = rmcp::model::ContentBlock::text(content.to_string());
                    }
                }
                result
            }
            Err(error) => CallToolResult::structured_error(
                json!({"success":false,"error":error,"elapsed_ms":started.elapsed().as_secs_f64() * 1000.0}),
            ),
        }
    }
    async fn rpc(&self, command: Command, cancel: CancellationToken) -> Result<Value> {
        self.registry
            .authorize(&self.project_id, &self.working_directory)?;
        let path = self.registry.directory().join("gateway.json");
        if !path.exists() {
            return Err(Error::new(
                "desktop_offline",
                "Open the Andromeda desktop app to use this project's MCP connection",
            ));
        }
        let endpoint: Endpoint = serde_json::from_slice(&read_bounded(&path, 4096)?)
            .map_err(|_| Error::new("managed_protocol", "Invalid desktop gateway metadata"))?;
        if endpoint.version != 1 || !endpoint.address.ip().is_loopback() {
            return Err(wire::auth_error());
        }
        let timeout = match &command {
            Command::Call { arguments, .. } => {
                arguments
                    .get("timeout_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(15000)
                    .min(120000)
                    + 5000
            }
            _ => 10000,
        };
        let operation = async {
            let mut socket = TcpStream::connect(endpoint.address).await.map_err(|_| {
                Error::new(
                    "desktop_offline",
                    "The Andromeda desktop gateway is unavailable; open the app",
                )
            })?;
            socket.set_nodelay(true)?;
            let nonce = random_hex(32)?;
            wire::write(
                &mut socket,
                &wire::Hello {
                    nonce: nonce.clone(),
                },
                1024,
            )
            .await?;
            let challenge: wire::Challenge = wire::read(&mut socket, 1024).await?;
            wire::verify(
                &endpoint.key,
                "server",
                &nonce,
                &challenge.nonce,
                &(),
                &challenge.proof,
            )?;
            let request = Request {
                project_id: self.project_id.clone(),
                credential: self.registry.credential(&self.project_id)?,
                working_directory: self.working_directory.clone(),
                command,
            };
            let proof = wire::proof(&endpoint.key, "client", &nonce, &challenge.nonce, &request)?;
            wire::write(
                &mut socket,
                &wire::Signed {
                    value: request,
                    proof,
                },
                wire::REQUEST_LIMIT,
            )
            .await?;
            match wire::read(&mut socket, wire::RESPONSE_LIMIT).await? {
                Reply::Success(value) => Ok(value),
                Reply::Error(error) => Err(error),
            }
        };
        tokio::select! {
            biased;
            _=cancel.cancelled()=>Err(Error::new("cancelled","MCP request cancelled; observe before retrying input")),
            result=tokio::time::timeout(Duration::from_millis(timeout),operation)=>result.map_err(|_|Error::new("timeout","Desktop request timed out; observe before retrying input"))?,
        }
    }
}
struct ConnectionLease {
    cancel: CancellationToken,
}
impl Drop for ConnectionLease {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T> {
    serde_json::from_value(value)
        .map_err(|_| Error::new("managed_protocol", "Invalid desktop response"))
}
