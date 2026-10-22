use andromeda::{
    actions::Controller,
    device::Adb,
    mcp::{Server, tools},
};
use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use std::{net::SocketAddr, process::ExitCode};
use tokio_util::sync::CancellationToken;

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
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .json()
        .init();
    if !cli.adb_server.ip().is_loopback() {
        return Err("ADB server must use a loopback address".into());
    }
    let controller = Controller::new(Adb::new(cli.adb_server), cli.device, cli.allow_control);
    let server = Server {
        controller: controller.clone(),
    };
    let result = match cli.command.unwrap_or(Command::Serve) {
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
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(if result.is_error == Some(true) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
