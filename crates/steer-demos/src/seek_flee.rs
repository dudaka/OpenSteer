//! "Seek and Flee": one vehicle steering toward, or away from, a target the user clicks.
//! Not an OpenSteer plug-in; it shows `seek`/`flee` alone, with the desired velocity and the
//! steering force (desired minus current velocity) drawn.

use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{label, line, point, vehicle_view, velocity_acceleration, Trail};
use crate::view::{ActionView, FrameView};

const TARGET_COLOR: u32 = 0xffd23f;
const DESIRED_COLOR: u32 = 0x3fd27a;
const FORCE_COLOR: u32 = 0xff5040;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Seek,
    Flee,
}

#[derive(Debug, Clone)]
pub struct SeekFlee {
    vehicle: Vehicle,
    target: Vec2,
    mode: Mode,
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
            trail: Trail::default(),
            time: 0.0,
        }
    }
}

impl SeekFlee {
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
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
            vehicles: vec![vehicle_view(v, 0x808080)],
            lines: [
                line(
                    t - Vec2::new(cross, 0.0),
                    t + Vec2::new(cross, 0.0),
                    TARGET_COLOR,
                ),
                line(
                    t - Vec2::new(0.0, cross),
                    t + Vec2::new(0.0, cross),
                    TARGET_COLOR,
                ),
                line(p, p + v.velocity() + force, DESIRED_COLOR),
                line(p + v.velocity(), p + v.velocity() + force, FORCE_COLOR),
            ]
            .into_iter()
            .chain(velocity_acceleration(v))
            .collect(),
            trails: vec![self.trail.view(p)],
            labels: vec![
                label(p, format!("speed: {:.2}", v.speed), 0xff0000),
                label(t, "target (click to move)", TARGET_COLOR),
            ],
            actions: [("seek", "Seek", Mode::Seek), ("flee", "Flee", Mode::Flee)]
                .into_iter()
                .map(|(id, label, mode)| ActionView {
                    id: id.to_string(),
                    label: label.to_string(),
                    active: self.mode == mode,
                })
                .collect(),
        }
    }

    fn force(&self) -> Vec2 {
        match self.mode {
            Mode::Seek => steer::seek(&self.vehicle, self.target),
            Mode::Flee => steer::flee(&self.vehicle, self.target),
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
