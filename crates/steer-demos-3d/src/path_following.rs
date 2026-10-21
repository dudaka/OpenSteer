//! "3D Path Following": walkers on PedestriansWalkingAnEight's figure eight raised into a roller
//! coaster (as `oracle/path_following_3d.cpp`), radius 2, so the path is a tube. Each steers with
//! `forward + wander + 0.5 * path force`, the path force from `steerToFollowPath` (directed,
//! turning back at the ends) or `steerToStayOnPath`, both looking 3 s ahead; unlike the plug-in,
//! the force is not flattened.

use std::f32::consts::TAU;

use steer_core_3d::pathway::Pathway;
use steer_core_3d::steer::{self, WanderState};
use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::{button, slider, Range};
use steer_demos::{color, legend};

use crate::annotation::{label, line, ring, stem, vehicle_view, Trail};
use crate::view::{Frame3View, Line3View};

const PEDESTRIANS: Range = Range::new(1.0, 50.0, 1.0);
const SEED: u64 = 1;
const LEAD_TIME: f32 = 3.0;

/// `getTestPath`'s figure eight (C++ `x, z`) with heights `4 sin(i * 45 degrees)`.
#[allow(clippy::excessive_precision)]
const COASTER: [Vec3; 16] = [
    Vec3::new(-12.678730011, 0.0, 0.523285984993),
    Vec3::new(-10.447640419, 2.8, -3.44138407707),
    Vec3::new(-5.88988399506, 4.0, -4.1717581749),
    Vec3::new(0.941263973713, 2.8, 0.350513994694),
    Vec3::new(5.83484792709, 0.0, 6.56243610382),
    Vec3::new(11.0144147873, -2.8, 10.175157547),
    Vec3::new(15.9621419907, -4.0, 8.82364273071),
    Vec3::new(18.697883606, -2.8, 2.42084693909),
    Vec3::new(16.0552558899, -0.0, -3.57153511047),
    Vec3::new(10.5450153351, 2.8, -9.92683887482),
    Vec3::new(5.88374519348, 4.0, -8.51393127441),
    Vec3::new(3.17790007591, 2.8, -2.33129906654),
    Vec3::new(1.94371795654, 0.0, 2.78656601906),
    Vec3::new(-1.04967498779, -2.8, 5.57114219666),
    Vec3::new(-7.58111476898, -4.0, 6.13661909103),
    Vec3::new(-12.4111375809, -2.8, 3.5670940876),
];

#[derive(Debug, Clone)]
struct Walker {
    vehicle: Vehicle,
    /// +1 or -1 along the path.
    direction: f32,
    walk: WanderState,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct PathFollowing3 {
    path: Pathway,
    walkers: Vec<Walker>,
    directed: bool,
    wander: bool,
    rng: fastrand::Rng,
    time: f32,
}

impl Default for PathFollowing3 {
    fn default() -> Self {
        Self::new(10, true, true)
    }
}

/// A uniformly random unit vector.
fn random_direction(rng: &mut fastrand::Rng) -> Vec3 {
    let y = rng.f32() * 2.0 - 1.0;
    let angle = rng.f32() * TAU;
    let r = (1.0 - y * y).sqrt();
    Vec3::new(r * angle.cos(), y, r * angle.sin())
}

impl PathFollowing3 {
    fn new(count: usize, directed: bool, wander: bool) -> Self {
        let mut demo = Self {
            path: Pathway::new(&COASTER, 2.0, false),
            walkers: Vec::new(),
            directed,
            wander,
            rng: fastrand::Rng::with_seed(SEED),
            time: 0.0,
        };
        for _ in 0..count {
            let w = demo.random_walker();
            demo.walkers.push(w);
        }
        demo
    }

