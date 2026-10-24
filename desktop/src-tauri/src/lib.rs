use andromeda::{
    actions::{Controller, Operation, RunOptions},
    capture,
    device::{Adb, DeviceSummary},
    errors::{Error, Result},
    managed::{BridgeStatus, ConnectionStatus, Host, ProjectSession, ProjectState},
    projects::{Client, ConfigChange, Project, Registry},
};
use base64::Engine;
use serde::Serialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

struct PreviewTask {
    generation: u64,
    cancel: CancellationToken,
}

struct AppState {
    registry: Registry,
    controller: Controller,
    companion: andromeda::companion::Companion,
    host: Host,
    changes: Mutex<BTreeMap<u64, ConfigChange>>,
    next_change: AtomicU64,
    settings: tokio::sync::Mutex<()>,
    preview: Mutex<Option<PreviewTask>>,
    next_preview: AtomicU64,
}

fn stop_preview_task(state: &AppState) {
    let mut preview = state
        .preview
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    state.next_preview.fetch_add(1, Ordering::SeqCst);
    if let Some(task) = preview.take() {
        task.cancel.cancel();
    }
}

#[derive(Serialize)]
struct Snapshot {
    projects: Vec<Project>,
    sessions: Vec<ProjectSession>,
    bridges: Vec<BridgeStatus>,
    pause_generation: u64,
    connections: Vec<ConnectionStatus>,
    tools: Vec<ToolSummary>,
}
#[derive(Serialize)]
struct ToolSummary {
    name: String,
    description: String,
    read_only: bool,
}

#[tauri::command]
async fn snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    Ok(Snapshot {
        projects: state.registry.list()?,
        sessions: state
            .registry
            .list()?
            .iter()
            .map(|project| state.host.project_session(&project.id))
            .collect(),
        bridges: state.host.bridge_statuses(),
        pause_generation: state.host.pause_generation(),
        connections: state.host.connections(),
        tools: andromeda::mcp::tools()
            .into_iter()
            .map(|tool| ToolSummary {
                name: tool.name.into_owned(),
                description: tool.description.unwrap_or_default().into_owned(),
                read_only: tool
                    .annotations
                    .is_some_and(|a| a.read_only_hint == Some(true)),
            })
            .collect(),
    })
}

#[tauri::command]
async fn choose_project(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<Project>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose a project for Andromeda")
        .pick_folder(move |path| {
            let _ = sender.send(path);
        });
    let Some(path) = receiver
        .await
        .map_err(|_| Error::new("dialog_closed", "The folder chooser closed"))?
    else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|_| Error::invalid("Choose a local project directory"))?;
    let _guard = state.settings.lock().await;
    state.registry.register(&path).map(Some)
}

#[tauri::command]
async fn save_project(
    state: State<'_, AppState>,
    id: String,
    serial: Option<String>,
    allowed_tools: Vec<String>,
    instruction_file: PathBuf,
    enabled: bool,
) -> Result<Project> {
    let _guard = state.settings.lock().await;
    require_paused(&state, &id)?;
    let previous_serial = state.registry.get(&id)?.serial;
    state.host.stop_project(&id);
    let saved = state
        .registry
        .update(&id, serial, allowed_tools, instruction_file, enabled)?;
    if saved.serial != previous_serial {
        stop_preview_task(state.inner());
    }
    Ok(saved)
}

fn require_paused(state: &AppState, id: &str) -> Result<()> {
    if state.host.project_session(id).state == ProjectState::Running {
        return Err(Error::new(
            "pause_required",
            "Pause this project before changing its access",
        ));
    }
    Ok(())
}

#[tauri::command]
async fn remove_project(state: State<'_, AppState>, id: String) -> Result<()> {
    let _guard = state.settings.lock().await;
    require_paused(&state, &id)?;
    state.registry.remove(&id)?;
    stop_preview_task(state.inner());
    Ok(())
}

#[tauri::command]
async fn devices(state: State<'_, AppState>) -> Result<Vec<DeviceSummary>> {
    let execution = state
        .controller
        .run(
            Operation::ListDevices,
            RunOptions::default(),
            CancellationToken::new(),
        )
        .await;
    serde_json::from_value(execution.result?.data["devices"].clone())
        .map_err(|_| Error::new("invalid_devices", "Could not read the connected devices"))
}

