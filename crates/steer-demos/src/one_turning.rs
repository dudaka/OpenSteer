//! "One Turning Away" (`plugins/OneTurning.cpp`): one vehicle under a constant steering
//! force, showing only the integrator: it turns toward the force and speeds up.

use steer_core::{Vec2, Vehicle};

use crate::annotation::{
    color, label, legend, line, point, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::FrameView;

const FORCE: Vec2 = Vec2::new(-2.0, -3.0);

#[derive(Debug, Clone)]
pub struct OneTurning {
    vehicle: Vehicle,
    trail: Trail,
    time: f32,
}

impl Default for OneTurning {
    /// `OneTurning::reset`.
    fn default() -> Self {
        Self {
            vehicle: Vehicle {
                speed: 1.5,
                max_force: 0.3,
                max_speed: 5.0,
                ..Vehicle::default()
            },
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl OneTurning {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.vehicle.apply_steering_force(FORCE, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn view(&self) -> FrameView {
        let v = &self.vehicle;
        let p = v.position;
        // The push as the vehicle receives it, capped at max_force, on the acceleration line's
        // scale: the blue (smoothed) line can be seen catching up with it.
        let push = FORCE.truncate_length(v.max_force) * (3.0 / v.max_force);
        FrameView {
            time: self.time,
            focus: point(p),
            vehicles: vec![vehicle_view(v, color::BODY)],
            // Push last, so it is drawn on top of the acceleration it soon matches.
            lines: velocity_acceleration(v)
                .into_iter()
                .chain([line(p, p + push, color::PUSH)])
                .collect(),
            trails: vec![self.trail.view(p)],
            labels: vec![
                label(p, format!("speed: {:.2}", v.speed), color::LABEL),
                label(Vec2::ZERO, "start", color::LABEL),
            ],
            actions: Vec::new(),
            legend: legend(&[
                (
                    color::PUSH,
                    "push (the same every step, capped at max force)",
                ),
                (color::HAVE, "have: current velocity"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_follows_the_vehicle() {
        let mut demo = OneTurning::default();
        for _ in 0..60 {
            demo.update(1.0 / 60.0);
        }
        let view = demo.view();
        assert_eq!(view.focus, view.vehicles[0].position);
        assert_eq!(view.labels[0].text, "speed: 1.28");
        assert_eq!(view.lines.len(), 3);
    }

    #[test]
    fn push_line_is_the_capped_force_at_three_units() {
        let view = OneTurning::default().view();
        let push = view.lines.last().expect("the push line");
        assert_eq!(push.color, color::PUSH);
        let len = Vec2::new(push.to[0] - push.from[0], push.to[1] - push.from[1]);
        assert!((len.length() - 3.0).abs() < 1e-5);
        assert!(len.normalize().dot(FORCE.normalize()) > 0.9999);
    }
}
