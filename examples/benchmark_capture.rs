//! Read-only capture and bounded device workloads; images stay in memory.
use andromeda::{
    actions::{Controller, Operation, RunOptions},
    capture,
    device::Adb,
    input::{Action, Key},
};
use clap::Parser;
use serde::Serialize;
use serde_json::json;
use std::{
    future::Future,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
struct Args {
    #[arg(long, env = "ANDROMEDA_DEVICE")]
    device: String,
    #[arg(long, default_value = "adb")]
    adb: PathBuf,
    #[arg(long,default_value_t=20,value_parser=clap::value_parser!(u32).range(3..=200))]
    samples: u32,
    #[arg(long,default_value_t=2,value_parser=clap::value_parser!(u32).range(0..=20))]
    warmup: u32,
    /// Maximum seconds spent collecting persistent-stream frames.
    #[arg(long,default_value_t=60,value_parser=clap::value_parser!(u64).range(1..=120))]
    stream_seconds: u64,
    /// Delay requested between persistent-stream captures.
    #[arg(long,default_value_t=100,value_parser=clap::value_parser!(u64).range(0..=5000))]
    stream_interval_ms: u64,
    /// Allow the harness to dispatch a harmless Home key between input samples.
    #[arg(long)]
    allow_input: bool,
    #[arg(long)]
    output: Option<PathBuf>,
}
#[derive(Serialize)]
struct Sample {
    elapsed_ms: f64,
    bytes: Option<usize>,
    error_code: Option<String>,
}
#[derive(Serialize, Default)]
struct Usage {
    cpu_percent_avg: Option<f64>,
    cpu_percent_max: Option<f64>,
    rss_kb_max: Option<u64>,
}
#[derive(Serialize)]
struct Workload {
    name: &'static str,
    description: &'static str,
    metric: &'static str,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    fps: Option<f64>,
    failures: usize,
    usage: Usage,
    samples: Vec<Sample>,
}
fn percentile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted[((sorted.len() as f64 * fraction).ceil() as usize).saturating_sub(1)])
}
fn battery(bytes: &[u8]) -> (Option<u32>, Option<f64>) {
    let text = String::from_utf8_lossy(bytes);
    let mut level = None;
    let mut temperature = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("level: ") {
            level = value.trim().parse().ok();
        }
        if let Some(value) = line.strip_prefix("temperature: ") {
            temperature = value.trim().parse::<f64>().ok().map(|value| value / 10.0);
        }
    }
    (level, temperature)
}

fn host_usage() -> Option<(f64, u64)> {
    #[cfg(unix)]
    {
        let output = std::process::Command::new("ps")
            .args(["-o", "%cpu=,rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut fields = text.split_whitespace();
        Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
    }
    #[cfg(not(unix))]
    {
        None
    }
}

fn spawn_usage(cancel: CancellationToken) -> tokio::task::JoinHandle<Usage> {
    tokio::spawn(async move {
        let mut cpu_samples = Vec::new();
        let mut rss_kb_max = None;
        loop {
            if let Some((cpu, rss)) = host_usage() {
                cpu_samples.push(cpu);
                rss_kb_max = Some(rss_kb_max.unwrap_or(0).max(rss));
            }
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_millis(500)) => {}
            }
        }
        Usage {
            cpu_percent_avg: (!cpu_samples.is_empty())
                .then(|| cpu_samples.iter().sum::<f64>() / cpu_samples.len() as f64),
            cpu_percent_max: cpu_samples.iter().copied().reduce(f64::max),
            rss_kb_max,
        }
    })
}

async fn measured<T>(future: impl Future<Output = T>) -> (T, Usage) {
    let cancel = CancellationToken::new();
    let sampler = spawn_usage(cancel.clone());
    let value = future.await;
    cancel.cancel();
    (value, sampler.await.unwrap_or_default())
}

fn build_workload(
    name: &'static str,
    description: &'static str,
    metric: &'static str,
    samples: Vec<Sample>,
    fps: Option<f64>,
    usage: Usage,
) -> Workload {
    let successful: Vec<f64> = samples
        .iter()
        .filter(|sample| sample.error_code.is_none())
        .map(|sample| sample.elapsed_ms)
        .collect();
    Workload {
        name,
        description,
        metric,
        p50_ms: percentile(&successful, 0.5),
        p95_ms: percentile(&successful, 0.95),
        fps,
        failures: samples.len() - successful.len(),
        usage,
        samples,
    }
}

