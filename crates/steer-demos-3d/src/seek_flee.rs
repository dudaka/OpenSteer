//! "3D Seek and Flee": one vehicle steering toward, away from, or arriving at a target in the
//! air (as `oracle/seek_flee_3d.cpp`: max force 1, max speed 3). "Move target" hops the target
//! between four points at different heights. Arrive (Reynolds 1999, not in OpenSteer) slows
//! inside a sphere around the target; Brake adds `applyBrakingForce` inside it, as in 2D.

use steer_core_3d::{steer, Vec3, Vehicle};
use steer_demos::controls::{button, slider, Range};
use steer_demos::{color, legend, ParamView};

use crate::annotation::{
    cross, label, line, point, sphere, stem, vehicle_view, velocity_acceleration, Trail,
};
use crate::view::Frame3View;

const SLOWING_RADIUS: Range = Range::new(1.0, 15.0, 0.5);
const BRAKING_RATE: Range = Range::new(0.25, 4.0, 0.25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Seek,
    Flee,
    Arrive,
}

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
    mode: Mode,
    slowing_radius: f32,
    /// Brake inside the slowing radius (Arrive only).
    brake: bool,
    braking_rate: f32,
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
            mode: Mode::Seek,
            slowing_radius: 9.0,
            brake: false,
            // CaptureTheFlag's gBrakingRate
            braking_rate: 0.75,
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl SeekFlee3 {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        if self.braking() {
            self.vehicle.apply_braking_force(self.braking_rate, dt);
        }
        let force = self.force();
        self.vehicle.apply_steering_force(force, dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "seek" => self.mode = Mode::Seek,
            "flee" => self.mode = Mode::Flee,
            "arrive" => self.mode = Mode::Arrive,
            "brake" => self.brake = !self.brake,
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
        if self.mode == Mode::Arrive {
            lines.extend(sphere(t, self.slowing_radius, color::RADIUS));
        }
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
                (color::RADIUS, "slowing radius (Arrive)"),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("seek", "Seek", self.mode == Mode::Seek),
                button("flee", "Flee", self.mode == Mode::Flee),
                button("arrive", "Arrive", self.mode == Mode::Arrive),
                button("brake", "Brake", self.brake),
                button("move_target", "Move target", false),
            ],
            params: self.params(),
        }
    }

    /// A slider moved; the value is held to the slider's range.
    pub fn param(&mut self, id: &str, value: f32) {
        match id {
            "slowing_radius" => self.slowing_radius = SLOWING_RADIUS.clamp(value),
            "braking_rate" => self.braking_rate = BRAKING_RATE.clamp(value),
            _ => {}
        }
    }

    /// Arrive's sliders, shown in Arrive mode.
    fn params(&self) -> Vec<ParamView> {
        if self.mode != Mode::Arrive {
            return Vec::new();
        }
        vec![
            slider(
                "slowing_radius",
                "slowing radius",
                SLOWING_RADIUS,
                self.slowing_radius,
            ),
            slider(
                "braking_rate",
                "braking rate",
                BRAKING_RATE,
                self.braking_rate,
            ),
        ]
    }

    fn force(&self) -> Vec3 {
        let t = TARGETS[self.target];
        match self.mode {
            Mode::Seek => steer::seek(&self.vehicle, t),
            Mode::Flee => steer::flee(&self.vehicle, t),
            Mode::Arrive => steer::arrive(&self.vehicle, t, self.slowing_radius),
        }
    }

    fn braking(&self) -> bool {
        self.mode == Mode::Arrive
            && self.brake
            && (TARGETS[self.target] - self.vehicle.position).length() < self.slowing_radius
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

    /// Distance and speed after 30 s of Arrive toward (6, 2, -4).
    fn arrive_for_30_s(brake: bool, radius: f32, rate: f32) -> (f32, f32) {
        let mut demo = SeekFlee3::default();
        demo.action("arrive");
        if brake {
            demo.action("brake");
        }
        demo.param("slowing_radius", radius);
        demo.param("braking_rate", rate);
        for _ in 0..1800 {
            demo.update(1.0 / 60.0);
        }
        let d = (TARGETS[0] - demo.vehicle.position).length();
        (d, demo.vehicle.speed)
    }

    /// As in 2D: Arrive alone orbits just under the low-speed rule's 0.6 (radius 9: 0.554 away
    /// at 0.582); braking stops it at radius 9 rate 0.75 (0.018 away, 0.004) and radius 5 rate
    /// 1.5 (0.002, 0.0007), but radius 5 rate 0.75 still orbits (0.547, 0.566).
    #[test]
    fn arrive_orbits_unless_braking_is_tuned() {
        let close = |(d, s): (f32, f32), (wd, ws): (f32, f32)| {
            assert!((d - wd).abs() < 0.001 && (s - ws).abs() < 0.001, "{d} {s}");
        };
        close(arrive_for_30_s(false, 9.0, 0.75), (0.554, 0.582));
        close(arrive_for_30_s(true, 9.0, 0.75), (0.018, 0.004));
        close(arrive_for_30_s(true, 5.0, 1.5), (0.002, 0.001));
        close(arrive_for_30_s(true, 5.0, 0.75), (0.547, 0.566));
    }
}
