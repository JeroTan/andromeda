use crate::{
    accessibility, capture,
    device::{Adb, Screen},
    errors::{Error, Result},
    input::{Action, Point, duration},
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    Text {
        value: String,
        #[serde(default)]
        exact: bool,
    },
    ResourceId {
        value: String,
    },
    Pixel {
        point: Point,
        rgb: [u8; 3],
        tolerance: u8,
        screen: Screen,
    },
}

#[derive(Debug, Clone)]
pub enum Operation {
    ListDevices,
    DeviceInfo,
    View(capture::Options),
    UiTree,
    ListApps {
        include_system: bool,
    },
    Actions {
        actions: Vec<Action>,
        interval_ms: u64,
    },
    WaitFor {
        condition: Condition,
        poll_ms: u64,
    },
}

#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub serial: Option<String>,
    pub timeout_ms: Option<u64>,
}

pub struct Output {
    pub data: Value,
    pub image: Option<capture::Image>,
}
struct UiObservation {
    screen: Screen,
    nodes: Vec<accessibility::Node>,
}
impl Output {
    fn data(data: Value) -> Self {
        Self { data, image: None }
    }
}

pub struct Execution {
    pub operation_id: u64,
    pub elapsed_ms: f64,
    pub result: Result<Output>,
}

#[derive(Clone)]
pub struct Controller {
    inner: Arc<Inner>,
}
struct Inner {
    adb: Adb,
    default_serial: Option<String>,
    allow_control: bool,
    next_id: AtomicU64,
    active: Mutex<BTreeMap<u64, CancellationToken>>,
    action_gate: tokio::sync::Mutex<()>,
    image_gate: Arc<tokio::sync::Semaphore>,
}

struct ActiveOperation {
    inner: Arc<Inner>,
    id: u64,
}
impl Drop for ActiveOperation {
    fn drop(&mut self) {
        self.inner
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.id);
    }
}

impl Controller {
    pub fn new(adb: Adb, default_serial: Option<String>, allow_control: bool) -> Self {
        Self {
            inner: Arc::new(Inner {
                adb,
                default_serial,
                allow_control,
                next_id: AtomicU64::new(1),
                active: Mutex::new(BTreeMap::new()),
                action_gate: tokio::sync::Mutex::new(()),
                image_gate: Arc::new(tokio::sync::Semaphore::new(2)),
            }),
        }
    }

    pub fn stop(&self) -> usize {
        let active = self.inner.active.lock().unwrap_or_else(|e| e.into_inner());
        for token in active.values() {
            token.cancel();
        }
        active.len()
    }

    pub async fn run(
        &self,
        operation: Operation,
        options: RunOptions,
        cancel: CancellationToken,
    ) -> Execution {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        let result = self.run_inner(operation, options, cancel, id).await;
        self.inner
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        tracing::info!(
            target: "andromeda::audit",
            operation_id = id,
            elapsed_ms,
            success = result.is_ok(),
            "device operation finished"
        );
        Execution {
            operation_id: id,
            elapsed_ms,
            result,
        }
    }

    async fn run_inner(
        &self,
        operation: Operation,
        options: RunOptions,
        cancel: CancellationToken,
        id: u64,
    ) -> Result<Output> {
        let timeout_ms = options.timeout_ms.unwrap_or(15000);
        duration(timeout_ms, 1, 120000)?;
        {
            let mut active = self.inner.active.lock().unwrap_or_else(|e| e.into_inner());
            if active.len() >= 32 {
                return Err(Error::new(
                    "busy",
                    "At most 32 requests may run concurrently",
                ));
            }
            active.insert(id, cancel.clone());
        }
        let _registration = ActiveOperation {
            inner: self.inner.clone(),
            id,
        };
        tokio::select! {
            biased;
            _=cancel.cancelled()=>Err(Error::new("cancelled","Operation cancelled; already dispatched input may have taken effect. Observe before retrying.")),
            result=tokio::time::timeout(Duration::from_millis(timeout_ms),self.perform(operation,options.serial))=>result.map_err(|_|Error::new("timeout","Operation deadline exceeded; observe before retrying input"))?,
        }
    }