async fn capture_samples(
    adb: &Adb,
    controller: &Controller,
    args: &Args,
    workload: usize,
) -> Vec<Sample> {
    let mut samples = Vec::new();
    for iteration in 0..args.warmup + args.samples {
        let started = Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(15), async {
            match workload {
                0 => adb
                    .shell(&args.device, "screencap -p")
                    .await
                    .map(|bytes| bytes.len())
                    .map_err(|e| e.code),
                1 => {
                    let output = tokio::process::Command::new(&args.adb)
                        .args(["-s", &args.device, "shell", "screencap", "-p"])
                        .kill_on_drop(true)
                        .output()
                        .await
                        .map_err(|_| "adb_cli_unavailable".to_owned())?;
                    if output.status.success() {
                        Ok(output.stdout.len())
                    } else {
                        Err("adb_cli_failed".into())
                    }
                }
                _ => {
                    let options = if workload == 2 {
                        capture::Options::default()
                    } else {
                        capture::Options {
                            max_width: Some(720),
                            format: capture::Format::Jpeg,
                            quality: Some(80),
                            ..Default::default()
                        }
                    };
                    let report = controller
                        .run(
                            Operation::View(options),
                            RunOptions {
                                timeout_ms: Some(15000),
                                ..Default::default()
                            },
                            CancellationToken::new(),
                        )
                        .await;
                    report
                        .result
                        .map_err(|e| e.code)?
                        .image
                        .map(|image| image.bytes.len())
                        .ok_or_else(|| "missing_image".into())
                }
            }
        })
        .await
        .unwrap_or_else(|_| Err("timeout".into()));
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        if iteration >= args.warmup {
            samples.push(match result {
                Ok(bytes) => Sample {
                    elapsed_ms,
                    bytes: Some(bytes),
                    error_code: None,
                },
                Err(code) => Sample {
                    elapsed_ms,
                    bytes: None,
                    error_code: Some(code),
                },
            });
        }
    }
    samples
}

async fn stream_samples(adb: &Adb, args: &Args) -> (Vec<Sample>, Option<f64>) {
    let mut stream = match adb.stream(&args.device, args.stream_interval_ms).await {
        Ok(stream) => stream,
        Err(error) => {
            return (
                vec![Sample {
                    elapsed_ms: 0.0,
                    bytes: None,
                    error_code: Some(error.code),
                }],
                None,
            );
        }
    };
    let maximum_frames = args.warmup + args.samples;
    let deadline = Instant::now() + Duration::from_secs(args.stream_seconds);
    let mut frames = 0;
    let mut samples = Vec::new();
    let mut previous = Instant::now();
    let mut first_measured = None;
    let mut last_measured = None;
    while frames < maximum_frames {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        let result =
            tokio::time::timeout(remaining, stream.next_frame(Duration::from_secs(10))).await;
        let frame = match result {
            Ok(Ok(Some(frame))) => frame,
            Ok(Ok(None)) => {
                samples.push(Sample {
                    elapsed_ms: previous.elapsed().as_secs_f64() * 1000.0,
                    bytes: None,
                    error_code: Some("stream_ended".into()),
                });
                break;
            }
            Ok(Err(error)) => {
                samples.push(Sample {
                    elapsed_ms: previous.elapsed().as_secs_f64() * 1000.0,
                    bytes: None,
                    error_code: Some(error.code),
                });
                break;
            }
            Err(_) => {
                // The outer timeout is the measurement deadline. A stream
                // stall is reported by next_frame's own timeout instead.
                break;
            }
        };
        let now = Instant::now();
        let interval_ms = now.duration_since(previous).as_secs_f64() * 1000.0;
        previous = now;
        if frames >= args.warmup {
            samples.push(Sample {
                elapsed_ms: interval_ms,
                bytes: Some(frame.len()),
                error_code: None,
            });
            first_measured.get_or_insert(now);
            last_measured = Some(now);
        }
        frames += 1;
    }
    let measured_frames = samples
        .iter()
        .filter(|sample| sample.error_code.is_none())
        .count();
    let fps = match (first_measured, last_measured, measured_frames) {
        (Some(first), Some(last), frames) if frames > 1 => {
            let seconds = last.duration_since(first).as_secs_f64();
            (seconds > 0.0).then_some((frames - 1) as f64 / seconds)
        }
        _ => None,
    };
    (samples, fps)
}

