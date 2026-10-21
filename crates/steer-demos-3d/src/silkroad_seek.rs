//! "3D Seek on Silkroad": the 3D Seek vehicle (max force 1, max speed 3) in the game world
//! around Jangan, at one of two places: the hills south of the town, or the town's west gate,
//! down a ramp onto the plaza. It seeks on the ground plane (toward the target at its own
//! height, so the push is level, as in OpenSteer's ground plug-ins); after each step it is set
//! onto the walkable surface (terrain or object floor) nearest its height, so its track is the
//! flat seek's. Walls are not modeled yet: where a floor ends it steps down to the next surface.
//! The world is the game's scaled by 1/10, so a region is 192 units and OpenSteer's speeds read as
//! walking pace. Without the game data the ground is flat (y = 0).

use std::sync::Arc;

use steer_core_3d::{steer, Vec3, Vehicle};
use steer_demos::controls::button;
use steer_demos::{color, legend};

use crate::annotation::{
    cross, label, line, point, stem, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::{Frame3View, TerrainView};
use crate::world::{data_path, World};

/// Demo units per game unit.
const SCALE: f32 = 0.1;
/// How far above its feet a walker steps onto a surface (5 game units, the vehicle's radius).
const STEP_UP: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// Four targets around a small hill at (80, -150), visited so that two legs cross the
    /// hilltop; the vehicle starts at (80, -110), facing +z, the first target behind it but not
    /// straight behind.
    Hill,
    /// Outside the west gate; the first target is past the ramp down onto the plaza, the others
    /// are server positions of characters on the plaza (caravan's height test).
    Plaza,
}