#[tauri::command]
async fn pair_companion(state: State<'_, AppState>, id: String) -> Result<()> {
    let _guard = state.settings.lock().await;
    require_paused(&state, &id)?;
    let project = state.registry.get(&id)?;
    let serial = project.serial.ok_or_else(|| {
        Error::new(
            "device_required",
            "Select and save this project's phone first",
        )
    })?;
    if !state.companion.is_paired(&serial)? || state.companion.status(&serial).await.is_err() {
        state.companion.pair(&serial).await?;
    }
    let _ = state.host.sync_phone_sessions().await;
    Ok(())
}

#[tauri::command]
async fn companion_status(state: State<'_, AppState>, id: String) -> Result<serde_json::Value> {
    let project = state.registry.get(&id)?;
    let serial = project.serial.ok_or_else(|| {
        Error::new(
            "device_required",
            "Select and save this project's phone first",
        )
    })?;
    state.companion.status(&serial).await
}

#[derive(Serialize)]
struct InstructionDocument {
    path: PathBuf,
    content: Option<String>,
    starter: &'static str,
}

#[tauri::command]
async fn instructions(state: State<'_, AppState>, id: String) -> Result<InstructionDocument> {
    let project = state.registry.get(&id)?;
    Ok(InstructionDocument {
        path: project.root.join(&project.instruction_file),
        content: state.registry.instructions(&id)?,
        starter: Registry::starter_instructions(),
    })
}

#[tauri::command]
async fn save_instructions(
    state: State<'_, AppState>,
    id: String,
    content: String,
    previous: Option<String>,
) -> Result<()> {
    let _guard = state.settings.lock().await;
    require_paused(&state, &id)?;
    state
        .registry
        .save_instructions(&id, &content, previous.as_deref())
}

#[derive(Serialize)]
struct ChangePreview {
    id: u64,
    path: PathBuf,
    replaces_existing: bool,
    before: String,
    after: String,
}

fn server_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "andromeda.exe"
    } else {
        "andromeda"
    };
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| Error::new("missing_server", e.to_string()))?
        .join("bin")
        .join(name);
    let path = if bundled.is_file() {
        bundled
    } else if cfg!(debug_assertions) {
        std::env::current_exe()?
            .parent()
            .ok_or_else(|| Error::new("missing_server", "Cannot locate the build directory"))?
            .join(name)
    } else {
        bundled
    };
    if !path.is_file() {
        return Err(Error::new(
            "missing_server",
            "The Andromeda MCP executable is missing from this installation",
        ));
    }
    Ok(path)
}

#[tauri::command]
async fn preview_connection(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    client: Client,
) -> Result<ChangePreview> {
    let change = state
        .registry
        .preview_client_config(&id, client, &server_path(&app)?)?;
    let id = state.next_change.fetch_add(1, Ordering::Relaxed);
    let preview = ChangePreview {
        id,
        path: change.path.clone(),
        replaces_existing: change.replaces_existing,
        before: change.before.clone(),
        after: change.after.clone(),
    };
    let mut changes = state.changes.lock().unwrap_or_else(|e| e.into_inner());
    while changes.len() >= 8 {
        changes.pop_first();
    }
    changes.insert(id, change);
    Ok(preview)
}

#[tauri::command]
async fn apply_connection(
    state: State<'_, AppState>,
    id: u64,
    allow_replacement: bool,
) -> Result<()> {
    let _guard = state.settings.lock().await;
    let change = state
        .changes
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&id)
        .ok_or_else(|| Error::new("preview_expired", "Open a fresh connection preview"))?;
    state
        .registry
        .apply_client_config(change, allow_replacement)
}

#[tauri::command]
async fn emergency_stop(state: State<'_, AppState>) -> Result<usize> {
    stop_preview_task(state.inner());
    let count = state.host.stop_all();
    let _ = state.host.sync_phone_sessions().await;
    Ok(count)
}

#[tauri::command]
async fn project_session(
    state: State<'_, AppState>,
    id: String,
    action: ProjectState,
    generation: u64,
    global_generation: u64,
) -> Result<()> {
    if action == ProjectState::Running {
        let _guard = state.settings.lock().await;
        state
            .host
            .set_project_state(&id, action, generation, global_generation)?;
    } else {
        // Pause/Stop must not wait behind a slow setup or pairing request.
        state
            .host
            .set_project_state(&id, action, generation, global_generation)?;
    }
    let _ = state.host.sync_phone_sessions().await;
    Ok(())
}

