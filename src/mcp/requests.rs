use crate::{
    actions::{Condition, RunOptions},
    capture,
    device::Screen,
    input::{Action, Key, Point, Stroke, Swipe, TextBackend, TextMode},
};
use schemars::JsonSchema;
use serde::Deserialize;

macro_rules! request {
    ($name:ident { $($(#[$meta:meta])* $field:ident : $kind:ty),* $(,)? }) => {
        #[derive(Deserialize,JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            /// Android serial. Required unless the operator set --device.
            pub serial: Option<String>,
            /// Deadline including queueing, transport and processing: 1..=120000 ms.
            pub timeout_ms: Option<u64>,
            $($(#[$meta])* pub $field:$kind),*
        }
        impl $name {
            pub fn options(&self)->RunOptions {RunOptions {serial:self.serial.clone(),timeout_ms:self.timeout_ms}}
        }
    }
}

request!(Target {});
request!(View {
    #[serde(default)]
    image: capture::Options
});
request!(LongView {
    screen: Screen,
    viewport: capture::Viewport,
    swipe: Swipe,
    #[serde(default = "capture::default_pages")]
    max_pages: u8,
    #[serde(default = "capture::default_settle_ms")]
    settle_ms: u64,
    #[serde(default)]
    image: capture::Options
});
request!(Apps {
    #[serde(default)]
    include_system: bool
});
request!(Touch {
    point: Point,
    screen: Screen
});
request!(TouchMany { points:Vec<Point>, screen:Screen, #[serde(default)] interval_ms:u64 });
request!(SwipeOne {
    /// Required gesture object containing from, to and duration_ms.
    swipe: Swipe,
    screen: Screen
});
request!(SwipeMany { swipes:Vec<Swipe>, screen:Screen, #[serde(default)] interval_ms:u64 });
request!(PressKey { key: Key });
request!(TypeText {
    text: String,
    #[serde(default)]
    backend: TextBackend,
    #[serde(default)]
    mode: TextMode
});
request!(MultiTouch { strokes:Vec<Stroke>, screen:Screen });
request!(LaunchApp { package: String });
request!(OpenUri { uri: String });
request!(Batch { actions:Vec<Action>, #[serde(default)] interval_ms:u64 });
request!(Wait {
    /// Required real condition to observe. Use batch pause for a fixed delay instead.
    condition: Condition,
    poll_ms: Option<u64>
});

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
