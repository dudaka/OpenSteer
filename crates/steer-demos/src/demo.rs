//! The demos the host can run, chosen by id. An enum, so dispatch is a plain `match`.

use steer_core::Vec2;

use crate::view::FrameView;
use crate::{Boids, DemoInfo, Obstacles, OneTurning, PathFollowing, Pursuit, SeekFlee, Wander};

/// Every demo in sidebar order (OpenSteerDemo's `selectionOrderSortKey`): id and name.
const DEMOS: [(&str, &str); 7] = [
    ("one-turning", "One Turning Away"),
    ("seek-flee", "Seek and Flee"),
    ("wander", "Wander"),
    ("pursuit", "Pursuit and Evasion"),
    ("obstacles", "Obstacle Avoidance"),
    ("path-following", "Path Following"),
    ("boids", "Boids"),
];

/// The demo selected at start.
pub const DEFAULT_DEMO: &str = "one-turning";

#[derive(Debug, Clone)]
pub enum Demo {
    OneTurning(OneTurning),
    SeekFlee(SeekFlee),
    Wander(Wander),
    Pursuit(Pursuit),
    Obstacles(Obstacles),
    PathFollowing(PathFollowing),
    Boids(Boids),
}

impl Demo {
    /// A fresh demo in its initial state; `None` for an unknown id.
    pub fn new(id: &str) -> Option<Demo> {
        match id {
            "one-turning" => Some(Demo::OneTurning(OneTurning::default())),
            "seek-flee" => Some(Demo::SeekFlee(SeekFlee::default())),
            "wander" => Some(Demo::Wander(Wander::default())),
            "pursuit" => Some(Demo::Pursuit(Pursuit::default())),
            "obstacles" => Some(Demo::Obstacles(Obstacles::default())),
            "path-following" => Some(Demo::PathFollowing(PathFollowing::default())),
            "boids" => Some(Demo::Boids(Boids::default())),
            _ => None,
        }
    }

    pub fn update(&mut self, dt: f32) {
        match self {
            Demo::OneTurning(d) => d.update(dt),
            Demo::SeekFlee(d) => d.update(dt),
            Demo::Wander(d) => d.update(dt),
            Demo::Pursuit(d) => d.update(dt),
            Demo::Obstacles(d) => d.update(dt),
            Demo::PathFollowing(d) => d.update(dt),
            Demo::Boids(d) => d.update(dt),
        }
    }

    pub fn view(&self) -> FrameView {
        match self {
            Demo::OneTurning(d) => d.view(),
            Demo::SeekFlee(d) => d.view(),
            Demo::Wander(d) => d.view(),
            Demo::Pursuit(d) => d.view(),
            Demo::Obstacles(d) => d.view(),
            Demo::PathFollowing(d) => d.view(),
            Demo::Boids(d) => d.view(),
        }
    }

    /// A click on the ground at world point `at`; demos without click input ignore it.
    pub fn click(&mut self, at: Vec2) {
        match self {
            Demo::SeekFlee(d) => d.click(at),
            Demo::Obstacles(d) => d.click(at),
            _ => {}
        }
    }

    /// One of the demo's `ActionView` buttons; unknown ids are ignored.
    pub fn action(&mut self, id: &str) {
        match self {
            Demo::SeekFlee(d) => d.action(id),
            Demo::Wander(d) => d.action(id),
            Demo::Pursuit(d) => d.action(id),
            Demo::Obstacles(d) => d.action(id),
            Demo::PathFollowing(d) => d.action(id),
            Demo::OneTurning(_) | Demo::Boids(_) => {}
        }
    }

    /// Sets one of the demo's `ParamView` sliders; unknown ids are ignored.
    pub fn param(&mut self, id: &str, value: f32) {
        match self {
            Demo::SeekFlee(d) => d.param(id, value),
            Demo::Wander(d) => d.param(id, value),
            Demo::Pursuit(d) => d.param(id, value),
            Demo::Obstacles(d) => d.param(id, value),
            Demo::PathFollowing(d) => d.param(id, value),
            Demo::Boids(d) => d.param(id, value),
            Demo::OneTurning(_) => {}
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
