//! "Path Following": pedestrians on a figure eight (`plugins/PedestriansWalkingAnEight.cpp`,
//! without obstacles and neighbors). Each steers with `forward + wander + 0.5 * path force`,
//! the path force from `steerToFollowPath` (directed, turning back at the ends) or
//! `steerToStayOnPath`, both looking 3 s ahead.

use std::f32::consts::TAU;

use steer_core::pathway::Pathway;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{color, label, legend, line, vehicle_view, Trail};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LineView};

const PEDESTRIANS: Range = Range::new(1.0, 50.0, 1.0);
const SEED: u64 = 1;
const LEAD_TIME: f32 = 3.0;

/// `getTestPath`'s figure eight (C++ `x, z`), radius 2.
#[allow(clippy::excessive_precision)]
const EIGHT: [(f32, f32); 16] = [
    (-12.678730011, 0.523285984993),
    (-10.447640419, -3.44138407707),
    (-5.88988399506, -4.1717581749),
    (0.941263973713, 0.350513994694),
    (5.83484792709, 6.56243610382),
    (11.0144147873, 10.175157547),
    (15.9621419907, 8.82364273071),
    (18.697883606, 2.42084693909),
    (16.0552558899, -3.57153511047),
    (10.5450153351, -9.92683887482),
    (5.88374519348, -8.51393127441),
    (3.17790007591, -2.33129906654),
    (1.94371795654, 2.78656601906),
    (-1.04967498779, 5.57114219666),
    (-7.58111476898, 6.13661909103),
    (-12.4111375809, 3.5670940876),
];

#[derive(Debug, Clone)]
struct Pedestrian {
    vehicle: Vehicle,
    /// +1 or -1 along the path.
    direction: f32,
    wander_side: f32,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct PathFollowing {
    path: Pathway,
    pedestrians: Vec<Pedestrian>,
    directed: bool,
    wander: bool,
    rng: fastrand::Rng,
    time: f32,
}

impl Default for PathFollowing {
    fn default() -> Self {
        Self::new(10, true, true)
    }
}

impl PathFollowing {
    fn new(count: usize, directed: bool, wander: bool) -> Self {
        let points: Vec<Vec2> = EIGHT.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
        let mut demo = Self {
            path: Pathway::new(&points, 2.0, false),
            pedestrians: Vec::new(),
            directed,
            wander,
            rng: fastrand::Rng::with_seed(SEED),
            time: 0.0,
        };
        for _ in 0..count {
            let p = demo.random_pedestrian();
            demo.pedestrians.push(p);
        }
        demo
    }