#[derive(Serialize)]
struct PreviewImage {
    data_url: String,
    elapsed_ms: f64,
    serial: String,
}

#[tauri::command]
async fn capture_preview(state: State<'_, AppState>, id: String) -> Result<PreviewImage> {
    let project = state.registry.get(&id)?;
    let serial = project
        .serial
        .ok_or_else(|| Error::new("device_required", "Select and save a device first"))?;
    let execution = state
        .controller
        .run(
            Operation::View(capture::Options {
                max_width: Some(720),
                max_height: Some(1280),
                format: capture::Format::Jpeg,
                quality: Some(80),
            }),
            RunOptions {
                serial: Some(serial.clone()),
                timeout_ms: Some(15000),
            },
            CancellationToken::new(),
        )
        .await;
    let image = execution
        .result?
        .image
        .ok_or_else(|| Error::new("missing_image", "Android returned no screenshot"))?;
    Ok(PreviewImage {
        serial,
        data_url: format!(
            "data:{};base64,{}",
            image.mime_type,
            base64::engine::general_purpose::STANDARD.encode(image.bytes)
        ),
        elapsed_ms: execution.elapsed_ms,
    })
}

#[derive(Clone, Serialize)]
struct PreviewFrame {
    generation: String,
    serial: String,
    data_url: String,
    elapsed_ms: f64,
    encode_ms: f64,
    sequence: u64,
    width: u32,
    height: u32,
}

async fn preview_loop<F>(
    controller: Controller,
    serial: String,
    cancel: CancellationToken,
    generation: String,
    mut emit: F,
) -> Result<()>
where
    F: FnMut(PreviewFrame),
{
    let mut stream = tokio::select! {
        _ = cancel.cancelled() => return Ok(()),
        result = controller.stream(Some(serial.clone()), 100) => result?,
    };
    let mut sequence = 0;
    let mut previous = Instant::now();
    loop {
        let frame = tokio::select! {
            _ = cancel.cancelled() => return Ok(()),
            result = stream.next_frame(Duration::from_secs(10)) => match result? {
                Some(frame) => frame,
                None => return Err(Error::new("stream_ended", "The phone stopped the capture stream")),
            },
        };
        let received = Instant::now();
        let elapsed_ms = received.duration_since(previous).as_secs_f64() * 1000.0;
        previous = received;
        let started = Instant::now();
        let image = tokio::task::spawn_blocking(move || {
            capture::encode(
                frame,
                capture::Options {
                    max_width: Some(720),
                    max_height: Some(1280),
                    format: capture::Format::Jpeg,
                    quality: Some(80),
                },
            )
        })
        .await
        .map_err(|error| Error::new("image_encoding", error.to_string()))??;
        let encode_ms = started.elapsed().as_secs_f64() * 1000.0;
        if cancel.is_cancelled() {
            return Ok(());
        }
        sequence += 1;
        emit(PreviewFrame {
            generation: generation.clone(),
            serial: serial.clone(),
            data_url: format!(
                "data:{};base64,{}",
                image.mime_type,
                base64::engine::general_purpose::STANDARD.encode(image.bytes)
            ),
            elapsed_ms,
            encode_ms,
            sequence,
            width: image.width,
            height: image.height,
        });
    }
}

