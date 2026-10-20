//! "Wander": MultiplePursuit's wanderer (`plugins/MultiplePursuit.cpp`, `MpWanderer`) alone.
//! Its push is a forward force plus three times `steerForWander`'s random sideways force. The
//! forward force is `forward` (as `MpWanderer`) or `steerForTargetSpeed`.

use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{
    color, label, legend, line, point, vehicle_view, velocity_acceleration, Trail,
};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, ParamView};

const TARGET_SPEED: Range = Range::new(0.0, 3.0, 0.1);
const WANDER_WEIGHT: Range = Range::new(0.0, 5.0, 0.25);

/// The same random walk after every reset.
const SEED: u64 = 1;

#[derive(Debug, Clone)]
pub struct Wander {
    vehicle: Vehicle,
    /// The random walk value in `-1..=1` (`WanderSide`).
    wander_side: f32,
    rng: fastrand::Rng,
    /// Push forward with `target_speed` instead of `forward`.
    use_target_speed: bool,
    target_speed: f32,
    wander_weight: f32,
    trail: Trail,
    time: f32,
}

impl Default for Wander {
    /// `MpBase::reset`: at rest at the origin, max_force 5, max_speed 3.
    fn default() -> Self {
        Self {
            vehicle: Vehicle {
                max_force: 5.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            wander_side: 0.0,
            rng: fastrand::Rng::with_seed(SEED),
            use_target_speed: false,
            target_speed: 1.5,
            wander_weight: 3.0,
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl Wander {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let rng = &mut self.rng;
        let wander = steer::wander(&self.vehicle, &mut self.wander_side, dt, &mut || rng.f32());
        let force = self.forward_push() + wander * self.wander_weight;
        self.vehicle.apply_steering_force(force, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "forward" => self.use_target_speed = false,
            "target_speed" => self.use_target_speed = true,
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

    pub fn view(&self) -> FrameView {
        let v = &self.vehicle;
        let p = v.position;
        let forward = self.forward_push();
        // The last step's wander force: the walk value along the current side.
        let wander = v.side() * self.wander_side * self.wander_weight;
        FrameView {
            time: self.time,
            focus: point(p),
            view_size: 20.0,
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines: [
                line(p, p + forward + wander, color::PUSH),
                line(p + forward, p + forward + wander, color::WANDER),
            ]
            .into_iter()
            .chain(velocity_acceleration(v))
            .collect(),
            trails: vec![self.trail.view(p)],
            labels: vec![label(
                p,
                format!("speed: {:.2}  wander: {:+.2}", v.speed, self.wander_side),
                color::LABEL,
            )],
            legend: legend(&[
                (color::PUSH, "push = forward part + wander part"),
                (color::WANDER, "wander: random sideways part"),
                (color::HAVE, "have: current velocity"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("forward", "Forward push", !self.use_target_speed),
                button("target_speed", "Target speed", self.use_target_speed),
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

    fn forward_push(&self) -> Vec2 {
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

    fn run(demo: &mut Wander, seconds: u32) {
        for _ in 0..seconds * 60 {
            demo.update(1.0 / 60.0);
        }
    }

    #[test]
    fn forward_push_reaches_max_speed_and_turns() {
        let mut demo = Wander::default();
        run(&mut demo, 10);
        assert!((demo.vehicle.speed - 3.0).abs() < 1e-3);
        assert!(demo.vehicle.forward != Vec2::new(0.0, 1.0));
    }

    /// Mean speed from 10 s to 60 s with target speed 1.5.
    fn mean_speed(wander_weight: f32) -> f32 {
        let mut demo = Wander::default();
        demo.action("target_speed");
        demo.param("wander_weight", wander_weight);
        run(&mut demo, 10);
        let mut sum = 0.0;
        for _ in 0..3000 {
            demo.update(1.0 / 60.0);
            sum += demo.vehicle.speed;
        }
        sum / 3000.0
    }

    /// Wandering runs faster than the target speed: the smoothed acceleration lags the turning
    /// side vector, so part of the sideways force ends up forward (with smoothing off the excess
    /// at weight 3 drops from 0.169 to 0.017). The C++ shows the same (oracle_wander).
    #[test]
    fn target_speed_is_exceeded_while_wandering() {
        assert!((mean_speed(0.0) - 1.5).abs() < 1e-3);
        assert!((mean_speed(3.0) - 1.669).abs() < 0.01);
        assert!((mean_speed(5.0) - 1.901).abs() < 0.01);
    }

    #[test]
    fn no_wander_goes_straight() {
        let mut demo = Wander::default();
        demo.param("wander_weight", 0.0);
        run(&mut demo, 5);
        assert_eq!(demo.vehicle.position.x, 0.0);
    }

    #[test]
    fn reset_repeats_the_same_walk() {
        let (mut a, mut b) = (Wander::default(), Wander::default());
        run(&mut a, 3);
        run(&mut b, 3);
        assert_eq!(a.vehicle, b.vehicle);
    }
}
