use super::Options;
use crate::{
    device::Screen,
    errors::{Error, Result},
    input::{Action, Swipe},
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Scroll {
    pub screen: Screen,
    pub viewport: Viewport,
    pub swipe: Swipe,
    pub max_pages: u8,
    pub settle_ms: u64,
    pub image: Options,
}

pub fn default_pages() -> u8 {
    4
}
pub fn default_settle_ms() -> u64 {
    300
}

impl Scroll {
    pub fn validate(&self) -> Result<()> {
        self.image.validate()?;
        let area = self.viewport;
        if !(1..=8).contains(&self.max_pages)
            || self.settle_ms > 3000
            || self.image.max_width.is_some_and(|n| n > 1024)
            || self
                .image
                .max_height
                .is_some_and(|n| n < u32::from(self.max_pages))
            || area.width == 0
            || area.height == 0
            || area
                .x
                .checked_add(area.width)
                .is_none_or(|end| end > self.screen.width)
            || area
                .y
                .checked_add(area.height)
                .is_none_or(|end| end > self.screen.height)
        {
            return Err(Error::invalid(
                "Long view needs 1..=8 pages, settle_ms 0..=3000, image width <=1024, and a nonempty viewport inside the screen",
            ));
        }
        Action::Swipe {
            swipe: self.swipe.clone(),
            screen: self.screen,
        }
        .validate()?;
        for point in [&self.swipe.from, &self.swipe.to] {
            if point.x < area.x
                || point.x >= area.x + area.width
                || point.y < area.y
                || point.y >= area.y + area.height
            {
                return Err(Error::invalid(
                    "Scroll endpoints must stay inside the requested viewport",
                ));
            }
        }
        if self.swipe.from.x == self.swipe.to.x && self.swipe.from.y == self.swipe.to.y {
            return Err(Error::invalid(
                "A scrolling capture needs distinct swipe endpoints",
            ));
        }
        Ok(())
    }
}
