//! The 3D demos the host can run, chosen by id, as the 2D `steer_demos::Demo`.

use steer_demos::DemoInfo;

use crate::ground::Ground;
use crate::view::{Frame3View, TerrainView};
use crate::{
    Boids3, Containment3, Crossing3, LeaderFollowing3, Obstacles3, OneTurning3, PathFollowing3,
    Pursuit3, SeekFlee3, SilkroadSeek3, WallFollowing3, Wander3,
};

/// Every 3D demo in sidebar order (after the 2D ones): id and name.
const DEMOS: [(&str, &str); 15] = [
    ("3d-one-turning", "3D One Turning"),
    ("3d-seek-flee", "3D Seek and Flee"),
    ("3d-wander", "3D Wander"),
    ("3d-pursuit", "3D Pursuit and Evasion"),
    ("3d-obstacles", "3D Obstacle Avoidance"),
    ("3d-path-following", "3D Path Following"),
    ("3d-boids", "3D Boids"),
    ("3d-crossing", "3D Crossing"),
    ("3d-capture-the-flag", "3D Capture the Flag (ground)"),
    ("3d-pedestrians", "3D Pedestrians (ground)"),
    ("3d-soccer", "3D Soccer (ground)"),
    ("3d-leader-following", "3D Leader Following"),
    ("3d-wall-following", "3D Wall Following"),
    ("3d-containment", "3D Containment"),
    ("3d-silkroad-seek", "3D Seek on Silkroad"),
];

#[derive(Debug, Clone)]
pub enum Demo3 {
    OneTurning(OneTurning3),
    SeekFlee(SeekFlee3),
    Wander(Wander3),
    Pursuit(Pursuit3),
    Obstacles(Obstacles3),
    PathFollowing(PathFollowing3),
    Boids(Boids3),
    Crossing(Crossing3),
    /// A ground-bound 2D demo in the 3D view.
    Ground(Ground),
    LeaderFollowing(LeaderFollowing3),
    WallFollowing(WallFollowing3),
    Containment(Containment3),
    SilkroadSeek(SilkroadSeek3),
}

impl Demo3 {
    /// A fresh demo in its initial state; `None` for an unknown id.
    pub fn new(id: &str) -> Option<Demo3> {
        match id {
            "3d-one-turning" => Some(Demo3::OneTurning(OneTurning3::default())),
            "3d-seek-flee" => Some(Demo3::SeekFlee(SeekFlee3::default())),
            "3d-wander" => Some(Demo3::Wander(Wander3::default())),
            "3d-pursuit" => Some(Demo3::Pursuit(Pursuit3::default())),
            "3d-obstacles" => Some(Demo3::Obstacles(Obstacles3::default())),
            "3d-path-following" => Some(Demo3::PathFollowing(PathFollowing3::default())),
            "3d-boids" => Some(Demo3::Boids(Boids3::default())),
            "3d-crossing" => Some(Demo3::Crossing(Crossing3::default())),
            "3d-capture-the-flag" | "3d-pedestrians" | "3d-soccer" => {
                Ground::new(&id["3d-".len()..]).map(Demo3::Ground)
            }
            "3d-leader-following" => Some(Demo3::LeaderFollowing(LeaderFollowing3::default())),
            "3d-wall-following" => Some(Demo3::WallFollowing(WallFollowing3::default())),
            "3d-containment" => Some(Demo3::Containment(Containment3::default())),
            "3d-silkroad-seek" => Some(Demo3::SilkroadSeek(SilkroadSeek3::default())),
            _ => None,
        }
    }

    pub fn update(&mut self, dt: f32) {
        match self {
            Demo3::OneTurning(d) => d.update(dt),
            Demo3::SeekFlee(d) => d.update(dt),
            Demo3::Wander(d) => d.update(dt),
            Demo3::Pursuit(d) => d.update(dt),
            Demo3::Obstacles(d) => d.update(dt),
            Demo3::PathFollowing(d) => d.update(dt),
            Demo3::Boids(d) => d.update(dt),
            Demo3::Crossing(d) => d.update(dt),
            Demo3::Ground(d) => d.update(dt),
            Demo3::LeaderFollowing(d) => d.update(dt),
            Demo3::WallFollowing(d) => d.update(dt),
            Demo3::Containment(d) => d.update(dt),
            Demo3::SilkroadSeek(d) => d.update(dt),
        }
    }

    pub fn view(&self) -> Frame3View {
        match self {
            Demo3::OneTurning(d) => d.view(),
            Demo3::SeekFlee(d) => d.view(),
            Demo3::Wander(d) => d.view(),
            Demo3::Pursuit(d) => d.view(),
            Demo3::Obstacles(d) => d.view(),
            Demo3::PathFollowing(d) => d.view(),
            Demo3::Boids(d) => d.view(),
            Demo3::Crossing(d) => d.view(),
            Demo3::Ground(d) => d.view(),
            Demo3::LeaderFollowing(d) => d.view(),
            Demo3::WallFollowing(d) => d.view(),
            Demo3::Containment(d) => d.view(),
            Demo3::SilkroadSeek(d) => d.view(),
        }
    }

    /// The ground to draw under the demo, if it has one; asked once when the demo is chosen.
    pub fn terrain(&self) -> Option<TerrainView> {
        match self {
            Demo3::SilkroadSeek(d) => d.terrain(),
            _ => None,
        }
    }

    /// One of the demo's `ActionView` buttons; unknown ids are ignored.
    pub fn action(&mut self, id: &str) {
        match self {
            Demo3::OneTurning(d) => d.action(id),
            Demo3::SeekFlee(d) => d.action(id),
            Demo3::Wander(d) => d.action(id),
            Demo3::Pursuit(d) => d.action(id),
            Demo3::Obstacles(d) => d.action(id),
            Demo3::PathFollowing(d) => d.action(id),
            Demo3::Boids(_) => {}
            Demo3::Crossing(d) => d.action(id),
            Demo3::Ground(d) => d.action(id),
            Demo3::SilkroadSeek(d) => d.action(id),
            Demo3::LeaderFollowing(_) | Demo3::WallFollowing(_) | Demo3::Containment(_) => {}
        }
    }

    /// Sets one of the demo's `ParamView` sliders; unknown ids are ignored.
    pub fn param(&mut self, id: &str, value: f32) {
        match self {
            Demo3::Wander(d) => d.param(id, value),
            Demo3::Pursuit(d) => d.param(id, value),
            Demo3::Obstacles(d) => d.param(id, value),
            Demo3::PathFollowing(d) => d.param(id, value),
            Demo3::Boids(d) => d.param(id, value),
            Demo3::Crossing(d) => d.param(id, value),
            Demo3::Ground(d) => d.param(id, value),
            Demo3::LeaderFollowing(d) => d.param(id, value),
            Demo3::WallFollowing(d) => d.param(id, value),
            Demo3::Containment(d) => d.param(id, value),
            Demo3::SeekFlee(d) => d.param(id, value),
            Demo3::OneTurning(_) | Demo3::SilkroadSeek(_) => {}
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
