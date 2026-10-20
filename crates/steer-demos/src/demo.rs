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

    /// No color goes unexplained: every line and trail color a demo draws, after any one of
    /// its actions, is in its legend.
    #[test]
    fn every_drawn_color_has_a_legend_entry() {
        for d in catalog() {
            let actions = Demo::new(&d.id).expect("catalog demo").view().actions;
            let pressed = actions.iter().map(|a| Some(a.id.as_str()));
            for action in [None].into_iter().chain(pressed) {
                let mut demo = Demo::new(&d.id).expect("catalog demo");
                if let Some(id) = action {
                    demo.action(id);
                }
                for _ in 0..60 {
                    demo.update(1.0 / 60.0);
                }
                let view = demo.view();
                let explained: Vec<u32> = view.legend.iter().map(|l| l.color).collect();
                let drawn = view.lines.iter().map(|l| l.color);
                for color in drawn.chain(view.trails.iter().map(|t| t.color)) {
                    assert!(
                        explained.contains(&color),
                        "{} after {action:?}: {color:06x} has no legend entry",
                        d.id
                    );
                }
            }
        }
    }
}
