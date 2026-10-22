use crate::{
    device::Screen,
    errors::{Error, Result},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Swipe {
    pub from: Point,
    pub to: Point,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Home,
    Back,
    Recents,
    Enter,
    Delete,
    Tab,
    Escape,
    VolumeUp,
    VolumeDown,
    Power,
    Wakeup,
}

impl Key {
    pub fn code(&self) -> u16 {
        match self {
            Self::Home => 3,
            Self::Back => 4,
            Self::Recents => 187,
            Self::Enter => 66,
            Self::Delete => 67,
            Self::Tab => 61,
            Self::Escape => 111,
            Self::VolumeUp => 24,
            Self::VolumeDown => 25,
            Self::Power => 26,
            Self::Wakeup => 224,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Touch { point: Point, screen: Screen },
    Swipe { swipe: Swipe, screen: Screen },
    PressKey { key: Key },
    TypeText { text: String },
    LaunchApp { package: String },
    OpenUri { uri: String },
    Pause { duration_ms: u64 },
}

impl Action {
    pub fn screen(&self) -> Option<Screen> {
        match self {
            Self::Touch { screen, .. } | Self::Swipe { screen, .. } => Some(*screen),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Touch { point, screen } => screen.validate_point(point.x, point.y),
            Self::Swipe { swipe, screen } => {
                screen.validate_point(swipe.from.x, swipe.from.y)?;
                screen.validate_point(swipe.to.x, swipe.to.y)?;
                duration(swipe.duration_ms, 1, 10000)
            }
            Self::TypeText { text } => {
                if text.is_empty()
                    || text.len() > 4096
                    || !text.bytes().all(|c| (32..=126).contains(&c) && c != b'%')
                {
                    return Err(Error::invalid(
                        "ADB text requires 1..=4096 printable ASCII bytes without %. Use companion input for Unicode or literal %.",
                    ));
                }
                Ok(())
            }
            Self::LaunchApp { package } => {
                if package.len() > 255
                    || !package.contains('.')
                    || !package.split('.').all(|s| {
                        !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                    })
                {
                    return Err(Error::invalid(
                        "Expected a fully qualified Android package name",
                    ));
                }
                Ok(())
            }
            Self::OpenUri { uri } => {
                if uri.len() > 8192
                    || uri.chars().any(char::is_control)
                    || !uri.split_once(':').is_some_and(|(s, _)| {
                        !s.is_empty()
                            && s.starts_with(|c: char| c.is_ascii_alphabetic())
                            && s.bytes()
                                .all(|c| c.is_ascii_alphanumeric() || b"+.-".contains(&c))
                    })
                {
                    return Err(Error::invalid(
                        "Expected a URI with a valid scheme and no control characters",
                    ));
                }
                if uri.starts_with("intent:")
                    || uri.starts_with("file:")
                    || uri.starts_with("javascript:")
                {
                    return Err(Error::invalid(
                        "intent, file and javascript URIs are not supported",
                    ));
                }
                Ok(())
            }
            Self::Pause { duration_ms } => duration(*duration_ms, 0, 10000),
            Self::PressKey { .. } => Ok(()),
        }
    }

    pub fn shell_command(&self) -> Result<Option<String>> {
        self.validate()?;
        Ok(Some(match self {
            Self::Touch { point, .. } => format!("input tap {} {}", point.x, point.y),
            Self::Swipe { swipe, .. } => format!(
                "input swipe {} {} {} {} {}",
                swipe.from.x, swipe.from.y, swipe.to.x, swipe.to.y, swipe.duration_ms
            ),
            Self::PressKey { key } => format!("input keyevent {}", key.code()),
            Self::TypeText { text } => format!("input text {}", quote(&text.replace(' ', "%s"))),
            Self::LaunchApp { package } => format!(
                "adm_component=$(cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p {} | tail -n 1); case \"$adm_component\" in {}*) am start -W -n \"$adm_component\";; *) printf '%s\\n' {} >&2; exit 1;; esac",
                quote(package),
                quote(&format!("{package}/")),
                quote(&format!("No launcher activity found for {package}"))
            ),
            Self::OpenUri { uri } => format!(
                "am start -W -a android.intent.action.VIEW -d {}",
                quote(uri)
            ),
            Self::Pause { .. } => return Ok(None),
        }))
    }
}

pub fn duration(value: u64, min: u64, max: u64) -> Result<()> {
    if !(min..=max).contains(&value) {
        return Err(Error::invalid(format!("Duration must be {min}..={max} ms")));
    }
    Ok(())
}

/// POSIX single-quote escaping. Arguments never become command syntax.
pub fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
