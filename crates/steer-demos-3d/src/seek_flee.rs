//! "3D Seek and Flee": one vehicle steering toward or away from a target in the air (as
//! `oracle/seek_flee_3d.cpp`: max force 1, max speed 3). "Move target" hops the target between
//! four points at different heights.

use steer_core_3d::{steer, Vec3, Vehicle};
use steer_demos::controls::button;
use steer_demos::{color, legend};

use crate::annotation::{
    cross, label, line, point, stem, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::Frame3View;

/// The target's spots; the first is the oracle's.
const TARGETS: [Vec3; 4] = [
    Vec3::new(6.0, 2.0, -4.0),
    Vec3::new(-5.0, 4.0, 3.0),
    Vec3::new(4.0, -3.0, 5.0),
    Vec3::new(-6.0, 1.0, -5.0),
];

#[derive(Debug, Clone)]
pub struct SeekFlee3 {
    vehicle: Vehicle,
    target: usize,
    flee: bool,
    trail: Trail,
    time: f32,
}

impl Default for SeekFlee3 {
    fn default() -> Self {
        Self {
            vehicle: Vehicle {
                max_force: 1.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            target: 0,
            flee: false,
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl SeekFlee3 {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let force = self.force();
        self.vehicle.apply_steering_force(force, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "seek" => self.flee = false,
            "flee" => self.flee = true,
            "move_target" => self.target = (self.target + 1) % TARGETS.len(),
            _ => {}
        }
    }

    pub fn view(&self) -> Frame3View {
        let v = &self.vehicle;
        let (p, t) = (v.position, TARGETS[self.target]);
        let force = self.force();
        let want = v.velocity() + force;
        let mut lines = vec![
            line(p, p + want, color::WANT),
            line(p + v.velocity(), p + want, color::PUSH),
            stem(p),
            stem(t),
        ];
        lines.extend(cross(t, 0.4, color::TARGET));
        lines.extend(velocity_acceleration(v));
        Frame3View {
            time: self.time,
            focus: point(p),
            view_size: 22.0,
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines,
            trails: vec![self.trail.view(p)],
            labels: vec![
                label(
                    p,
                    format!("speed {:.2}, height {:.2}", v.speed, p.y),
                    color::LABEL,
                ),
                label(t, format!("target, height {:.0}", t.y), color::TARGET),
            ],
            legend: legend(&[
                (color::WANT, "want: desired velocity"),
                (color::HAVE, "have: current velocity"),
                (color::PUSH, "push = want - have"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::TARGET, "target"),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("seek", "Seek", !self.flee),
                button("flee", "Flee", self.flee),
                button("move_target", "Move target", false),
            ],
            params: Vec::new(),
        }
    }

    fn force(&self) -> Vec3 {
        let t = TARGETS[self.target];
        if self.flee {
            steer::flee(&self.vehicle, t)
        } else {
            steer::seek(&self.vehicle, t)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The demo is the oracle's run: after 300 steps of seek the position matches
    /// `oracle_seek_flee_3d`'s last checkpoint.
    #[test]
    fn seek_runs_the_oracle_scenario() {
        let mut demo = SeekFlee3::default();
        for _ in 0..300 {
            demo.update(1.0 / 60.0);
        }
        let p = demo.vehicle.position;
        assert!(
            (p - Vec3::new(5.343373, 1.781124, -3.162268)).length() < 1e-4,
            "{p:?}"
        );
    }

    #[test]
    fn move_target_cycles_through_four_spots() {
        let mut demo = SeekFlee3::default();
        for _ in 0..4 {
            demo.action("move_target");
        }
        assert_eq!(demo.target, 0);
        demo.action("move_target");
        assert_eq!(demo.view().labels[1].at, point(TARGETS[1]));
    }
}
