use crate::{
    actions::{Condition, RunOptions},
    capture,
    device::Screen,
    input::{Action, Key, Point, Swipe},
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
    swipe: Swipe,
    screen: Screen
});
request!(SwipeMany { swipes:Vec<Swipe>, screen:Screen, #[serde(default)] interval_ms:u64 });
request!(PressKey { key: Key });
request!(TypeText { text: String });
request!(LaunchApp { package: String });
request!(OpenUri { uri: String });
request!(Batch { actions:Vec<Action>, #[serde(default)] interval_ms:u64 });
request!(Wait { condition:Condition, poll_ms:Option<u64> });

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Empty {}
