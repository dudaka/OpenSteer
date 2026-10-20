//! The demos the host can run, chosen by id. An enum, so dispatch is a plain `match`.

use steer_core::Vec2;

use crate::view::FrameView;
use crate::{DemoInfo, OneTurning, SeekFlee};

/// Every demo in sidebar order (OpenSteerDemo's `selectionOrderSortKey`): id and name.
const DEMOS: [(&str, &str); 2] = [
    ("one-turning", "One Turning Away"),
    ("seek-flee", "Seek and Flee"),
];

/// The demo selected at start.
pub const DEFAULT_DEMO: &str = "one-turning";

#[derive(Debug, Clone)]
pub enum Demo {
    OneTurning(OneTurning),
    SeekFlee(SeekFlee),
}

impl Demo {
    /// A fresh demo in its initial state; `None` for an unknown id.
    pub fn new(id: &str) -> Option<Demo> {
        match id {
            "one-turning" => Some(Demo::OneTurning(OneTurning::default())),
            "seek-flee" => Some(Demo::SeekFlee(SeekFlee::default())),
            _ => None,
        }
    }

    pub fn update(&mut self, dt: f32) {
        match self {
            Demo::OneTurning(d) => d.update(dt),
            Demo::SeekFlee(d) => d.update(dt),
        }
    }

    pub fn view(&self) -> FrameView {
        match self {
            Demo::OneTurning(d) => d.view(),
            Demo::SeekFlee(d) => d.view(),
        }
    }

    /// A click on the ground at world point `at`; demos without click input ignore it.
    pub fn click(&mut self, at: Vec2) {
        if let Demo::SeekFlee(d) = self {
            d.click(at);
        }
    }

    /// One of the demo's `ActionView` buttons; unknown ids are ignored.
    pub fn action(&mut self, id: &str) {
        if let Demo::SeekFlee(d) = self {
            d.action(id);
        }
    }
}

pub fn catalog() -> Vec<DemoInfo> {
    DEMOS
        .iter()
        .map(|&(id, name)| DemoInfo {
            id: id.to_string(),
            name: name.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_entry_builds() {
        for d in catalog() {
            assert!(Demo::new(&d.id).is_some(), "{}", d.id);
        }
        assert!(Demo::new(DEFAULT_DEMO).is_some());
        assert!(Demo::new("nope").is_none());
    }
}