async fn input_samples(controller: &Controller, args: &Args) -> Vec<Sample> {
    let mut samples = Vec::new();
    for iteration in 0..args.warmup + args.samples {
        let started = Instant::now();
        let report = controller
            .run(
                Operation::Actions {
                    actions: vec![Action::PressKey { key: Key::Home }],
                    interval_ms: 0,
                },
                RunOptions {
                    timeout_ms: Some(15000),
                    ..Default::default()
                },
                CancellationToken::new(),
            )
            .await;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        if iteration >= args.warmup {
            samples.push(match report.result {
                Ok(_) => Sample {
                    elapsed_ms,
                    bytes: None,
                    error_code: None,
                },
                Err(error) => Sample {
                    elapsed_ms,
                    bytes: None,
                    error_code: Some(error.code),
                },
            });
        }
    }
    samples
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let adb = Adb::new("127.0.0.1:5037".parse()?);
    let controller = Controller::new(adb.clone(), Some(args.device.clone()), args.allow_input);
    let screen = adb.screen(&args.device).await?;
    let properties=adb.shell(&args.device,"getprop ro.product.model; getprop ro.build.version.release; getprop ro.build.version.sdk").await?;
    let properties = String::from_utf8_lossy(&properties);
    let mut properties = properties.lines();
    let details = adb
        .list_devices()
        .await
        .ok()
        .and_then(|devices| {
            devices
                .into_iter()
                .find(|device| device.serial == args.device)
        })
        .map(|device| device.details)
        .unwrap_or_default();
    let emulator = adb
        .shell(&args.device, "getprop ro.kernel.qemu")
        .await
        .ok()
        .is_some_and(|bytes| String::from_utf8_lossy(&bytes).trim() == "1");
    let transport = if emulator {
        "emulator"
    } else if details.contains("usb:") {
        "usb"
    } else {
        "adb-network-or-unknown"
    };
    let (battery_level, battery_temperature) = adb
        .shell(&args.device, "dumpsys battery")
        .await
        .ok()
        .map(|bytes| battery(&bytes))
        .unwrap_or((None, None));
    let metadata = json!({"model":properties.next(),"android_version":properties.next(),"api_level":properties.next(),"screen":screen,"transport":transport,"emulator":emulator,"battery_level_percent":battery_level,"battery_temperature_c":battery_temperature,"host_os":std::env::consts::OS,"host_arch":std::env::consts::ARCH,"build_profile":if cfg!(debug_assertions){"debug"}else{"release"},"logical_cpus":std::thread::available_parallelism().map(|n|n.get()).ok(),"timestamp_unix_ms":SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),"warmup_per_workload":args.warmup,"samples_per_workload":args.samples,"deadline_ms":15000,"stream_seconds":args.stream_seconds,"stream_interval_ms":args.stream_interval_ms,"stream_max_frames":args.warmup+args.samples,"input_enabled":args.allow_input,"usage_source":"ps %cpu/rss (host process; platform averaging differs)","model_reasoning_included":false,"screenshots_saved":false});
    let definitions = [
        (
            "direct_adb_png",
            "Direct ADB shell-v2 screencap; native PNG, no decode or geometry checks inside the timed request",
        ),
        (
            "adb_cli_png",
            "External adb shell screencap process; same native PNG request, includes host process startup",
        ),
        (
            "controller_native_png",
            "Controller capture with before/after geometry checks, bounded PNG decode and native PNG output",
        ),
        (
            "controller_jpeg_720",
            "Controller capture with geometry checks, resize to max width 720, JPEG quality 80",
        ),
    ];
    let mut workloads = Vec::new();
    for (index, (name, description)) in definitions.into_iter().enumerate() {
        let (samples, usage) = measured(capture_samples(&adb, &controller, &args, index)).await;
        workloads.push(build_workload(
            name,
            description,
            "request_latency_ms",
            samples,
            None,
            usage,
        ));
        adb.ensure_screen(&args.device, screen).await?;
    }
    let ((samples, fps), usage) = measured(stream_samples(&adb, &args)).await;
    workloads.push(build_workload(
        "persistent_stream_png",
        "One persistent shell-v2 screencap loop; elapsed_ms is the interval between complete native PNG frames",
        "frame_interval_ms",
        samples,
        fps,
        usage,
    ));
    if args.allow_input {
        let (samples, usage) = measured(input_samples(&controller, &args)).await;
        workloads.push(build_workload(
            "controller_press_home",
            "Controller dispatches KEYCODE_HOME through the normal validation and transport path; no screen capture is included",
            "input_latency_ms",
            samples,
            None,
            usage,
        ));
    }
    let report = json!({"schema_version":2,"metadata":metadata,"workloads":workloads,"limitations":["Workloads run sequentially; repeat in reversed order to examine temperature/order effects.","External adb is a transport baseline, not an Android MCP product comparison.","Controller workloads include validation and image processing; compare raw PNG paths for transport overhead.","Battery level and temperature are recorded, but thermal throttling is not controlled.","Host CPU/RSS come from periodic ps samples of this harness process; CPU averaging differs by platform and includes image work.","Persistent-stream frame intervals are not input latency and do not prove a stable frame rate under load.","The input workload presses Home and does not measure gesture accuracy, task completion or model/tool overhead."]});
    let bytes = serde_json::to_vec_pretty(&report)?;
    if let Some(path) = args.output {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        println!(
            "Saved {} measured workloads to {}",
            workloads.len(),
            path.display()
        );
    } else {
        println!("{}", String::from_utf8(bytes)?);
    }
    Ok(())
}
