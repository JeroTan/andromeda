use andromeda::{
    actions::{Controller, Operation, RunOptions},
    capture,
    device::{Adb, DeviceSummary},
    errors::{Error, Result},
    managed::{Approval, ConnectionStatus, Host},
    projects::{Client, ConfigChange, Project, Registry},
};
use base64::Engine;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio_util::sync::CancellationToken;

struct AppState {
    registry: Registry,
    controller: Controller,
    companion: andromeda::companion::Companion,
    host: Host,
    changes: Mutex<BTreeMap<u64, ConfigChange>>,
    next_change: AtomicU64,
    settings: tokio::sync::Mutex<()>,
}

#[derive(Serialize)]
struct Snapshot {
    projects: Vec<Project>,
    paused: bool,
    pause_generation: u64,
    approvals: Vec<Approval>,
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
        paused: state.host.is_paused(),
        pause_generation: state.host.pause_generation(),
        approvals: state.host.approvals(),
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
    require_paused(&state)?;
    state
        .registry
        .update(&id, serial, allowed_tools, instruction_file, enabled)
}

fn require_paused(state: &AppState) -> Result<()> {
    if !state.host.is_paused() {
        return Err(Error::new(
            "pause_required",
            "Pause the session before changing project access",
        ));
    }
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
    require_paused(&state)?;
    let project = state.registry.get(&id)?;
    let serial = project.serial.ok_or_else(|| {
        Error::new(
            "device_required",
            "Select and save this project's phone first",
        )
    })?;
    state.companion.pair(&serial).await
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
    require_paused(&state)?;
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
fn pause(state: State<'_, AppState>) -> usize {
    state.host.stop_all()
}

#[tauri::command]
async fn resume(state: State<'_, AppState>, generation: u64) -> Result<()> {
    let _guard = state.settings.lock().await;
    state.host.resume(generation)
}

#[tauri::command]
fn decide(state: State<'_, AppState>, id: u64, allow: bool) -> Result<()> {
    state.host.decide(id, allow)
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
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            choose_project,
            save_project,
            devices,
            pair_companion,
            companion_status,
            instructions,
            save_instructions,
            preview_connection,
            apply_connection,
            pause,
            resume,
            decide,
            capture_preview
        ])
        .run(tauri::generate_context!())
        .expect("Andromeda could not start");
}
