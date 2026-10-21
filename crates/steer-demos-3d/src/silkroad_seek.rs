//! "3D Seek on Silkroad": a walker with Pedestrian.cpp's limits (max force 8, max speed 2) in the game world
//! around Jangan, at one of two places: the hills south of the town, or the town's west gate,
//! down a ramp onto the plaza. It seeks on the ground plane (toward the target at its own
//! height, so the push is level, as in OpenSteer's ground plug-ins); after each step it is set
//! onto the walkable surface (terrain or object floor) nearest its height. The walls of its own
//! floor near it (the navmesh's blocked edges within a step of its height) are OpenSteer
//! rectangles, avoided first as in Pedestrian.cpp, the push flattened as in the ground plug-ins.
//! The world is the game's scaled by 1/10, so a region is 192 units and OpenSteer's speeds read
//! as walking pace. Without the game data the ground is flat (y = 0) and there are no walls.

use std::sync::Arc;

use steer_core_3d::obstacle::{avoid_obstacles, Rectangle, SeenFrom};
use steer_core_3d::{steer, Vec3, Vehicle};
use steer_demos::controls::button;
use steer_demos::{color, legend};

use crate::annotation::{
    cross, label, line, point, stem, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::{Frame3View, TerrainView};
use crate::world::{data_path, World};
use silkroad_navmesh::walls::Wall;

/// Demo units per game unit.
const SCALE: f32 = 0.1;
/// How far above its feet a walker steps onto a surface (5 game units, the vehicle's radius).
const STEP_UP: f32 = 0.5;
/// Avoidance looks this many seconds ahead (`minTimeToCollision`, Pedestrian.cpp's `oTime`).
const LOOK_AHEAD: f32 = 6.0;
/// Walls within this distance are obstacles: the look-ahead at full speed (2) plus the vehicle's
/// radius.
const WALL_REACH: f32 = 12.5;
/// Walls within this distance are drawn.
const WALL_SHOWN: f32 = 20.0;
/// A wall's rectangle reaches this far above and below its floor.
const WALL_HEIGHT: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// Four targets around a small hill at (80, -150), visited so that two legs cross the
    /// hilltop; the vehicle starts at (80, -110), facing +z, the first target behind it but not
    /// straight behind.
    Hill,
    /// Outside the west gate; the first target is past the ramp down onto the plaza, the second
    /// straight across the hole in the plaza's floor (x 880..1070 game units along z = 854), the
    /// others server positions of characters on the plaza (caravan's height test).
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
                [(70.0, 85.4), (115.3, 85.4), (115.3, 105.7), (97.4, 100.42)],
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
                // Pedestrian.cpp's: with the 3D Seek vehicle's (1, 3) it turns on a radius of
                // about 9 and runs into walls it has been avoiding for 2.4 units.
                max_force: 8.0,
                max_speed: 2.0,
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
        for (a, b) in self.walls_near(WALL_SHOWN) {
            let up = Vec3::new(0.0, 1.0, 0.0);
            lines.push(line(a, b, color::OBSTACLE));
            lines.push(line(a + up, b + up, color::OBSTACLE));
        }
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
                (color::OBSTACLE, "walls of the walker's floor (navmesh)"),
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

    /// The walls of the walker's floor within `reach` on the ground plane, in demo units: those
    /// whose height range (its two ends) holds the walker's height, give or take a step.
    fn walls_near(&self, reach: f32) -> Vec<(Vec3, Vec3)> {
        let Some(world) = &self.world else {
            return Vec::new();
        };
        let p = self.vehicle.position;
        let scaled = |[x, y, z]: [f32; 3]| Vec3::new(x, y, z) * SCALE;
        world
            .walls()
            .iter()
            .map(|w: &Wall| (scaled(w.a), scaled(w.b)))
            .filter(|&(a, b)| {
                let (low, high) = (a.y.min(b.y), a.y.max(b.y));
                p.y >= low - STEP_UP && p.y <= high + STEP_UP && flat_distance(p, a, b) < reach
            })
            .collect()
    }

    /// Avoid the walls of its floor first, the push made level; otherwise seek toward the target
    /// at the vehicle's own height (a seek on the ground plane).
    fn force(&self) -> Vec3 {
        let walls: Vec<Rectangle> = (self.walls_near(WALL_REACH).into_iter())
            .filter_map(|(a, b)| rectangle(a, b))
            .collect();
        let avoid = avoid_obstacles(&self.vehicle, LOOK_AHEAD, &walls).set_y_to_zero();
        if avoid != Vec3::ZERO {
            return avoid;
        }
        let t = self.targets[self.target];
        let level = Vec3::new(t.x, self.vehicle.position.y, t.z);
        steer::seek(&self.vehicle, level)
    }
}