    /// `Pedestrian::reset`: at rest at a random path distance plus a random offset within the
    /// radius, random heading and direction; max speed 2, max force 8, radius 0.5.
    fn random_pedestrian(&mut self) -> Pedestrian {
        let rng = &mut self.rng;
        let along = self.path.point_at(self.path.length() * rng.f32());
        let (angle, r) = (rng.f32() * TAU, rng.f32().sqrt() * self.path.radius);
        let heading = rng.f32() * TAU;
        Pedestrian {
            vehicle: Vehicle {
                position: along + Vec2::new(angle.cos(), angle.sin()) * r,
                forward: Vec2::new(heading.cos(), heading.sin()),
                max_speed: 2.0,
                max_force: 8.0,
                radius: 0.5,
                ..Vehicle::default()
            },
            direction: if rng.bool() { 1.0 } else { -1.0 },
            wander_side: 0.0,
            trail: Trail::default(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let ends = [self.path.points()[0], *self.path.points().last().unwrap()];
        for p in &mut self.pedestrians {
            let v = &p.vehicle;
            let mut force = v.forward;
            if self.wander {
                let rng = &mut self.rng;
                force += steer::wander(v, &mut p.wander_side, dt, &mut || rng.f32());
            }
            force += path_force(v, p.direction, self.directed, &self.path) * 0.5;
            p.vehicle.apply_steering_force(force, dt);
            if self.directed {
                if (p.vehicle.position - ends[0]).length() < self.path.radius {
                    p.direction = 1.0;
                }
                if (p.vehicle.position - ends[1]).length() < self.path.radius {
                    p.direction = -1.0;
                }
            }
            p.trail.record(self.time, p.vehicle.position);
        }
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "directed" => self.directed = !self.directed,
            "wander" => self.wander = !self.wander,
            _ => {}
        }
    }

    /// The pedestrian count restarts the demo.
    pub fn param(&mut self, id: &str, value: f32) {
        if id == "pedestrians" {
            *self = Self::new(
                PEDESTRIANS.clamp(value) as usize,
                self.directed,
                self.wander,
            );
        }
    }

    pub fn view(&self) -> FrameView {
        let mut lines: Vec<LineView> = self
            .path
            .points()
            .windows(2)
            .map(|w| line(w[0], w[1], color::PATH))
            .collect();
        let first = &self.pedestrians[0];
        lines.extend(self.look_ahead(first));
        FrameView {
            time: self.time,
            focus: [3.0, 0.0],
            view_size: 36.0,
            vehicles: self
                .pedestrians
                .iter()
                .map(|p| vehicle_view(&p.vehicle, color::BODY))
                .collect(),
            lines,
            trails: self
                .pedestrians
                .iter()
                .map(|p| p.trail.view(p.vehicle.position))
                .collect(),
            labels: vec![label(
                first.vehicle.position,
                format!("direction {:+.0}", first.direction),
                color::LABEL,
            )],
            legend: legend(&[
                (color::PATH, "path center line (radius 2)"),
                (
                    color::RADIUS,
                    "look-ahead: position in 3 s (labeled pedestrian)",
                ),
                (color::TARGET, "path point it seeks"),
                (color::PUSH, "push: path force x 0.5"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("directed", "Follow in a direction", self.directed),
                button("wander", "Wander", self.wander),
            ],
            params: vec![slider(
                "pedestrians",
                "pedestrians",
                PEDESTRIANS,
                self.pedestrians.len() as f32,
            )],
        }
    }

    /// For one pedestrian: its 3 s prediction, and when the path force acts, the point it seeks
    /// and the push.
    fn look_ahead(&self, p: &Pedestrian) -> Vec<LineView> {
        let v = &p.vehicle;
        let future = v.predict_future_position(LEAD_TIME);
        let mut lines = vec![line(v.position, future, color::RADIUS)];
        let target = if self.directed {
            steer::follow_path_target(v, p.direction, LEAD_TIME, &self.path)
        } else {
            steer::stay_on_path_target(v, LEAD_TIME, &self.path)
        };
        if let Some(t) = target {
            lines.push(line(future, t, color::TARGET));
            let push = path_force(v, p.direction, self.directed, &self.path) * 0.5;
            lines.push(line(v.position, v.position + push, color::PUSH));
        }
        lines
    }
}

fn path_force(v: &Vehicle, direction: f32, directed: bool, path: &Pathway) -> Vec2 {
    if directed {
        steer::follow_path(v, direction, LEAD_TIME, path)
    } else {
        steer::stay_on_path(v, LEAD_TIME, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(demo: &mut PathFollowing, seconds: u32) {
        for _ in 0..seconds * 60 {
            demo.update(1.0 / 60.0);
        }
    }

    fn worst_outside(demo: &PathFollowing) -> f32 {
        demo.pedestrians
            .iter()
            .map(|p| demo.path.map_point(p.vehicle.position).outside)
            .fold(f32::MIN, f32::max)
    }

    #[test]
    fn pedestrians_start_inside_the_path() {
        assert!(worst_outside(&PathFollowing::default()) <= 0.0);
    }

    /// Worst distance outside the path over 60 s, after 10 s of settling.
    fn worst_over_a_minute(directed: bool, wander: bool) -> f32 {
        let mut demo = PathFollowing::new(10, directed, wander);
        run(&mut demo, 10);
        let mut worst = f32::MIN;
        for _ in 0..60 * 60 {
            demo.update(1.0 / 60.0);
            worst = worst.max(worst_outside(&demo));
        }
        worst
    }

    /// Directed following alone keeps everyone inside; the plug-in's wander (weight 1) pushes
    /// them out by up to ~2.3; stay-on-path has no direction, so they walk off the open ends.
    #[test]
    fn only_directed_following_without_wander_stays_inside() {
        assert!(worst_over_a_minute(true, false) < 0.0);
        assert!((worst_over_a_minute(true, true) - 2.326).abs() < 0.01);
        assert!((worst_over_a_minute(false, false) - 1.939).abs() < 0.01);
    }

    #[test]
    fn directed_pedestrians_move_along_the_path() {
        let mut demo = PathFollowing::new(1, true, false);
        demo.pedestrians[0].direction = 1.0;
        run(&mut demo, 5);
        let before = demo
            .path
            .map_point(demo.pedestrians[0].vehicle.position)
            .distance;
        run(&mut demo, 2);
        let after = demo
            .path
            .map_point(demo.pedestrians[0].vehicle.position)
            .distance;
        assert!(after > before, "{before} -> {after}");
    }
}
