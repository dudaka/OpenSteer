//! "One Turning Away" (`plugins/OneTurning.cpp`): one vehicle under a constant steering
//! force, showing only the integrator: it turns toward the force and speeds up.

use steer_core::{Vec2, Vehicle};

use crate::annotation::{label, point, vehicle_view, velocity_acceleration, Trail};
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
        FrameView {
            time: self.time,
            focus: point(v.position),
            vehicles: vec![vehicle_view(v, 0x808080)],
            lines: velocity_acceleration(v).to_vec(),
            trails: vec![self.trail.view(v.position)],
            labels: vec![
                label(v.position, format!("speed: {:.2}", v.speed), 0xff0000),
                label(Vec2::ZERO, "start", 0x00ff00),
            ],
            actions: Vec::new(),
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
        assert_eq!(view.lines.len(), 2);
    }
}
