//! "3D One Turning": the integrator alone, as `oracle/vehicle_3d.cpp`. Climbing turn: a constant
//! push (-2, 1.5, -3) with max force 0.3, max speed 5, from speed 1.5. Banking circle: a push
//! `0.8 side + 0.1 forward` with banking on (as Boids), max force 1, max speed 2, from speed 1;
//! rolling into the turn tilts `side` down, so it slowly sinks.

use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::button;
use steer_demos::{color, legend};

use crate::annotation::{label, line, point, stem, vehicle_view, velocity_acceleration, Trail};
use crate::view::Frame3View;

const CLIMB_PUSH: Vec3 = Vec3::new(-2.0, 1.5, -3.0);

#[derive(Debug, Clone)]
pub struct OneTurning3 {
    vehicle: Vehicle,
    trail: Trail,
    time: f32,
}

impl Default for OneTurning3 {
    fn default() -> Self {
        Self::climb()
    }
}

impl OneTurning3 {
    fn climb() -> Self {
        Self::with(Vehicle {
            speed: 1.5,
            max_force: 0.3,
            max_speed: 5.0,
            ..Vehicle::default()
        })
    }

    fn bank() -> Self {
        Self::with(Vehicle {
            speed: 1.0,
            max_force: 1.0,
            max_speed: 2.0,
            banking: true,
            ..Vehicle::default()
        })
    }

    fn with(vehicle: Vehicle) -> Self {
        Self {
            vehicle,
            trail: Trail::default(),
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let force = self.force();
        self.vehicle.apply_steering_force(force, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    /// Switching restarts.
    pub fn action(&mut self, id: &str) {
        match id {
            "climb" => *self = Self::climb(),
            "bank" => *self = Self::bank(),
            _ => {}
        }
    }

    pub fn view(&self) -> Frame3View {
        let v = &self.vehicle;
        let p = v.position;
        let mut lines = vec![line(p, p + self.force(), color::PUSH), stem(p)];
        lines.extend(velocity_acceleration(v));
        Frame3View {
            time: self.time,
            focus: point(p),
            view_size: 14.0,
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines,
            trails: vec![self.trail.view(p)],
            labels: vec![label(
                p,
                format!(
                    "speed {:.2}, height {:.2}, up.y {:.3}",
                    v.speed, p.y, v.up.y
                ),
                color::LABEL,
            )],
            legend: legend(&[
                (color::PUSH, "push"),
                (color::HAVE, "have: current velocity"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("climb", "Climbing turn", !v.banking),
                button("bank", "Banking circle", v.banking),
            ],
            params: Vec::new(),
        }
    }

    fn force(&self) -> Vec3 {
        let v = &self.vehicle;
        if v.banking {
            v.side * 0.8 + v.forward * 0.1
        } else {
            CLIMB_PUSH
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after_600(mut demo: OneTurning3) -> Vec3 {
        for _ in 0..600 {
            demo.update(1.0 / 60.0);
        }
        demo.vehicle.position
    }

    /// Both modes are the oracle's runs (`oracle_vehicle_3d`'s step 600).
    #[test]
    fn modes_run_the_oracle_scenarios() {
        let climb = after_600(OneTurning3::default());
        assert!((climb - Vec3::new(-7.578992, 5.684265, 3.703887)).length() < 1e-4);
        let mut demo = OneTurning3::default();
        demo.action("bank");
        let bank = after_600(demo);
        assert!((bank - Vec3::new(1.461162, -3.297424, -5.676027)).length() < 1e-4);
    }
}
