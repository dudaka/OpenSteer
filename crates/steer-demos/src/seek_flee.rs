//! "Seek and Flee": one vehicle steering toward, away from, or arriving at a target the user
//! clicks. Not an OpenSteer plug-in; it shows `seek`/`flee`/`arrive` alone, with the desired
//! velocity and the steering force (desired minus current velocity) drawn. In Arrive mode,
//! Brake adds OpenSteer's `applyBrakingForce` inside the slowing radius.

use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{
    circle, color, label, legend, line, point, vehicle_view, velocity_acceleration, Trail,
};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LineView, ParamView};

const SLOWING_RADIUS: Range = Range::new(1.0, 15.0, 0.5);
const BRAKING_RATE: Range = Range::new(0.25, 4.0, 0.25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Seek,
    Flee,
    Arrive,
}

#[derive(Debug, Clone)]
pub struct SeekFlee {
    vehicle: Vehicle,
    target: Vec2,
    mode: Mode,
    slowing_radius: f32,
    /// Brake inside the slowing radius (Arrive only).
    brake: bool,
    braking_rate: f32,
    trail: Trail,
    time: f32,
}

impl Default for SeekFlee {
    fn default() -> Self {
        Self {
            vehicle: Vehicle {
                max_force: 1.0,
                max_speed: 3.0,
                ..Vehicle::default()
            },
            target: Vec2::new(6.0, -4.0),
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

impl SeekFlee {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        if self.braking() {
            self.vehicle.apply_braking_force(self.braking_rate, dt);
        }
        self.vehicle.apply_steering_force(self.force(), dt);
        self.trail.record(self.time, self.vehicle.position);
    }

    /// A click moves the target.
    pub fn click(&mut self, at: Vec2) {
        self.target = at;
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "seek" => self.mode = Mode::Seek,
            "flee" => self.mode = Mode::Flee,
            "arrive" => self.mode = Mode::Arrive,
            "brake" => self.brake = !self.brake,
            _ => {}
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

    pub fn view(&self) -> FrameView {
        let v = &self.vehicle;
        let p = v.position;
        let force = self.force();
        let t = self.target;
        let cross = 0.4;
        FrameView {
            time: self.time,
            focus: point(p),
            view_size: 20.0,
            vehicles: vec![vehicle_view(v, color::BODY)],
            lines: [
                line(
                    t - Vec2::new(cross, 0.0),
                    t + Vec2::new(cross, 0.0),
                    color::TARGET,
                ),
                line(
                    t - Vec2::new(0.0, cross),
                    t + Vec2::new(0.0, cross),
                    color::TARGET,
                ),
                line(p, p + v.velocity() + force, color::WANT),
                line(p + v.velocity(), p + v.velocity() + force, color::PUSH),
            ]
            .into_iter()
            .chain(velocity_acceleration(v))
            .chain(self.slowing_circle())
            .collect(),
            trails: vec![self.trail.view(p)],
            labels: vec![
                label(p, format!("speed: {:.2}", v.speed), color::LABEL),
                label(t, "target (click to move)", color::TARGET),
            ],
            legend: legend(&[
                (color::WANT, "want: desired velocity"),
                (color::HAVE, "have: current velocity"),
                (color::PUSH, "push = want - have"),
                (color::CHANGE, "change: smoothed acceleration"),
                (color::TARGET, "target"),
                (color::RADIUS, "slowing radius (Arrive)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("seek", "Seek", self.mode == Mode::Seek),
                button("flee", "Flee", self.mode == Mode::Flee),
                button("arrive", "Arrive", self.mode == Mode::Arrive),
                button("brake", "Brake", self.brake),
            ],
            params: self.params(),
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

    fn force(&self) -> Vec2 {
        match self.mode {
            Mode::Seek => steer::seek(&self.vehicle, self.target),
            Mode::Flee => steer::flee(&self.vehicle, self.target),
            Mode::Arrive => steer::arrive(&self.vehicle, self.target, self.slowing_radius),
        }
    }

    fn braking(&self) -> bool {
        self.mode == Mode::Arrive
            && self.brake
            && (self.target - self.vehicle.position).length() < self.slowing_radius
    }

    fn slowing_circle(&self) -> Vec<LineView> {
        match self.mode {
            Mode::Arrive => circle(self.target, self.slowing_radius, color::RADIUS),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(demo: &mut SeekFlee, steps: u32) {
        for _ in 0..steps {
            demo.update(1.0 / 60.0);
        }
    }

    fn distance_to_target(demo: &SeekFlee) -> f32 {
        (demo.target - demo.vehicle.position).length()
    }

    /// Pure seek never comes to rest: near the target it settles into an orbit (the C++ does
    /// the same, to the printed digit). Stopping is what `arrive` is for.
    #[test]
    fn seek_ends_up_orbiting_a_clicked_target() {
        let mut demo = SeekFlee::default();
        demo.click(Vec2::new(-3.0, 4.0));
        run(&mut demo, 1800);
        assert!(distance_to_target(&demo) < 1.0);
        assert!(demo.vehicle.speed > 0.5);
    }

    /// Arrive at (-3, 4) from rest at the origin, the setup of the seek orbit finding.
    fn arrive(slowing_radius: f32, brake: Option<f32>) -> SeekFlee {
        let mut demo = SeekFlee {
            slowing_radius,
            brake: brake.is_some(),
            braking_rate: brake.unwrap_or(0.0),
            ..SeekFlee::default()
        };
        demo.action("arrive");
        demo.click(Vec2::new(-3.0, 4.0));
        demo
    }

    /// The slowing radius alone does not stop it: the low-speed rule turns braking into a
    /// forward push, so it orbits just under 0.2 * max_speed, as seek does.
    #[test]
    fn arrive_without_brake_orbits() {
        let mut demo = arrive(9.0, None);
        run(&mut demo, 1800);
        assert!((distance_to_target(&demo) - 0.544).abs() < 0.01);
        assert!((demo.vehicle.speed - 0.583).abs() < 0.01);
    }

    #[test]
    fn arrive_with_brake_can_stop() {
        let mut demo = arrive(9.0, Some(0.75));
        run(&mut demo, 3600);
        assert!(distance_to_target(&demo) < 1e-3);
        assert!(demo.vehicle.speed < 1e-3);
    }

    /// Creeping past the target puts it behind; the low-speed rule turns the pull back into a
    /// forward push and the vehicle leaves.
    #[test]
    fn arrive_with_brake_that_passes_the_target_leaves_it() {
        let mut demo = arrive(5.0, Some(0.75));
        let mut closest = f32::MAX;
        for _ in 0..1800 {
            demo.update(1.0 / 60.0);
            closest = closest.min(distance_to_target(&demo));
        }
        assert!(closest < 1e-3);
        assert!(distance_to_target(&demo) > 0.1);
    }

    /// Strong braking against a full forward push holds speed at max_force / rate.
    #[test]
    fn strong_brake_holds_speed_at_max_force_over_rate() {
        let mut demo = arrive(5.0, Some(3.0));
        run(&mut demo, 1200);
        assert!((demo.vehicle.speed - 1.0 / 3.0).abs() < 1e-3);
        assert!(distance_to_target(&demo) > 2.0);
    }

    #[test]
    fn sliders_show_in_arrive_and_set_values_within_range() {
        let mut demo = SeekFlee::default();
        assert!(demo.view().params.is_empty());
        demo.action("arrive");
        demo.param("slowing_radius", 4.5);
        demo.param("braking_rate", 99.0);
        let values: Vec<f32> = demo.view().params.iter().map(|p| p.value).collect();
        assert_eq!(values, [4.5, 4.0]);
        demo.param("slowing_radius", 0.0);
        assert_eq!(demo.slowing_radius, 1.0);
    }

    #[test]
    fn flee_moves_away() {
        let mut demo = SeekFlee::default();
        demo.action("flee");
        let start = distance_to_target(&demo);
        run(&mut demo, 300);
        assert!(distance_to_target(&demo) > start + 5.0);
        assert!(demo.view().actions[1].active);
    }

    #[test]
    fn desired_velocity_line_is_velocity_plus_force() {
        let demo = SeekFlee::default();
        // At rest the desired velocity is the full seek force, truncated to max_speed.
        let desired = &demo.view().lines[2];
        assert_eq!(desired.from, [0.0, 0.0]);
        assert!((Vec2::new(desired.to[0], desired.to[1]).length() - 3.0).abs() < 1e-5);
    }
}
