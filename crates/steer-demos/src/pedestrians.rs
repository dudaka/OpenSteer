//! "Pedestrians" (`plugins/Pedestrian.cpp`): a crowd follows a path with two rock obstacles.
//! Each step a pedestrian takes the first nonzero of: obstacle avoidance (6 s look-ahead),
//! neighbor avoidance x 10 (3 s), else wander + 0.5 x path following; the two avoidances are
//! each skipped 10% of the time ("leak through"). The plug-in's tilted rectangle obstacle is
//! 3D and left out.

use std::f32::consts::TAU;

use steer_core::neighbors::avoid_neighbors;
use steer_core::obstacle::{avoid_obstacles, Circle, SeenFrom};
use steer_core::pathway::Pathway;
use steer_core::proximity::Grid;
use steer_core::util::interpolate;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, vehicle_view, Trail};
use crate::controls::{slider, Range};
use crate::view::FrameView;

const CROWD: Range = Range::new(1.0, 200.0, 1.0);
const SEED: u64 = 1;
const LEAK_THROUGH: f32 = 0.1;
/// `caLeadTime * maxSpeed * 2`: neighbors looked at for avoidance.
const NEIGHBOR_RADIUS: f32 = 3.0 * 2.0 * 2.0;

#[derive(Debug, Clone)]
struct Walker {
    vehicle: Vehicle,
    /// +1 or -1 along the path.
    direction: f32,
    wander_side: f32,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct Pedestrians {
    path: Pathway,
    rocks: [Circle; 2],
    crowd: Vec<Walker>,
    rng: fastrand::Rng,
    /// Agent-steps steering around a rock / a neighbor, since the start.
    pub obstacle_steps: u32,
    pub neighbor_steps: u32,
    time: f32,
}

impl Default for Pedestrians {
    /// `PedestrianPlugIn::open`: 100 pedestrians.
    fn default() -> Self {
        let mut demo = Self::from_layout(Vec::new());
        for _ in 0..100 {
            let walker = demo.random_walker();
            demo.crowd.push(walker);
        }
        demo
    }
}

/// `getTestPath`'s points (C++ `x, z`).
fn path_points() -> [Vec2; 7] {
    let (size, h) = (30.0, 0.5);
    let (top, gap, out) = (2.0 * size, 1.2 * size, 2.0 * size);
    [
        Vec2::new(h + gap - out, h + top - out),
        Vec2::new(h + gap, h + top),
        Vec2::new(h + gap + top / 2.0, h + top / 2.0),
        Vec2::new(h + gap, h),
        Vec2::new(h, h),
        Vec2::new(h, h + top),
        Vec2::new(h + gap, h + top / 2.0),
    ]
}

/// `Pedestrian::reset`'s speeds: max speed 2, max force 8, radius 0.5, at rest.
fn walker_vehicle(position: Vec2, forward: Vec2) -> Vehicle {
    Vehicle {
        position,
        forward,
        max_speed: 2.0,
        max_force: 8.0,
        radius: 0.5,
        ..Vehicle::default()
    }
}

impl Pedestrians {
    /// A crowd from given positions, headings and path directions.
    pub fn from_layout(walkers: Vec<(Vehicle, f32)>) -> Self {
        let p = path_points();
        let path = Pathway::new(&p, 2.0, false);
        let rocks = [
            Circle::new(interpolate(0.2, p[0], p[1]), 3.0, SeenFrom::Outside),
            Circle::new(interpolate(0.5, p[2], p[3]), 5.0, SeenFrom::Outside),
        ];
        let crowd = walkers
            .into_iter()
            .map(|(v, direction)| Walker {
                vehicle: walker_vehicle(v.position, v.forward),
                direction,
                wander_side: 0.0,
                trail: Trail::default(),
            })
            .collect();
        Self {
            path,
            rocks,
            crowd,
            rng: fastrand::Rng::with_seed(SEED),
            obstacle_steps: 0,
            neighbor_steps: 0,
            time: 0.0,
        }
    }

    pub fn path(&self) -> &Pathway {
        &self.path
    }

    /// Each pedestrian's vehicle and path direction.
    pub fn walkers(&self) -> impl Iterator<Item = (&Vehicle, f32)> {
        self.crowd.iter().map(|w| (&w.vehicle, w.direction))
    }

    pub fn update(&mut self, dt: f32) {
        let mut rng = self.rng.clone();
        self.step(dt, &mut || rng.f32());
        self.rng = rng;
        self.time += dt;
        for w in &mut self.crowd {
            w.trail.record(self.time, w.vehicle.position);
        }
    }

