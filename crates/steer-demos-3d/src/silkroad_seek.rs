//! "3D Seek on Silkroad": the 3D Seek vehicle (max force 1, max speed 3) on the hills south of
//! Jangan. It seeks on the ground plane (toward the target at its own height, so the push is
//! level, as in OpenSteer's ground plug-ins); after each step the vehicle is set onto the terrain
//! (y = the ground height under it), so its track is the flat seek's. The world is the game's
//! scaled by 1/10, so a region is 192 units and OpenSteer's speeds read as walking pace. Without
//! the game data the ground is flat (y = 0).

use std::sync::Arc;

use steer_core_3d::{steer, Vec3, Vehicle};
use steer_demos::controls::button;
use steer_demos::{color, legend};

use crate::annotation::{
    cross, label, line, point, stem, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::Frame3View;
use crate::world::{data_path, World};

/// Demo units per game unit.
const SCALE: f32 = 0.1;

/// The target's spots (x, z) south of Jangan, in demo units: the corners of a square around a
/// small hill at (80, -150), visited so that two legs cross the hilltop.
const TARGETS: [(f32, f32); 4] = [
    (60.0, -170.0),
    (100.0, -130.0),
    (60.0, -130.0),
    (100.0, -170.0),
];
/// Where the vehicle starts (x, z), facing +z; the first target is behind it, not straight behind.
const START: (f32, f32) = (80.0, -110.0);

#[derive(Debug, Clone)]
pub struct SilkroadSeek3 {
    world: Option<Arc<World>>,
    vehicle: Vehicle,
    target: usize,
    trail: Trail,
    time: f32,
}

impl Default for SilkroadSeek3 {
    fn default() -> Self {
        let world = World::load(data_path()).ok().map(Arc::new);
        let mut demo = Self {
            world,
            vehicle: Vehicle {
                max_force: 1.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            target: 0,
            trail: Trail::default(),
            time: 0.0,
        };
        let (x, z) = START;
        demo.vehicle.position = demo.on_ground(x, z);
        demo
    }
}

impl SilkroadSeek3 {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let force = self.force();
        self.vehicle.apply_steering_force(force, dt);
        let p = self.vehicle.position;
        self.vehicle.position = self.on_ground(p.x, p.z);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn action(&mut self, id: &str) {
        if id == "move_target" {
            self.target = (self.target + 1) % TARGETS.len();
        }
    }

    pub fn view(&self) -> Frame3View {
        let v = &self.vehicle;
        let (p, t) = (v.position, self.target());
        let force = self.force();
        let want = v.velocity() + force;
        let mut lines = vec![
            line(p, p + want, color::WANT),
            line(p + v.velocity(), p + want, color::PUSH),
            stem(p),
            stem(t),
        ];
        lines.extend(cross(t, 0.4, color::TARGET));
        lines.extend(velocity_acceleration(v));
        let ground = match self.world {
            Some(_) => format!("speed {:.2}, ground {:.2}", v.speed, p.y),
            None => format!("no game data at {}: flat ground", data_path().display()),
        };
        Frame3View {
            time: self.time,
            focus: point(p),
            view_size: 22.0,
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines,
            trails: vec![self.trail.view(p)],
            labels: vec![
                label(p, ground, color::LABEL),
                label(t, format!("target, ground {:.2}", t.y), color::TARGET),
            ],
            legend: legend(&[
                (color::WANT, "want: desired velocity"),
                (color::HAVE, "have: current velocity"),
                (color::PUSH, "push = want - have, level"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::TARGET, "target, on the ground"),
                (color::EDGE, "height: drop line to y = 0"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![button("move_target", "Move target", false)],
            params: Vec::new(),
        }
    }

    /// The point (x, z) on the terrain; y = 0 without the world or outside it.
    fn on_ground(&self, x: f32, z: f32) -> Vec3 {
        let height = self
            .world
            .as_ref()
            .and_then(|w| w.terrain_height(x / SCALE, z / SCALE));
        Vec3::new(x, height.map_or(0.0, |h| h * SCALE), z)
    }

    fn target(&self) -> Vec3 {
        let (x, z) = TARGETS[self.target];
        self.on_ground(x, z)
    }

    /// Seek toward the target at the vehicle's own height: a seek on the ground plane, so the
    /// track is the flat one and the terrain only sets the height.
    fn force(&self) -> Vec3 {
        let t = self.target();
        let level = Vec3::new(t.x, self.vehicle.position.y, t.z);
        steer::seek(&self.vehicle, level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo on the game's terrain, None (test skipped) without the game data.
    fn on_terrain() -> Option<SilkroadSeek3> {
        let demo = SilkroadSeek3::default();
        if demo.world.is_none() {
            eprintln!(
                "game Data/ not present at {}: skipped",
                data_path().display()
            );
            return None;
        }
        Some(demo)
    }

    /// Runs 30 s toward each target in turn; the highest ground per leg.
    fn four_legs(demo: &mut SilkroadSeek3, mut each_step: impl FnMut(&SilkroadSeek3)) -> Vec<f32> {
        let mut highest = Vec::new();
        for _ in 0..4 {
            let mut top = f32::MIN;
            for _ in 0..1800 {
                demo.update(1.0 / 60.0);
                top = top.max(demo.vehicle.position.y);
                each_step(demo);
            }
            highest.push(top);
            demo.action("move_target");
        }
        highest
    }

    /// The vehicle seeks on the ground plane, so its track (x, z) is the flat seek's, bit for
    /// bit, over all four legs; only the height differs.
    #[test]
    fn the_terrain_does_not_change_the_track() {
        let Some(mut demo) = on_terrain() else { return };
        let mut flat = SilkroadSeek3 {
            world: None,
            ..demo.clone()
        };
        flat.vehicle.position.y = 0.0;
        let track = |d: &mut SilkroadSeek3| {
            let mut xz = Vec::new();
            four_legs(d, |d| xz.push((d.vehicle.position.x, d.vehicle.position.z)));
            xz
        };
        assert!(track(&mut demo) == track(&mut flat));
    }

    /// The vehicle stands on the terrain: the corners lie at -4.49..-5.52, the diagonal legs
    /// (to the second and fourth targets) cross the hill and reach -0.02 and 0.34.
    #[test]
    fn the_diagonal_legs_cross_the_hill() {
        let Some(mut demo) = on_terrain() else { return };
        let highest = four_legs(&mut demo, |d| {
            let p = d.vehicle.position;
            assert_eq!(p, d.on_ground(p.x, p.z));
        });
        let want = [-3.40, -0.02, -4.40, 0.34];
        for (h, w) in highest.iter().zip(want) {
            assert!((h - w).abs() < 0.01, "{highest:?}");
        }
    }

    /// Without the game data the ground is flat and the label says why.
    #[test]
    fn without_the_world_the_ground_is_flat() {
        let mut demo = SilkroadSeek3 {
            world: None,
            ..SilkroadSeek3::default()
        };
        demo.vehicle.position.y = 0.0;
        four_legs(&mut demo, |d| assert_eq!(d.vehicle.position.y, 0.0));
        assert!(demo.view().labels[0].text.starts_with("no game data"));
    }
}
