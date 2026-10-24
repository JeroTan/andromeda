use andromeda::{
    actions::Controller,
    device::Adb,
    mcp::{Server, tools},
};
use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use std::{net::SocketAddr, path::PathBuf, process::ExitCode};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(
    version,
    about = "Local Android observation and control for MCP agents"
)]
struct Cli {
    #[arg(long, env = "ANDROMEDA_ADB_SERVER", default_value = "127.0.0.1:5037")]
    adb_server: SocketAddr,
    #[arg(long, env = "ANDROMEDA_DEVICE")]
    device: Option<String>,
    /// Authorize ordinary device input for this session. Ask the human before sensitive actions.
    #[arg(long)]
    allow_control: bool,
    /// Screenshot acquisition: auto prefers a paired bridge, adb forces the baseline.
    #[arg(long, value_enum, default_value_t = andromeda::actions::CaptureBackend::Auto)]
    capture_backend: andromeda::actions::CaptureBackend,
    /// Private application state used by the desktop app and managed connections.
    #[arg(long, env = "ANDROMEDA_STATE_DIR")]
    state_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Pair the selected phone while its user-opened pairing window is active.
    Pair,
    /// Read optional companion pairing/session state for the selected phone.
    CompanionStatus,
    /// Stop the phone companion and remove this computer's saved credential.
    Unpair,
    /// Run a project-scoped MCP connection managed by the desktop app.
    Managed {
        #[arg(long)]
        project: String,
    },
    /// Run the MCP stdio server (also the default).
    Serve,
    /// Print the complete tool catalog and JSON input schemas.
    Tools,
    /// List devices as a structured tool result.
    Devices,
    /// Execute one tool locally; intended for diagnostics and reproducible scripts.
    Call {
        tool: String,
        #[arg(long, default_value = "{}")]
        args: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("andromeda: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    // Dependencies may log complete MCP payloads at debug level. Only reviewed
    // audit events may reach stderr, even when the operator sets RUST_LOG=trace.
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false)
                .json()
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    metadata.target() == "andromeda::audit"
                })),
        )
        .init();
    if !cli.adb_server.ip().is_loopback() {
        return Err("ADB server must use a loopback address".into());
    }
    let direct_device_override = cli.device.is_some();
    let directory = match cli.state_dir.clone() {
        Some(path) => path,
        None => andromeda::projects::Registry::default_directory()?,
    };
    let store = andromeda::companion::Store::open(directory)?;
    let companion = andromeda::companion::Companion::new(Adb::new(cli.adb_server), store.clone());
    let controller = Controller::new(
        Adb::new(cli.adb_server),
        cli.device.clone(),
        cli.allow_control,
    )
    .with_companion(store.clone())
    .with_capture_backend(cli.capture_backend);
    let server = Server {
        controller: controller.clone(),
    };
    let result = match cli.command.unwrap_or(Command::Serve) {
        Command::Pair => {
            let serial = cli
                .device
                .as_deref()
                .ok_or("Choose --device explicitly when pairing")?;
            companion.pair(serial).await?;
            println!(
                "{{\"success\":true,\"paired\":true,\"next\":\"Start your desktop project, or use Connection controls for a standalone session\"}}"
            );
            return Ok(ExitCode::SUCCESS);
        }
        Command::CompanionStatus => {
            let serial = cli.device.as_deref().ok_or("Choose --device explicitly")?;
            println!(
                "{}",
                serde_json::json!({"success":true,"data":companion.status(serial).await?})
            );
            return Ok(ExitCode::SUCCESS);
        }
        Command::Unpair => {
            let serial = cli.device.as_deref().ok_or("Choose --device explicitly")?;
            let stopped = companion.stop(serial).await.is_ok();
            store.forget(serial)?;
            println!(
                "{}",
                serde_json::json!({"success":true,"saved_pairing_removed":true,"phone_session_stopped":stopped,"note":"Use Forget this computer on the phone to revoke its pairing credential"})
            );
            return Ok(ExitCode::SUCCESS);
        }
        Command::Tools => {
            println!("{}", serde_json::to_string_pretty(&tools())?);
            return Ok(ExitCode::SUCCESS);
        }
        Command::Devices => {
            server
                .call(
                    "adm_list_devices",
                    serde_json::json!({}),
                    CancellationToken::new(),
                )
                .await
        }
        Command::Call { tool, args } => {
            server
                .call(
                    &tool,
                    serde_json::from_str(&args)?,
                    CancellationToken::new(),
                )
                .await
        }
        Command::Serve => {
            let service = server.serve(rmcp::transport::stdio()).await?;
            let shutdown = service.cancellation_token();
            tokio::select! {result=service.waiting()=>{result?;},_=tokio::signal::ctrl_c()=>{controller.stop();shutdown.cancel();}}
            controller.stop();
            return Ok(ExitCode::SUCCESS);
        }
        Command::Managed { project } => {
            if cli.allow_control
                || direct_device_override
                || !matches!(
                    cli.capture_backend,
                    andromeda::actions::CaptureBackend::Auto
                )
                || cli.adb_server != "127.0.0.1:5037".parse::<SocketAddr>()?
            {
                return Err("Managed devices and control are configured in the desktop app".into());
            }
            let directory = match cli.state_dir {
                Some(path) => path,
                None => andromeda::projects::Registry::default_directory()?,
            };
            let registry = andromeda::projects::Registry::open(directory)?;
            let client =
                andromeda::managed::Client::new(registry, project, std::env::current_dir()?)?;
            let proxy = andromeda::managed::Proxy::connect(client).await?;
            let service = proxy.serve(rmcp::transport::stdio()).await?;
            let shutdown = service.cancellation_token();
            tokio::select! {result=service.waiting()=>{result?;},_=tokio::signal::ctrl_c()=>shutdown.cancel()}
            return Ok(ExitCode::SUCCESS);
        }
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(if result.is_error == Some(true) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