    /// `PedestrianPlugIn::update`: each pedestrian in turn; `random01` draws in the plug-in's
    /// order (obstacle leak, neighbor leak, wander side, wander up).
    pub fn step(&mut self, dt: f32, random01: &mut impl FnMut() -> f32) {
        let mut positions: Vec<Vec2> = self.crowd.iter().map(|w| w.vehicle.position).collect();
        let mut grid = Grid::new(&positions, NEIGHBOR_RADIUS);
        let ends = [self.path.points()[0], *self.path.points().last().unwrap()];
        for i in 0..self.crowd.len() {
            let near = grid.neighbors(&positions, positions[i], NEIGHBOR_RADIUS);
            let force = self.combined_steering(i, &near, dt, random01);
            let w = &mut self.crowd[i];
            w.vehicle.apply_steering_force(force, dt);
            if (w.vehicle.position - ends[0]).length() < self.path.radius {
                w.direction = 1.0;
            }
            if (w.vehicle.position - ends[1]).length() < self.path.radius {
                w.direction = -1.0;
            }
            grid.move_point(i, positions[i], w.vehicle.position);
            positions[i] = w.vehicle.position;
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        if id == "crowd" {
            let target = CROWD.clamp(value) as usize;
            self.crowd.truncate(target);
            while self.crowd.len() < target {
                let walker = self.random_walker();
                self.crowd.push(walker);
            }
        }
    }

    pub fn view(&self) -> FrameView {
        let mut lines: Vec<_> = self
            .path
            .points()
            .windows(2)
            .map(|w| line(w[0], w[1], color::PATH))
            .collect();
        lines.extend(
            self.rocks
                .iter()
                .flat_map(|r| circle(r.center, r.radius, color::OBSTACLE)),
        );
        let steps = (self.time * 60.0).round().max(1.0) * self.crowd.len().max(1) as f32;
        FrameView {
            time: self.time,
            focus: [30.0, 30.0],
            view_size: 80.0,
            vehicles: self
                .crowd
                .iter()
                .map(|w| vehicle_view(&w.vehicle, color::BODY))
                .collect(),
            lines,
            trails: self
                .crowd
                .iter()
                .map(|w| w.trail.view(w.vehicle.position))
                .collect(),
            labels: vec![label(
                Vec2::new(30.0, -8.0),
                format!(
                    "avoiding rocks {:.0}%, neighbors {:.0}% of agent-steps",
                    100.0 * self.obstacle_steps as f32 / steps,
                    100.0 * self.neighbor_steps as f32 / steps
                ),
                color::LABEL,
            )],
            legend: legend(&[
                (color::PATH, "path center line (radius 2)"),
                (color::OBSTACLE, "rock"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: Vec::new(),
            params: vec![slider(
                "crowd",
                "pedestrians",
                CROWD,
                self.crowd.len() as f32,
            )],
        }
    }

    /// `Pedestrian::determineCombinedSteering` over the candidate neighbors `near` (sorted
    /// indices, may include `i`).
    fn combined_steering(
        &mut self,
        i: usize,
        near: &[usize],
        dt: f32,
        random01: &mut impl FnMut() -> f32,
    ) -> Vec2 {
        let v = &self.crowd[i].vehicle;
        let mut force = v.forward;
        let obstacle_avoidance = if LEAK_THROUGH < random01() {
            avoid_obstacles(v, 6.0, &self.rocks)
        } else {
            Vec2::ZERO
        };
        if obstacle_avoidance != Vec2::ZERO {
            self.obstacle_steps += 1;
            return force + obstacle_avoidance;
        }
        let others: Vec<&Vehicle> = near
            .iter()
            .filter(|&&j| j != i)
            .map(|&j| &self.crowd[j].vehicle)
            .collect();
        let collision_avoidance = if LEAK_THROUGH < random01() {
            avoid_neighbors(v, 3.0, &others) * 10.0
        } else {
            Vec2::ZERO
        };
        if collision_avoidance != Vec2::ZERO {
            self.neighbor_steps += 1;
            return force + collision_avoidance;
        }
        let w = &mut self.crowd[i];
        // steerForWander walks side, then up (unused on the ground): two draws.
        let mut wander_draw = || {
            let side = random01();
            random01();
            side
        };
        force += steer::wander(&w.vehicle, &mut w.wander_side, dt, &mut wander_draw);
        force + steer::follow_path(&w.vehicle, w.direction, 3.0, &self.path) * 0.5
    }

    /// `Pedestrian::reset`: a random path distance plus an offset within the radius, random
    /// heading and direction.
    fn random_walker(&mut self) -> Walker {
        let rng = &mut self.rng;
        let along = self.path.point_at(self.path.length() * rng.f32());
        let (angle, r) = (rng.f32() * TAU, rng.f32().sqrt() * self.path.radius);
        let heading = rng.f32() * TAU;
        Walker {
            vehicle: walker_vehicle(
                along + Vec2::new(angle.cos(), angle.sin()) * r,
                Vec2::new(heading.cos(), heading.sin()),
            ),
            direction: if rng.bool() { 1.0 } else { -1.0 },
            wander_side: 0.0,
            trail: Trail::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside_a_rock(demo: &Pedestrians) -> Vec<usize> {
        let inside = |w: &Walker| {
            let p = w.vehicle.position;
            demo.rocks
                .iter()
                .any(|r| (p - r.center).length() < r.radius)
        };
        (0..demo.crowd.len())
            .filter(|&i| inside(&demo.crowd[i]))
            .collect()
    }

    /// The rocks sit on the path and `Pedestrian::reset` does not avoid them, so some start
    /// inside one (6 of 100 here); nobody else walks into one.
    #[test]
    fn nobody_walks_into_a_rock() {
        let mut demo = Pedestrians::default();
        let started_inside = inside_a_rock(&demo);
        assert_eq!(started_inside, [6, 12, 52, 59, 72, 87]);
        for _ in 0..60 * 60 {
            demo.update(1.0 / 60.0);
            let inside = inside_a_rock(&demo);
            assert!(
                inside.iter().all(|i| started_inside.contains(i)),
                "{inside:?}"
            );
        }
        assert!(demo.obstacle_steps > 0 && demo.neighbor_steps > 0);
    }

    #[test]
    fn crowd_slider_adds_and_removes() {
        let mut demo = Pedestrians::default();
        demo.param("crowd", 20.0);
        assert_eq!(demo.crowd.len(), 20);
        demo.param("crowd", 30.0);
        assert_eq!(demo.crowd.len(), 30);
    }
}