impl Place {
    /// Where the vehicle starts and the four targets, (x, z) in demo units.
    fn spots(self) -> ((f32, f32), [(f32, f32); 4]) {
        match self {
            Place::Hill => (
                (80.0, -110.0),
                [
                    (60.0, -170.0),
                    (100.0, -130.0),
                    (60.0, -130.0),
                    (100.0, -170.0),
                ],
            ),
            Place::Plaza => (
                (30.0, 85.4),
                [(70.0, 85.4), (97.4, 100.42), (115.3, 105.7), (101.0, 113.5)],
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SilkroadSeek3 {
    world: Option<Arc<World>>,
    place: Place,
    vehicle: Vehicle,
    /// The four targets, on the highest surface under each.
    targets: [Vec3; 4],
    target: usize,
    trail: Trail,
    time: f32,
}

impl Default for SilkroadSeek3 {
    fn default() -> Self {
        let world = World::load(data_path()).ok().map(Arc::new);
        Self::at(world, Place::Hill)
    }
}

impl SilkroadSeek3 {
    /// A fresh run at `place`, starting on the highest surface.
    fn at(world: Option<Arc<World>>, place: Place) -> Self {
        let mut demo = Self {
            world,
            place,
            vehicle: Vehicle {
                max_force: 1.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            targets: [Vec3::ZERO; 4],
            target: 0,
            trail: Trail::default(),
            time: 0.0,
        };
        let ((x, z), targets) = place.spots();
        demo.vehicle.position = demo.on_top(x, z);
        demo.targets = targets.map(|(x, z)| demo.on_top(x, z));
        demo
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let force = self.force();
        self.vehicle.apply_steering_force(force, dt);
        let p = self.vehicle.position;
        self.vehicle.position = self.on_ground(p.x, p.z, p.y);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "hill" => *self = Self::at(self.world.clone(), Place::Hill),
            "plaza" => *self = Self::at(self.world.clone(), Place::Plaza),
            "move_target" => self.target = (self.target + 1) % self.targets.len(),
            _ => {}
        }
    }

    pub fn view(&self) -> Frame3View {
        let v = &self.vehicle;
        let (p, t) = (v.position, self.targets[self.target]);
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
                (
                    color::GROUND,
                    "ground: the game's terrain (floors not drawn)",
                ),
            ]),
            actions: vec![
                button("hill", "Hill", self.place == Place::Hill),
                button("plaza", "Plaza", self.place == Place::Plaza),
                button("move_target", "Move target", false),
            ],
            params: Vec::new(),
        }
    }

    /// The terrain of the 3x3 regions, a point every 80 game units (8 demo units); None without
    /// the world.
    pub fn terrain(&self) -> Option<TerrainView> {
        let grid = self.world.as_ref()?.terrain_grid(4);
        Some(TerrainView {
            origin: grid.origin.map(|c| c * SCALE),
            spacing: grid.spacing * SCALE,
            size: grid.size as u32,
            heights: grid.heights.into_iter().map(|h| h * SCALE).collect(),
            color: color::GROUND,
        })
    }

    /// The walkable surfaces over (x, z), in demo units; only y = 0 without the world.
    fn surfaces(&self, x: f32, z: f32) -> Vec<f32> {
        match &self.world {
            Some(world) => world
                .surface_heights(x / SCALE, z / SCALE)
                .into_iter()
                .map(|h| h * SCALE)
                .collect(),
            None => vec![0.0],
        }
    }

    /// The point (x, z) on the highest surface at most [`STEP_UP`] above height `y` (a walker
    /// steps onto a bridge just above the terrain and stays on a ramp over it); the nearest
    /// surface when none is that low (a steep climb: walls are not modeled), `y` where there is
    /// none.
    fn on_ground(&self, x: f32, z: f32, y: f32) -> Vec3 {
        let surfaces = self.surfaces(x, z);
        let reachable = surfaces
            .iter()
            .copied()
            .filter(|&h| h <= y + STEP_UP)
            .reduce(f32::max);
        let nearest = || {
            surfaces
                .iter()
                .copied()
                .min_by(|a, b| (a - y).abs().total_cmp(&(b - y).abs()))
        };
        Vec3::new(x, reachable.or_else(nearest).unwrap_or(y), z)
    }

    /// The point (x, z) on the highest surface; y = 0 where there is none.
    fn on_top(&self, x: f32, z: f32) -> Vec3 {
        let top = self.surfaces(x, z).into_iter().reduce(f32::max);
        Vec3::new(x, top.unwrap_or(0.0), z)
    }

    /// Seek toward the target at the vehicle's own height: a seek on the ground plane, so the
    /// track is the flat one and the ground only sets the height.
    fn force(&self) -> Vec3 {
        let t = self.targets[self.target];
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

    /// Runs 30 s toward each target in turn; the lowest and highest ground per leg.
    fn four_legs(
        demo: &mut SilkroadSeek3,
        mut each_step: impl FnMut(&SilkroadSeek3),
    ) -> Vec<(f32, f32)> {
        let mut ranges = Vec::new();
        for _ in 0..4 {
            let (mut low, mut high) = (f32::MAX, f32::MIN);
            for _ in 0..1800 {
                demo.update(1.0 / 60.0);
                low = low.min(demo.vehicle.position.y);
                high = high.max(demo.vehicle.position.y);
                each_step(demo);
            }
            ranges.push((low, high));
            demo.action("move_target");
        }
        ranges
    }

    /// The vehicle seeks on the ground plane, so its track (x, z) is the flat seek's, bit for
    /// bit, over all four legs at both places; only the height differs.
    #[test]
    fn the_ground_does_not_change_the_track() {
        let Some(mut demo) = on_terrain() else { return };
        for place in ["hill", "plaza"] {
            demo.action(place);
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
            assert!(track(&mut demo) == track(&mut flat), "{place}");
        }
    }

    /// The vehicle stands on the terrain: the corners lie at -4.49..-5.52, the diagonal legs
    /// (to the second and fourth targets) cross the hill and reach -0.02 and 0.34.
    #[test]
    fn the_diagonal_legs_cross_the_hill() {
        let Some(mut demo) = on_terrain() else { return };
        let highest = four_legs(&mut demo, |d| {
            let p = d.vehicle.position;
            assert_eq!(p, d.on_ground(p.x, p.z, p.y));
        });
        let want = [-3.40, -0.02, -4.40, 0.34];
        for ((_, h), w) in highest.iter().zip(want) {
            assert!((h - w).abs() < 0.01, "{highest:?}");
        }
    }

    /// At the plaza the vehicle starts on open ground (0), steps 0.10 up onto the gate's floor
    /// (not down into the moat under it, which the terrain follows to -3.3), goes down the ramp
    /// and stays on the plaza's floor (-3.26, the terrain under it is lower) for every leg; the
    /// targets stand on the plaza too. No step goes down more than 0.5.
    #[test]
    fn the_plaza_walk_takes_the_gate_and_the_ramp() {
        let Some(mut demo) = on_terrain() else { return };
        demo.action("plaza");
        assert_eq!(demo.vehicle.position.y, 0.0);
        for t in demo.targets {
            assert!((t.y - -3.26).abs() < 0.005, "{t:?}");
        }
        let mut last = demo.vehicle.position.y;
        let ranges = four_legs(&mut demo, |d| {
            let y = d.vehicle.position.y;
            assert!(last - y < 0.5, "{last} -> {y} at {}", d.time);
            last = y;
        });
        let close = |a: f32, b: f32| (a - b).abs() < 0.005;
        let (low, high) = ranges[0];
        assert!(close(low, -3.26) && close(high, 0.097), "{ranges:?}");
        for &(low, high) in &ranges[1..] {
            assert!(close(low, -3.26) && close(high, -3.26), "{ranges:?}");
        }
    }

    /// The terrain sent to the browser is the ground the vehicle walks on, scaled: 73 x 73
    /// points 8 apart from (-192, -192); the point at (80, -104) on the hill is the terrain
    /// height there.
    #[test]
    fn the_terrain_view_is_the_walked_terrain() {
        let Some(demo) = on_terrain() else { return };
        let t = demo.terrain().unwrap();
        assert_eq!((t.origin, t.spacing, t.size), ([-192.0, -192.0], 8.0, 73));
        let (column, row) = (34, 11);
        let (x, z) = (-192.0 + 8.0 * column as f32, -192.0 + 8.0 * row as f32);
        assert_eq!((x, z), (80.0, -104.0));
        assert_eq!(demo.surfaces(x, z), vec![t.heights[row * 73 + column]]);
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
        assert_eq!(demo.terrain(), None);
    }
}
