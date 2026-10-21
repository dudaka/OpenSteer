//! "3D Obstacle Avoidance": a 3D wanderer inside a container sphere with rocks at different
//! heights (`steerToAvoidObstacles` with `SphereObstacle`s, as `oracle/obstacles_3d.cpp`). When
//! the forward path, widened by the vehicle's radius, meets an obstacle within
//! `min time * speed`, a full sideways push away from the surface replaces the wander push
//! (`Pedestrian.cpp`'s priority).

use std::f32::consts::TAU;

use steer_core_3d::obstacle::{avoid_obstacles, nearest_intersection, SeenFrom, Sphere};
use steer_core_3d::steer::{self, WanderState};
use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::{button, slider, Range};
use steer_demos::{color, legend};

use crate::annotation::{
    cross, label, line, sphere, stem, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::{Frame3View, Line3View};

const MIN_TIME: Range = Range::new(0.25, 3.0, 0.25);
const SEED: u64 = 1;
const ROCK_RADIUS: f32 = 1.5;
const ARENA_RADIUS: f32 = 15.0;

/// The oracle's container and rocks.
fn starting_obstacles() -> Vec<Sphere> {
    vec![
        Sphere::new(Vec3::ZERO, ARENA_RADIUS, SeenFrom::Inside),
        Sphere::new(Vec3::new(0.0, 2.0, 6.0), 2.0, SeenFrom::Outside),
        Sphere::new(Vec3::new(-6.0, -3.0, -2.0), 1.5, SeenFrom::Outside),
        Sphere::new(Vec3::new(7.0, 1.0, -6.0), 3.0, SeenFrom::Outside),
    ]
}

#[derive(Debug, Clone)]
pub struct Obstacles3 {
    vehicle: Vehicle,
    obstacles: Vec<Sphere>,
    walk: WanderState,
    rng: fastrand::Rng,
    /// Places added rocks; separate from the wander's numbers so adding one does not change the
    /// walk.
    rock_rng: fastrand::Rng,
    min_time: f32,
    /// Steps spent avoiding since the start.
    avoiding: u32,
    trail: Trail,
    time: f32,
}

impl Default for Obstacles3 {
    fn default() -> Self {
        Self {
            vehicle: Vehicle {
                max_force: 5.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            obstacles: starting_obstacles(),
            walk: WanderState::default(),
            rng: fastrand::Rng::with_seed(SEED),
            rock_rng: fastrand::Rng::with_seed(SEED),
            min_time: 1.0,
            avoiding: 0,
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl Obstacles3 {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let mut force = self.avoidance();
        if force == Vec3::ZERO {
            let rng = &mut self.rng;
            let v = &self.vehicle;
            let wander = steer::wander(v, &mut self.walk, dt, &mut || rng.f32());
            force = v.forward + wander * 3.0;
        } else {
            self.avoiding += 1;
        }
        self.vehicle.apply_steering_force(force, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "add_rock" => {
                let at = self.random_spot();
                self.obstacles
                    .push(Sphere::new(at, ROCK_RADIUS, SeenFrom::Outside));
            }
            "clear" => self.obstacles.truncate(starting_obstacles().len()),
            _ => {}
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        if id == "min_time" {
            self.min_time = MIN_TIME.clamp(value);
        }
    }

    pub fn view(&self) -> Frame3View {
        let v = &self.vehicle;
        let p = v.position;
        let avoidance = self.avoidance();
        let mut lines: Vec<Line3View> = self
            .obstacles
            .iter()
            .flat_map(|o| sphere(o.center, o.radius, color::OBSTACLE))
            .collect();
        lines.extend(self.look_ahead());
        if avoidance != Vec3::ZERO {
            lines.push(line(p, p + avoidance, color::PUSH));
        }
        lines.push(stem(p));
        lines.extend(velocity_acceleration(v));
        Frame3View {
            time: self.time,
            focus: [0.0, 0.0, 0.0],
            view_size: 45.0,
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines,
            trails: vec![self.trail.view(p)],
            labels: vec![label(
                p,
                format!(
                    "avoiding {:.0}% of the time, height {:.2}",
                    self.avoiding_share() * 100.0,
                    p.y
                ),
                color::LABEL,
            )],
            legend: legend(&[
                (color::OBSTACLE, "obstacle: the container and rocks"),
                (
                    color::RADIUS,
                    "look-ahead: min time x speed, and the hit point",
                ),
                (color::PUSH, "push: avoidance, sideways at max force"),
                (color::HAVE, "have: current velocity"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("add_rock", "Add a rock", false),
                button("clear", "Remove added rocks", false),
            ],
            params: vec![slider(
                "min_time",
                "min time to collision",
                MIN_TIME,
                self.min_time,
            )],
        }
    }

    fn avoidance(&self) -> Vec3 {
        avoid_obstacles(&self.vehicle, self.min_time, &self.obstacles)
    }

    /// A random point at most 12 from the center, so the rock sits inside the container.
    fn random_spot(&mut self) -> Vec3 {
        let rng = &mut self.rock_rng;
        let y = rng.f32() * 2.0 - 1.0;
        let angle = rng.f32() * TAU;
        let r = (1.0 - y * y).sqrt();
        Vec3::new(r * angle.cos(), y, r * angle.sin()) * (12.0 * rng.f32().cbrt())
    }

    /// The tube the vehicle checks (its diameter is the vehicle's), as four lines along its
    /// side and up edges, and a cross at the hit point when the path meets an obstacle in it.
    fn look_ahead(&self) -> Vec<Line3View> {
        let v = &self.vehicle;
        let reach = v.forward * (self.min_time * v.speed);
        let mut lines: Vec<Line3View> = [v.side, -v.side, v.up, -v.up]
            .iter()
            .map(|&e| {
                let start = v.position + e * v.radius;
                line(start, start + reach, color::RADIUS)
            })
            .collect();
        if let Some(hit) = nearest_intersection(v, &self.obstacles) {
            if hit.distance < self.min_time * v.speed {
                lines.extend(cross(hit.surface_point, 0.3, color::RADIUS));
            }
        }
        lines
    }

    fn avoiding_share(&self) -> f32 {
        let steps = (self.time * 60.0).round().max(1.0);
        self.avoiding as f32 / steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(demo: &mut Obstacles3, steps: u32) {
        for _ in 0..steps {
            demo.update(1.0 / 60.0);
        }
    }

    /// As in 2D: over two minutes the body never touches a rock (min clearance 0.86, above its
    /// radius 0.5), but its center crosses the container wall (max 15.02 from the center),
    /// since OpenSteer widens the container too by the vehicle's radius. Avoiding 1142 of 7200
    /// steps.
    #[test]
    fn rocks_keep_a_body_width_but_the_container_wall_is_crossed() {
        let mut demo = Obstacles3::default();
        let (mut max_from_center, mut min_clear) = (0.0f32, f32::MAX);
        for _ in 0..7200 {
            demo.update(1.0 / 60.0);
            let p = demo.vehicle.position;
            max_from_center = max_from_center.max(p.length());
            for rock in &demo.obstacles[1..] {
                min_clear = min_clear.min((p - rock.center).length() - rock.radius);
            }
        }
        assert!(
            (max_from_center - 15.022).abs() < 0.001,
            "{max_from_center}"
        );
        assert!((min_clear - 0.860).abs() < 0.001, "{min_clear}");
        assert_eq!(demo.avoiding, 1142);
    }

    #[test]
    fn added_rocks_sit_inside_and_clear_removes_them() {
        let mut demo = Obstacles3::default();
        for _ in 0..10 {
            demo.action("add_rock");
        }
        assert_eq!(demo.obstacles.len(), 14);
        assert!(demo.obstacles[4..]
            .iter()
            .all(|r| r.center.length() <= 12.0));
        demo.action("clear");
        assert_eq!(demo.obstacles, starting_obstacles());
    }

    #[test]
    fn longer_look_ahead_avoids_more_often() {
        let mut short = Obstacles3::default();
        short.param("min_time", 0.5);
        let mut long = Obstacles3::default();
        long.param("min_time", 2.0);
        run(&mut short, 3600);
        run(&mut long, 3600);
        assert!(long.avoiding > short.avoiding);
    }
}