    async fn perform(&self, operation: Operation, serial: Option<String>) -> Result<Output> {
        if matches!(operation, Operation::Actions { .. }) && !self.inner.allow_control {
            return Err(Error::new(
                "control_disabled",
                "Input is disabled. The operator must restart with --allow-control to authorize device control.",
            ));
        }
        if matches!(operation, Operation::ListDevices) {
            return Ok(Output::data(
                json!({"devices":self.inner.adb.list_devices().await?}),
            ));
        }
        let serial=serial.or_else(||self.inner.default_serial.clone()).ok_or_else(||Error::new("device_required","Choose a serial from adm_list_devices or set --device. Devices are never selected implicitly."))?;
        match operation {
            Operation::DeviceInfo => {
                let screen = self.inner.adb.screen(&serial).await?;
                let properties=self.inner.adb.shell(&serial,"getprop ro.product.model; getprop ro.product.manufacturer; getprop ro.build.version.release; getprop ro.build.version.sdk").await?;
                let properties = String::from_utf8_lossy(&properties);
                let mut lines = properties.lines();
                Ok(Output::data(
                    json!({"serial":serial,"screen":screen,"model":lines.next(),"manufacturer":lines.next(),"android_version":lines.next(),"api_level":lines.next(),"control_enabled":self.inner.allow_control}),
                ))
            }
            Operation::ListApps { include_system } => {
                let output = self
                    .inner
                    .adb
                    .shell(
                        &serial,
                        if include_system {
                            "pm list packages"
                        } else {
                            "pm list packages -3"
                        },
                    )
                    .await?;
                let text = String::from_utf8_lossy(&output);
                let mut packages: Vec<&str> = text
                    .lines()
                    .filter_map(|line| line.strip_prefix("package:"))
                    .collect();
                packages.sort_unstable();
                Ok(Output::data(json!({"packages":packages})))
            }
            Operation::View(options) => {
                options.validate()?;
                let permit = self
                    .inner
                    .image_gate
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| Error::new("shutdown", "Image processor closed"))?;
                let screen = self.inner.adb.screen(&serial).await?;
                let png = self.inner.adb.shell(&serial, "screencap -p").await?;
                self.inner.adb.ensure_screen(&serial, screen).await?;
                let image = tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    capture::encode(png, options)
                })
                .await
                .map_err(|e| Error::new("image_encoding", e.to_string()))??;
                if image.native_width != screen.width || image.native_height != screen.height {
                    return Err(Error::new(
                        "stale_screen",
                        "Screenshot dimensions disagree with display geometry; observe again",
                    ));
                }
                Ok(Output {
                    data: json!({"serial":serial,"screen":screen,"image_width":image.width,"image_height":image.height,"scale_to_screen":{"x":screen.width as f64/image.width as f64,"y":screen.height as f64/image.height as f64},"bytes":image.bytes.len()}),
                    image: Some(image),
                })
            }
            Operation::UiTree => {
                let tree = self.ui_tree(&serial).await?;
                Ok(Output::data(
                    json!({"serial":serial,"screen":tree.screen,"nodes":tree.nodes}),
                ))
            }
            Operation::Actions {
                actions,
                interval_ms,
            } => {
                if actions.is_empty() || actions.len() > 100 {
                    return Err(Error::invalid("Provide 1..=100 actions"));
                }
                duration(interval_ms, 0, 10000)?;
                for action in &actions {
                    action.validate()?;
                }
                let _guard = self.inner.action_gate.try_lock().map_err(|_| {
                    Error::new(
                        "device_busy",
                        "Another input sequence is running; wait or call adm_stop",
                    )
                })?;
                for (index, action) in actions.iter().enumerate() {
                    if index > 0 {
                        tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                    }
                    if let Some(screen) = action.screen() {
                        self.inner.adb.ensure_screen(&serial, screen).await?;
                    }
                    match action {
                        Action::Pause { duration_ms } => {
                            tokio::time::sleep(Duration::from_millis(*duration_ms)).await
                        }
                        _ => {
                            if let Some(command) = action.shell_command()? {
                                let output=self.inner.adb.shell(&serial,&command).await.map_err(|mut e| {e.retryable=false;e.message=format!("Action {index} failed after {index} completed actions: {}",e.message);e})?;
                                let output = String::from_utf8_lossy(&output);
                                if output
                                    .lines()
                                    .any(|line| line.trim_start().starts_with("Error:"))
                                {
                                    return Err(Error::new(
                                        "device_command_failed",
                                        format!("Android rejected action {index}: {output}"),
                                    ));
                                }
                            }
                        }
                    }
                }
                Ok(Output::data(
                    json!({"completed_actions":actions.len(),"verified":false,"note":"Input dispatched. Observe the device to verify the intended outcome."}),
                ))
            }
            Operation::WaitFor { condition, poll_ms } => {
                duration(poll_ms, 100, 5000)?;
                match &condition {
                    Condition::Text { value, .. } | Condition::ResourceId { value }
                        if value.is_empty() || value.len() > 4096 =>
                    {
                        return Err(Error::invalid("Wait condition must contain 1..=4096 bytes"));
                    }
                    Condition::Pixel { point, screen, .. } => {
                        screen.validate_point(point.x, point.y)?
                    }
                    _ => {}
                }
                loop {
                    let matched = match &condition {
                        Condition::Text { value, exact } => {
                            self.ui_tree(&serial).await?.nodes.iter().any(|node| {
                                if *exact {
                                    node.text == *value || node.description == *value
                                } else {
                                    node.text.contains(value) || node.description.contains(value)
                                }
                            })
                        }
                        Condition::ResourceId { value } => self
                            .ui_tree(&serial)
                            .await?
                            .nodes
                            .iter()
                            .any(|node| node.resource_id == *value),
                        Condition::Pixel {
                            point,
                            rgb,
                            tolerance,
                            screen,
                        } => {
                            self.inner.adb.ensure_screen(&serial, *screen).await?;
                            let permit = self
                                .inner
                                .image_gate
                                .clone()
                                .acquire_owned()
                                .await
                                .map_err(|_| Error::new("shutdown", "Image processor closed"))?;
                            let png = self.inner.adb.shell(&serial, "screencap -p").await?;
                            self.inner.adb.ensure_screen(&serial, *screen).await?;
                            let (point, rgb, tolerance, screen) =
                                (point.clone(), *rgb, *tolerance, *screen);
                            tokio::task::spawn_blocking(move || {
                                use image::GenericImageView;
                                let _permit = permit;
                                let image = capture::decode(&png)?;
                                if image.dimensions() != (screen.width, screen.height) {
                                    return Err(Error::new(
                                        "stale_screen",
                                        "Screenshot dimensions changed",
                                    ));
                                }
                                let pixel = image.get_pixel(point.x, point.y).0;
                                Ok((0..3).all(|i| pixel[i].abs_diff(rgb[i]) <= tolerance))
                            })
                            .await
                            .map_err(|e| Error::new("image_decode", e.to_string()))??
                        }
                    };
                    if matched {
                        return Ok(Output::data(json!({"matched":true})));
                    }
                    tokio::time::sleep(Duration::from_millis(poll_ms)).await;
                }
            }
            Operation::ListDevices => unreachable!("handled before serial selection"),
        }
    }

    async fn ui_tree(&self, serial: &str) -> Result<UiObservation> {
        let screen = self.inner.adb.screen(serial).await?;
        let output = self
            .inner
            .adb
            .shell(serial, r#"umask 077; file=$(mktemp /data/local/tmp/andromeda-ui.XXXXXX) || exit 1; trap 'rm -f "$file"' EXIT HUP INT TERM; uiautomator dump "$file" >/dev/null && cat "$file""#)
            .await?;
        let hierarchy = accessibility::parse(&String::from_utf8_lossy(&output))?;
        self.inner.adb.ensure_screen(serial, screen).await?;
        if hierarchy.rotation != screen.rotation {
            return Err(Error::new(
                "stale_screen",
                "UI hierarchy rotation changed during observation; inspect again",
            ));
        }
        Ok(UiObservation {
            screen,
            nodes: hierarchy.nodes,
        })
    }
}