    /// `Pedestrian::reset` in 3D: at rest at a random path distance plus a random offset inside
    /// the tube, random heading and direction; max speed 2, max force 8, radius 0.5.
    fn random_walker(&mut self) -> Walker {
        let along = self.path.point_at(self.path.length() * self.rng.f32());
        let offset = random_direction(&mut self.rng) * (self.rng.f32().cbrt() * self.path.radius);
        let mut vehicle = Vehicle {
            position: along + offset,
            max_speed: 2.0,
            max_force: 8.0,
            ..Vehicle::default()
        };
        vehicle.regenerate_basis(random_direction(&mut self.rng));
        Walker {
            vehicle,
            direction: if self.rng.bool() { 1.0 } else { -1.0 },
            walk: WanderState::default(),
            trail: Trail::default(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let ends = [self.path.points()[0], *self.path.points().last().unwrap()];
        for w in &mut self.walkers {
            let v = &w.vehicle;
            let mut force = v.forward;
            if self.wander {
                let rng = &mut self.rng;
                force += steer::wander(v, &mut w.walk, dt, &mut || rng.f32());
            }
            force += path_force(v, w.direction, self.directed, &self.path) * 0.5;
            w.vehicle.apply_steering_force(force, dt);
            if self.directed {
                if (w.vehicle.position - ends[0]).length() < self.path.radius {
                    w.direction = 1.0;
                }
                if (w.vehicle.position - ends[1]).length() < self.path.radius {
                    w.direction = -1.0;
                }
            }
            w.trail.record(self.time, w.vehicle.position);
        }
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "directed" => self.directed = !self.directed,
            "wander" => self.wander = !self.wander,
            _ => {}
        }
    }

    /// The walker count restarts the demo.
    pub fn param(&mut self, id: &str, value: f32) {
        if id == "pedestrians" {
            *self = Self::new(
                PEDESTRIANS.clamp(value) as usize,
                self.directed,
                self.wander,
            );
        }
    }

    pub fn view(&self) -> Frame3View {
        let points = self.path.points();
        let mut lines: Vec<Line3View> = points
            .windows(2)
            .map(|w| line(w[0], w[1], color::PATH))
            .collect();
        for (i, &p) in points.iter().enumerate() {
            let (a, b) = if i + 1 < points.len() {
                (p, points[i + 1])
            } else {
                (points[i - 1], p)
            };
            lines.extend(ring(p, (b - a).normalize(), self.path.radius, color::PATH));
        }
        let first = &self.walkers[0];
        lines.extend(self.look_ahead(first));
        lines.push(stem(first.vehicle.position));
        Frame3View {
            time: self.time,
            focus: [3.0, 0.0, 0.0],
            view_size: 45.0,
            vehicles: self
                .walkers
                .iter()
                .map(|w| vehicle_view(&w.vehicle, color::BODY))
                .collect(),
            lines,
            trails: self
                .walkers
                .iter()
                .map(|w| w.trail.view(w.vehicle.position))
                .collect(),
            labels: vec![label(
                first.vehicle.position,
                format!(
                    "direction {:+.0}, height {:.2}",
                    first.direction, first.vehicle.position.y
                ),
                color::LABEL,
            )],
            legend: legend(&[
                (color::PATH, "path center line and tube (radius 2)"),
                (
                    color::RADIUS,
                    "look-ahead: position in 3 s (labeled walker)",
                ),
                (color::TARGET, "path point it seeks"),
                (color::PUSH, "push: path force x 0.5"),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("directed", "Follow in a direction", self.directed),
                button("wander", "Wander", self.wander),
            ],
            params: vec![slider(
                "pedestrians",
                "walkers",
                PEDESTRIANS,
                self.walkers.len() as f32,
            )],
        }
    }

    /// For one walker: its 3 s prediction, and when the path force acts, the point it seeks and
    /// the push.
    fn look_ahead(&self, w: &Walker) -> Vec<Line3View> {
        let v = &w.vehicle;
        let future = v.predict_future_position(LEAD_TIME);
        let mut lines = vec![line(v.position, future, color::RADIUS)];
        let target = if self.directed {
            steer::follow_path_target(v, w.direction, LEAD_TIME, &self.path)
        } else {
            steer::stay_on_path_target(v, LEAD_TIME, &self.path)
        };
        if let Some(t) = target {
            lines.push(line(future, t, color::TARGET));
            let push = path_force(v, w.direction, self.directed, &self.path) * 0.5;
            lines.push(line(v.position, v.position + push, color::PUSH));
        }
        lines
    }
}

fn path_force(v: &Vehicle, direction: f32, directed: bool, path: &Pathway) -> Vec3 {
    if directed {
        steer::follow_path(v, direction, LEAD_TIME, path)
    } else {
        steer::stay_on_path(v, LEAD_TIME, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(demo: &mut PathFollowing3, seconds: u32) {
        for _ in 0..seconds * 60 {
            demo.update(1.0 / 60.0);
        }
    }

    fn worst_outside(demo: &PathFollowing3) -> f32 {
        demo.walkers
            .iter()
            .map(|w| demo.path.map_point(w.vehicle.position).outside)
            .fold(f32::MIN, f32::max)
    }

    #[test]
    fn walkers_start_inside_the_tube() {
        assert!(worst_outside(&PathFollowing3::default()) <= 0.0);
    }

    /// Worst distance outside the tube over 60 s, after 10 s of settling.
    fn worst_over_a_minute(directed: bool, wander: bool) -> f32 {
        let mut demo = PathFollowing3::new(10, directed, wander);
        run(&mut demo, 10);
        let mut worst = f32::MIN;
        for _ in 0..60 * 60 {
            demo.update(1.0 / 60.0);
            worst = worst.max(worst_outside(&demo));
        }
        worst
    }

    /// Worst distance outside the tube over 60 s (10 walkers, after 10 s): directed 0.602 (in
    /// 2D the directed walkers never left the band), directed + wander 1.705, stay on path 0.915,
    /// stay on path + wander 3.349.
    #[test]
    fn excursions_from_the_tube() {
        let cases = [
            (true, false, 0.602),
            (true, true, 1.705),
            (false, false, 0.915),
            (false, true, 3.349),
        ];
        for (directed, wander, want) in cases {
            let got = worst_over_a_minute(directed, wander);
            assert!((got - want).abs() < 0.001, "{directed} {wander}: {got}");
        }
    }
}