#[tauri::command]
async fn start_preview(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    generation: String,
) -> Result<()> {
    let requested_generation = state.next_preview.load(Ordering::SeqCst);
    if generation.is_empty()
        || generation.len() > 64
        || !generation
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(Error::invalid(
            "Preview generation must be a bounded request identifier",
        ));
    }
    let project = state.registry.get(&id)?;
    let serial = project
        .serial
        .ok_or_else(|| Error::new("device_required", "Select and save a device first"))?;
    let cancel = CancellationToken::new();
    let task_generation;
    {
        let mut preview = state
            .preview
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if state.next_preview.load(Ordering::SeqCst) != requested_generation {
            return Err(Error::new(
                "cancelled",
                "Preview startup was superseded by a stop or a newer request",
            ));
        }
        if let Some(previous) = preview.take() {
            previous.cancel.cancel();
        }
        task_generation = state.next_preview.fetch_add(1, Ordering::SeqCst);
        *preview = Some(PreviewTask {
            generation: task_generation,
            cancel: cancel.clone(),
        });
    }
    let controller = state.controller.clone();
    let app_handle = app.clone();
    let emit_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let frame_cancel = cancel.clone();
        let result = preview_loop(
            controller,
            serial.clone(),
            cancel.clone(),
            generation.clone(),
            move |frame| {
                if let Some(state) = emit_handle.try_state::<AppState>() {
                    let current = state
                        .preview
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if !frame_cancel.is_cancelled()
                        && current
                            .as_ref()
                            .is_some_and(|task| task.generation == task_generation)
                    {
                        let _ = emit_handle.emit("preview-frame", frame);
                    }
                }
            },
        )
        .await;
        if let Some(state) = app_handle.try_state::<AppState>() {
            let mut preview = state
                .preview
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if preview
                .as_ref()
                .is_some_and(|task| task.generation == task_generation)
            {
                if !cancel.is_cancelled()
                    && let Err(error) = result
                {
                    let _ = app_handle.emit("preview-error", json!({"generation":generation,"serial": serial, "code": error.code, "message": error.message}));
                }
                *preview = None;
            }
        }
    });
    Ok(())
}

#[tauri::command]
fn stop_preview(state: State<'_, AppState>) {
    stop_preview_task(state.inner());
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let directory = std::env::var_os("ANDROMEDA_STATE_DIR")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(Registry::default_directory)?;
            let registry = Registry::open(directory.clone())?;
            let store = andromeda::companion::Store::open(directory)?;
            let adb = Adb::new("127.0.0.1:5037".parse()?);
            let companion = andromeda::companion::Companion::new(adb.clone(), store.clone());
            let controller = Controller::new(adb, None, true).with_companion(store);
            let host =
                tauri::async_runtime::block_on(Host::start(registry.clone(), controller.clone()))?;
            app.manage(AppState {
                registry,
                controller,
                companion,
                host,
                changes: Mutex::default(),
                next_change: AtomicU64::new(1),
                settings: tokio::sync::Mutex::new(()),
                preview: Mutex::default(),
                next_preview: AtomicU64::new(1),
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(
                event,
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
            ) && let Some(state) = window.try_state::<AppState>()
            {
                state.host.shutdown();
                stop_preview_task(state.inner());
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            choose_project,
            save_project,
            remove_project,
            devices,
            pair_companion,
            companion_status,
            instructions,
            save_instructions,
            preview_connection,
            apply_connection,
            emergency_stop,
            project_session,
            capture_preview,
            start_preview,
            stop_preview
        ])
        .run(tauri::generate_context!())
        .expect("Andromeda could not start");
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    use std::time::Duration;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn request(socket: &mut tokio::net::TcpStream) -> String {
        let mut prefix = [0; 4];
        socket.read_exact(&mut prefix).await.unwrap();
        let length = usize::from_str_radix(std::str::from_utf8(&prefix).unwrap(), 16).unwrap();
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        String::from_utf8(body).unwrap()
    }

    fn png() -> Vec<u8> {
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(
                &[
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                ],
                2,
                2,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        bytes
    }

    #[tokio::test]
    async fn preview_loop_encodes_a_stream_frame() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let frame = png();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert_eq!(request(&mut socket).await, "host:transport:phone-A");
            socket.write_all(b"OKAY").await.unwrap();
            let service = request(&mut socket).await;
            assert!(service.starts_with("shell,v2,raw:while true; do screencap -p"));
            socket.write_all(b"OKAY").await.unwrap();
            socket.write_u8(1).await.unwrap();
            socket.write_u32_le(frame.len() as u32).await.unwrap();
            socket.write_all(&frame).await.unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
        });
        let controller = Controller::new(Adb::new(address), None, false);
        let cancel = CancellationToken::new();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(preview_loop(
            controller,
            "phone-A".into(),
            cancel.clone(),
            "test-preview".into(),
            move |frame| {
                let _ = sender.send(frame);
            },
        ));
        let frame = tokio::time::timeout(Duration::from_secs(2), receiver.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(frame.serial, "phone-A");
        assert_eq!(frame.sequence, 1);
        assert_eq!(frame.generation, "test-preview");
        assert!(frame.width > 0 && frame.height > 0);
        assert!(frame.data_url.starts_with("data:image/jpeg;base64,"));
        cancel.cancel();
        assert!(task.await.unwrap().is_ok());
        server.abort();
    }
}
