//! "3D Wander": MultiplePursuit's wanderer without the flattening (as `oracle/wander_3d.cpp`, but
//! with its own random numbers). Push = forward part + `weight * steerForWander`, whose random
//! walks move it sideways and up/down; the forward part is `forward` or `steerForTargetSpeed`.
//! Banking (as Boids) rolls it into its turns.

use steer_core_3d::steer::{self, WanderState};
use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::{button, slider, Range};
use steer_demos::{color, legend, ParamView};

use crate::annotation::{label, line, point, stem, vehicle_view, velocity_acceleration, Trail};
use crate::view::Frame3View;

const TARGET_SPEED: Range = Range::new(0.0, 3.0, 0.1);
const WANDER_WEIGHT: Range = Range::new(0.0, 5.0, 0.25);
/// The same random walk after every reset.
const SEED: u64 = 1;

#[derive(Debug, Clone)]
pub struct Wander3 {
    vehicle: Vehicle,
    walk: WanderState,
    rng: fastrand::Rng,
    use_target_speed: bool,
    target_speed: f32,
    wander_weight: f32,
    trail: Trail,
    time: f32,
}

impl Default for Wander3 {
    /// From rest at the origin, max force 5, max speed 3.
    fn default() -> Self {
        Self {
            vehicle: Vehicle {
                max_force: 5.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            walk: WanderState::default(),
            rng: fastrand::Rng::with_seed(SEED),
            use_target_speed: false,
            target_speed: 1.5,
            wander_weight: 3.0,
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl Wander3 {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let rng = &mut self.rng;
        let wander = steer::wander(&self.vehicle, &mut self.walk, dt, &mut || rng.f32());
        let force = self.forward_push() + wander * self.wander_weight;
        self.vehicle.apply_steering_force(force, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "forward" => self.use_target_speed = false,
            "target_speed" => self.use_target_speed = true,
            "banking" => self.vehicle.banking = !self.vehicle.banking,
            _ => {}
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        match id {
            "target_speed" => self.target_speed = TARGET_SPEED.clamp(value),
            "wander_weight" => self.wander_weight = WANDER_WEIGHT.clamp(value),
            _ => {}
        }
    }

    pub fn view(&self) -> Frame3View {
        let v = &self.vehicle;
        let p = v.position;
        let forward = self.forward_push();
        // The last step's wander force: the walk values along the current side and up.
        let wander = (v.side * self.walk.side + v.up * self.walk.up) * self.wander_weight;
        let mut lines = vec![
            line(p, p + forward + wander, color::PUSH),
            line(p + forward, p + forward + wander, color::WANDER),
            stem(p),
        ];
        lines.extend(velocity_acceleration(v));
        Frame3View {
            time: self.time,
            focus: point(p),
            view_size: 18.0,
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines,
            trails: vec![self.trail.view(p)],
            labels: vec![label(
                p,
                format!(
                    "speed {:.2}, height {:.2}, side {:+.2}, up {:+.2}",
                    v.speed, p.y, self.walk.side, self.walk.up
                ),
                color::LABEL,
            )],
            legend: legend(&[
                (color::PUSH, "push = forward part + wander part"),
                (color::WANDER, "wander: random sideways and up/down part"),
                (color::HAVE, "have: current velocity"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("forward", "Forward push", !self.use_target_speed),
                button("target_speed", "Target speed", self.use_target_speed),
                button("banking", "Banking", v.banking),
            ],
            params: self.params(),
        }
    }

    fn params(&self) -> Vec<ParamView> {
        let mut params = vec![slider(
            "wander_weight",
            "wander weight",
            WANDER_WEIGHT,
            self.wander_weight,
        )];
        if self.use_target_speed {
            params.push(slider(
                "target_speed",
                "target speed",
                TARGET_SPEED,
                self.target_speed,
            ));
        }
        params
    }

    fn forward_push(&self) -> Vec3 {
        if self.use_target_speed {
            steer::target_speed(&self.vehicle, self.target_speed)
        } else {
            self.vehicle.forward
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(demo: &mut Wander3, seconds: u32) {
        for _ in 0..seconds * 60 {
            demo.update(1.0 / 60.0);
        }
    }

    #[test]
    fn wandering_leaves_the_ground_and_banking_toggles() {
        let mut demo = Wander3::default();
        run(&mut demo, 10);
        assert!(demo.vehicle.position.y.abs() > 0.5);
        demo.action("banking");
        assert!(demo.vehicle.banking);
    }

    #[test]
    fn no_wander_stays_on_the_line() {
        let mut demo = Wander3::default();
        demo.param("wander_weight", 0.0);
        run(&mut demo, 5);
        let p = demo.vehicle.position;
        assert_eq!((p.x, p.y), (0.0, 0.0));
    }
}
