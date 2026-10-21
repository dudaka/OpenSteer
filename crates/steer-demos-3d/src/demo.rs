//! The 3D demos the host can run, chosen by id, as the 2D `steer_demos::Demo`.

use steer_demos::DemoInfo;

use crate::view::Frame3View;
use crate::{OneTurning3, Pursuit3, SeekFlee3, Wander3};

/// Every 3D demo in sidebar order (after the 2D ones): id and name.
const DEMOS: [(&str, &str); 4] = [
    ("3d-one-turning", "3D One Turning"),
    ("3d-seek-flee", "3D Seek and Flee"),
    ("3d-wander", "3D Wander"),
    ("3d-pursuit", "3D Pursuit and Evasion"),
];

#[derive(Debug, Clone)]
pub enum Demo3 {
    OneTurning(OneTurning3),
    SeekFlee(SeekFlee3),
    Wander(Wander3),
    Pursuit(Pursuit3),
}

impl Demo3 {
    /// A fresh demo in its initial state; `None` for an unknown id.
    pub fn new(id: &str) -> Option<Demo3> {
        match id {
            "3d-one-turning" => Some(Demo3::OneTurning(OneTurning3::default())),
            "3d-seek-flee" => Some(Demo3::SeekFlee(SeekFlee3::default())),
            "3d-wander" => Some(Demo3::Wander(Wander3::default())),
            "3d-pursuit" => Some(Demo3::Pursuit(Pursuit3::default())),
            _ => None,
        }
    }

    pub fn update(&mut self, dt: f32) {
        match self {
            Demo3::OneTurning(d) => d.update(dt),
            Demo3::SeekFlee(d) => d.update(dt),
            Demo3::Wander(d) => d.update(dt),
            Demo3::Pursuit(d) => d.update(dt),
        }
    }

    pub fn view(&self) -> Frame3View {
        match self {
            Demo3::OneTurning(d) => d.view(),
            Demo3::SeekFlee(d) => d.view(),
            Demo3::Wander(d) => d.view(),
            Demo3::Pursuit(d) => d.view(),
        }
    }

    /// One of the demo's `ActionView` buttons; unknown ids are ignored.
    pub fn action(&mut self, id: &str) {
        match self {
            Demo3::OneTurning(d) => d.action(id),
            Demo3::SeekFlee(d) => d.action(id),
            Demo3::Wander(d) => d.action(id),
            Demo3::Pursuit(d) => d.action(id),
        }
    }

    /// Sets one of the demo's `ParamView` sliders; unknown ids are ignored.
    pub fn param(&mut self, id: &str, value: f32) {
        match self {
            Demo3::Wander(d) => d.param(id, value),
            Demo3::Pursuit(d) => d.param(id, value),
            Demo3::OneTurning(_) | Demo3::SeekFlee(_) => {}
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
            assert!(Demo3::new(&d.id).is_some(), "{}", d.id);
        }
        assert!(Demo3::new("seek-flee").is_none());
    }

    /// No color goes unexplained, as in the 2D demos.
    #[test]
    fn every_drawn_color_has_a_legend_entry() {
        for d in catalog() {
            let actions = Demo3::new(&d.id).expect("catalog demo").view().actions;
            let pressed = actions.iter().map(|a| Some(a.id.as_str()));
            for action in [None].into_iter().chain(pressed) {
                let mut demo = Demo3::new(&d.id).expect("catalog demo");
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
