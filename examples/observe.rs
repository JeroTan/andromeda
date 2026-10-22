//! Run with ANDROMEDA_DEVICE set to an authorized device's serial.
use andromeda::{
    actions::{Controller, Operation, RunOptions},
    capture,
    device::Adb,
};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let serial = std::env::var("ANDROMEDA_DEVICE")?;
    let controller = Controller::new(Adb::new("127.0.0.1:5037".parse()?), Some(serial), false);
    for operation in [
        Operation::DeviceInfo,
        Operation::View(capture::Options {
            max_width: Some(720),
            ..Default::default()
        }),
        Operation::UiTree,
    ] {
        let report = controller
            .run(operation, RunOptions::default(), CancellationToken::new())
            .await;
        let output = report.result?;
        println!("{}", serde_json::to_string_pretty(&output.data)?);
        if let Some(image) = output.image {
            println!(
                "Received {} bytes of {} in memory",
                image.bytes.len(),
                image.mime_type
            );
        }
    }
    Ok(())
}
