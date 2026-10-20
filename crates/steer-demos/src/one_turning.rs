//! "One Turning Away" (`plugins/OneTurning.cpp`): one vehicle under a constant steering
//! force, showing only the integrator: it turns toward the force and speeds up.

use steer_core::{Vec2, Vehicle};

use crate::annotation::{
    color, label, legend, line, point, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::FrameView;

const FORCE: Vec2 = Vec2::new(-2.0, -3.0);

/// The push is drawn as a direction, longer than the acceleration line's 3 units at max force:
/// once the smoothed acceleration matches the push it lies along the first part of this line,
/// and the two split where the low-speed rule bends the force.
const PUSH_LENGTH: f32 = 4.5;

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
        FrameView {
            time: self.time,
            focus: point(p),
            vehicles: vec![vehicle_view(v, color::BODY)],
            // The push first, so velocity and acceleration are drawn over it.
            lines: [line(p, p + FORCE.normalize() * PUSH_LENGTH, color::PUSH)]
                .into_iter()
                .chain(velocity_acceleration(v))
                .collect(),
            trails: vec![self.trail.view(p)],
            labels: vec![
                label(p, format!("speed: {:.2}", v.speed), color::LABEL),
                label(Vec2::ZERO, "start", color::LABEL),
            ],
            actions: Vec::new(),
            params: Vec::new(),
            legend: legend(&[
                (color::PUSH, "push direction (the same every step)"),
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
    fn push_is_drawn_first_and_longer_than_the_acceleration_line() {
        let mut demo = OneTurning::default();
        for _ in 0..120 {
            demo.update(1.0 / 60.0);
        }
        let view = demo.view();
        let offset =
            |l: &crate::view::LineView| Vec2::new(l.to[0] - l.from[0], l.to[1] - l.from[1]);
        let push = &view.lines[0];
        assert_eq!(push.color, color::PUSH);
        assert!((offset(push).length() - PUSH_LENGTH).abs() < 1e-5);
        assert!(offset(push).normalize().dot(FORCE.normalize()) > 0.9999);
        let change = view
            .lines
            .iter()
            .find(|l| l.color == color::CHANGE)
            .expect("acceleration line");
        // At 2 s the acceleration has settled on the push: same direction, 3 units, inside it.
        assert!(offset(change).normalize().dot(FORCE.normalize()) > 0.9999);
        assert!(offset(change).length() < PUSH_LENGTH);
    }
}
