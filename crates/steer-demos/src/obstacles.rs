//! "Obstacle Avoidance": a wanderer in a round arena with rocks (`steerToAvoidObstacles` with
//! `SphereObstacle`s, as `oracle/obstacles.cpp`). When the forward path, widened by the
//! vehicle's radius, meets an obstacle within `min time * speed`, a full sideways push away
//! from the surface replaces the wander push (`Pedestrian.cpp`'s priority).

use steer_core::obstacle::{avoid_obstacles, nearest_intersection, Circle, SeenFrom};
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{
    circle, color, label, legend, line, vehicle_view, velocity_acceleration, Trail,
};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LineView};

const MIN_TIME: Range = Range::new(0.25, 3.0, 0.25);
const SEED: u64 = 1;
const ROCK_RADIUS: f32 = 1.5;

#[derive(Debug, Clone)]
pub struct Obstacles {
    vehicle: Vehicle,
    obstacles: Vec<Circle>,
    wander_side: f32,
    rng: fastrand::Rng,
    min_time: f32,
    /// Steps spent avoiding since the start.
    avoiding: u32,
    trail: Trail,
    time: f32,
}

fn starting_obstacles() -> Vec<Circle> {
    vec![
        Circle::new(Vec2::ZERO, 15.0, SeenFrom::Inside),
        Circle::new(Vec2::new(0.0, 6.0), 2.0, SeenFrom::Outside),
        Circle::new(Vec2::new(-6.0, -2.0), 1.5, SeenFrom::Outside),
        Circle::new(Vec2::new(7.0, -6.0), 3.0, SeenFrom::Outside),
    ]
}

impl Default for Obstacles {
    fn default() -> Self {
        Self {
            vehicle: Vehicle {
                max_force: 5.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            obstacles: starting_obstacles(),
            wander_side: 0.0,
            rng: fastrand::Rng::with_seed(SEED),
            min_time: 1.0,
            avoiding: 0,
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl Obstacles {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let mut force = self.avoidance();
        if force == Vec2::ZERO {
            let rng = &mut self.rng;
            let v = &self.vehicle;
            let wander = steer::wander(v, &mut self.wander_side, dt, &mut || rng.f32());
            force = v.forward + wander * 3.0;
        } else {
            self.avoiding += 1;
        }
        self.vehicle.apply_steering_force(force, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    /// A click drops a rock there.
    pub fn click(&mut self, at: Vec2) {
        self.obstacles
            .push(Circle::new(at, ROCK_RADIUS, SeenFrom::Outside));
    }

    pub fn action(&mut self, id: &str) {
        if id == "clear" {
            self.obstacles = starting_obstacles();
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        if id == "min_time" {
            self.min_time = MIN_TIME.clamp(value);
        }
    }

    pub fn view(&self) -> FrameView {
        let v = &self.vehicle;
        let p = v.position;
        let avoidance = self.avoidance();
        let mut lines: Vec<LineView> = self
            .obstacles
            .iter()
            .flat_map(|o| circle(o.center, o.radius, color::OBSTACLE))
            .collect();
        lines.extend(self.look_ahead());
        if avoidance != Vec2::ZERO {
            lines.push(line(p, p + avoidance, color::PUSH));
        }
        lines.extend(velocity_acceleration(v));
        FrameView {
            time: self.time,
            focus: [0.0, 0.0],
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines,
            trails: vec![self.trail.view(p)],
            labels: vec![label(
                p,
                format!("avoiding {:.0}% of the time", self.avoiding_share() * 100.0),
                color::LABEL,
            )],
            legend: legend(&[
                (color::OBSTACLE, "obstacle (click to add a rock)"),
                (
                    color::RADIUS,
                    "look-ahead: min time x speed, and the hit point",
                ),
                (color::PUSH, "push: avoidance, sideways at max force"),
                (color::HAVE, "have: current velocity"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![button("clear", "Remove added rocks", false)],
            params: vec![slider(
                "min_time",
                "min time to collision",
                MIN_TIME,
                self.min_time,
            )],
        }
    }

    fn avoidance(&self) -> Vec2 {
        avoid_obstacles(&self.vehicle, self.min_time, &self.obstacles)
    }

    /// The corridor the vehicle checks (its width is the vehicle's diameter) and, if the path
    /// meets an obstacle within it, a cross at the hit point.
    fn look_ahead(&self) -> Vec<LineView> {
        let v = &self.vehicle;
        let reach = v.forward * (self.min_time * v.speed);
        let edge = v.side() * v.radius;
        let mut lines = vec![
            line(v.position + edge, v.position + edge + reach, color::RADIUS),
            line(v.position - edge, v.position - edge + reach, color::RADIUS),
        ];
        if let Some(hit) = nearest_intersection(v, &self.obstacles) {
            if hit.distance < self.min_time * v.speed {
                let s = hit.surface_point;
                let d = 0.3;
                lines.push(line(
                    s - Vec2::new(d, d),
                    s + Vec2::new(d, d),
                    color::RADIUS,
                ));
                lines.push(line(
                    s - Vec2::new(d, -d),
                    s + Vec2::new(d, -d),
                    color::RADIUS,
                ));
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

    fn run(demo: &mut Obstacles, steps: u32) {
        for _ in 0..steps {
            demo.update(1.0 / 60.0);
        }
    }

    /// Over two minutes the body never touches a rock (clearance >= its radius 0.5), but its
    /// center crosses the arena wall: OpenSteer widens every obstacle by the vehicle's radius,
    /// so a container's wall is tested at 15.5 (max measured 15.02; shrinking the container
    /// instead keeps it within 14.17).
    #[test]
    fn rocks_keep_a_body_width_but_the_arena_wall_is_crossed() {
        let mut demo = Obstacles::default();
        let mut max_from_center = 0.0f32;
        for _ in 0..7200 {
            demo.update(1.0 / 60.0);
            let p = demo.vehicle.position;
            max_from_center = max_from_center.max(p.length());
            for rock in &demo.obstacles[1..] {
                assert!((p - rock.center).length() - rock.radius > 0.499, "{p:?}");
            }
        }
        assert!((max_from_center - 15.02).abs() < 0.01, "{max_from_center}");
        assert!(demo.avoiding > 0);
    }

    #[test]
    fn click_adds_a_rock_and_clear_removes_it() {
        let mut demo = Obstacles::default();
        demo.click(Vec2::new(3.0, 3.0));
        assert_eq!(demo.obstacles.len(), 5);
        demo.action("clear");
        assert_eq!(demo.obstacles.len(), 4);
    }

    #[test]
    fn longer_look_ahead_avoids_more_often() {
        let mut short = Obstacles::default();
        short.param("min_time", 0.5);
        let mut long = Obstacles::default();
        long.param("min_time", 2.0);
        run(&mut short, 3600);
        run(&mut long, 3600);
        assert!(long.avoiding > short.avoiding);
    }
}