/// The distance from `p` to the segment `a`-`b` on the ground plane (x, z).
fn flat_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let (p, a, b) = (p.set_y_to_zero(), a.set_y_to_zero(), b.set_y_to_zero());
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    if t.is_nan() {
        return (p - a).length();
    }
    (p - (a + ab * t)).length()
}

/// A wall from `a` to `b` as an upright rectangle seen from both sides: as wide as the wall,
/// [`WALL_HEIGHT`] above and below its middle; None for a wall with no width on the ground.
fn rectangle(a: Vec3, b: Vec3) -> Option<Rectangle> {
    let along = (b - a).set_y_to_zero();
    let width = along.length();
    if width == 0.0 {
        return None;
    }
    let side = along / width;
    let up = Vec3::new(0.0, 1.0, 0.0);
    Some(Rectangle::new(
        width,
        2.0 * WALL_HEIGHT,
        side,
        up,
        side.cross(up),
        (a + b) * 0.5,
        SeenFrom::Both,
    ))
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

    /// What a run of four 40 s legs (one per target) did.
    #[derive(Debug, Default)]
    struct Run {
        /// Per leg: the step it came within 1 of its target, the lowest and highest ground, and
        /// the steps spent avoiding walls.
        arrived: [Option<u32>; 4],
        ground: [(f32, f32); 4],
        avoiding: [u32; 4],
        /// The first step avoiding a wall, and the first where (x, z) left a flat run's.
        first_avoid: Option<u32>,
        first_off_flat: Option<u32>,
        /// Steps with the body (radius 0.5) over a wall of its floor; the closest approach.
        overlapping: u32,
        closest: f32,
        /// The largest drop in one step.
        drop: f32,
    }

    /// Runs `demo` four legs, next to a copy on flat ground without walls.
    fn four_legs(mut demo: SilkroadSeek3) -> Run {
        let mut flat = SilkroadSeek3 {
            world: None,
            ..demo.clone()
        };
        flat.vehicle.position.y = 0.0;
        let mut run = Run {
            closest: f32::MAX,
            ..Run::default()
        };
        for leg in 0..4 {
            run.ground[leg] = (f32::MAX, f32::MIN);
            for step in 1..=2400 {
                let n = leg as u32 * 2400 + step;
                let (p, t) = (demo.vehicle.position, demo.targets[demo.target]);
                if demo.force() != steer::seek(&demo.vehicle, Vec3::new(t.x, p.y, t.z)) {
                    run.avoiding[leg] += 1;
                    run.first_avoid.get_or_insert(n);
                }
                demo.update(1.0 / 60.0);
                flat.update(1.0 / 60.0);
                let (q, f) = (demo.vehicle.position, flat.vehicle.position);
                if (q.x, q.z) != (f.x, f.z) {
                    run.first_off_flat.get_or_insert(n);
                }
                let wall = (demo.walls_near(WALL_REACH).into_iter())
                    .map(|(a, b)| flat_distance(q, a, b))
                    .fold(f32::MAX, f32::min);
                run.overlapping += u32::from(wall < 0.5);
                run.closest = run.closest.min(wall);
                run.drop = run.drop.max(p.y - q.y);
                let (low, high) = &mut run.ground[leg];
                (*low, *high) = (low.min(q.y), high.max(q.y));
                if run.arrived[leg].is_none() && (t - q).set_y_to_zero().length() < 1.0 {
                    run.arrived[leg] = Some(step);
                }
            }
            demo.action("move_target");
            flat.action("move_target");
        }
        run
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.001
    }

    /// The walker seeks on the ground plane, so its track (x, z) is the flat seek's, bit for
    /// bit, until it first avoids a wall: at the hill's slab (step 5257, leg 3) and at the
    /// plaza's hole (step 2655, leg 2).
    #[test]
    fn the_track_is_the_flat_seek_until_a_wall_is_avoided() {
        let Some(demo) = on_terrain() else { return };
        for (place, step) in [("hill", 5257), ("plaza", 2655)] {
            let mut demo = demo.clone();
            demo.action(place);
            let run = four_legs(demo);
            assert_eq!(
                (run.first_avoid, run.first_off_flat),
                (Some(step), Some(step)),
                "{place}"
            );
        }
    }

    /// On the hill the walker reaches every target; the diagonal legs (to the second and fourth
    /// targets) cross the hilltop (highest ground 0.049 and 0.454); it avoids walls only on leg
    /// 3, where the straight line crosses a small sunken slab near (80, -130) (124 steps), and
    /// its body overlaps the slab's walls on 40 steps (closest 0.363).
    #[test]
    fn the_hill_legs_cross_the_hilltop_and_skirt_a_slab() {
        let Some(mut demo) = on_terrain() else { return };
        demo.action("hill");
        let run = four_legs(demo);
        assert!(run.arrived.iter().all(Option::is_some), "{run:?}");
        for ((_, high), want) in run.ground.iter().zip([-3.465, 0.049, -4.392, 0.454]) {
            assert!(close(*high, want), "{run:?}");
        }
        assert_eq!((run.avoiding, run.overlapping), ([0, 0, 124, 0], 40));
        assert!(close(run.closest, 0.363), "{run:?}");
    }

    /// At the plaza the walker starts on open ground (0), steps 0.10 up onto the gate's floor
    /// (not down into the moat under it), goes down the ramp onto the plaza's floor (-3.26), and
    /// stays on it for every leg: on leg 2, aimed straight across the hole in the floor, it
    /// avoids the hole's walls for 565 steps instead of dropping 2.2 into it, and still reaches
    /// the far side. No step goes down more than 0.5. Its body overlaps a wall on 299 steps, and
    /// once its center comes within 0.002 of a corner of the hole: avoidance only looks along
    /// its heading, so passing a corner, the seek pulls it in.
    #[test]
    fn the_plaza_walk_goes_around_the_hole() {
        let Some(mut demo) = on_terrain() else { return };
        demo.action("plaza");
        assert_eq!(demo.vehicle.position.y, 0.0);
        for t in demo.targets {
            assert!((t.y - -3.26).abs() < 0.005, "{t:?}");
        }
        let run = four_legs(demo);
        assert!(run.arrived.iter().all(Option::is_some), "{run:?}");
        assert!(close(run.ground[0].0, -3.261) && close(run.ground[0].1, 0.097));
        for &(low, high) in &run.ground[1..] {
            assert!(close(low, -3.261) && close(high, -3.261), "{run:?}");
        }
        assert!(run.drop < 0.5, "{run:?}");
        assert_eq!((run.avoiding, run.overlapping), ([0, 565, 0, 0], 299));
        assert!(run.closest > 0.0 && run.closest < 0.002, "{run:?}");
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

    /// Without the game data the ground is flat, there are no walls, and the label says why.
    #[test]
    fn without_the_world_the_ground_is_flat() {
        let mut demo = SilkroadSeek3 {
            world: None,
            ..SilkroadSeek3::default()
        };
        demo.vehicle.position.y = 0.0;
        let run = four_legs(demo.clone());
        assert!(run.ground.iter().all(|&g| g == (0.0, 0.0)), "{run:?}");
        assert_eq!((run.avoiding, run.first_off_flat), ([0; 4], None));
        assert!(demo.view().labels[0].text.starts_with("no game data"));
        assert_eq!(demo.terrain(), None);
    }
}
