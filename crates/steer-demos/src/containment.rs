//! "Containment" (Reynolds 1999, not in OpenSteer, so no C++ oracle): wanderers inside an
//! L-shaped room. Each steers with `steer_core::containment::contain` when its probe is outside
//! the room, braking at `brake` meanwhile (`applyBrakingForce`), else `forward + 3 * wander`, as
//! the Obstacle Avoidance demo. Without braking, wanderers at full speed turn too wide to get
//! out of a corner and slip through it.

use steer_core::containment::{contain, probes};
use steer_core::wall::Wall;
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{color, label, legend, line, vehicle_view, Trail};
use crate::controls::{slider, Range};
use crate::view::FrameView;

const WANDERERS: Range = Range::new(1.0, 20.0, 1.0);
const LOOK_AHEAD: Range = Range::new(0.25, 3.0, 0.25);
const BRAKE: Range = Range::new(0.0, 4.0, 0.5);
const SEED: u64 = 1;

#[derive(Debug, Clone)]
struct Wanderer {
    vehicle: Vehicle,
    wander_side: f32,
    /// Whether the body's center was outside the room after the last step.
    outside: bool,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct Containment {
    room: Wall,
    wanderers: Vec<Wanderer>,
    rng: fastrand::Rng,
    look_ahead: f32,
    /// Braking rate while the containment push is on.
    brake: f32,
    /// Times a body's center left the room, wanderer-steps outside it, and the furthest out.
    escapes: u32,
    outside_steps: u32,
    furthest_out: f32,
    time: f32,
}

impl Default for Containment {
    fn default() -> Self {
        Self::new(5, 1.0, 4.0)
    }
}

/// A 24 x 10 bar with a 10 x 10 block on its left end, centered near the origin.
fn room() -> Wall {
    let corners = [
        (0., 0.),
        (24., 0.),
        (24., 10.),
        (10., 10.),
        (10., 20.),
        (0., 20.),
    ];
    let points: Vec<Vec2> = corners
        .iter()
        .map(|&(x, y)| Vec2::new(x - 12.0, y - 10.0))
        .collect();
    Wall::new(&points)
}

impl Containment {
    /// `count` wanderers at rest along the bar, facing alternately right and left.
    fn new(count: usize, look_ahead: f32, brake: f32) -> Self {
        let wanderers = (0..count)
            .map(|i| {
                let x = -10.0 + 20.0 * i as f32 / count as f32;
                let facing = if i % 2 == 0 { 1.0 } else { -1.0 };
                Wanderer {
                    vehicle: Vehicle {
                        position: Vec2::new(x, -5.0),
                        forward: Vec2::new(facing, 0.0),
                        max_force: 5.0,
                        max_speed: 3.0,
                        ..Vehicle::default()
                    },
                    wander_side: 0.0,
                    outside: false,
                    trail: Trail::default(),
                }
            })
            .collect();
        Self {
            room: room(),
            wanderers,
            rng: fastrand::Rng::with_seed(SEED),
            look_ahead,
            brake,
            escapes: 0,
            outside_steps: 0,
            furthest_out: 0.0,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for w in &mut self.wanderers {
            let v = &w.vehicle;
            let push = contain(v, &self.room, self.look_ahead);
            if push == Vec2::ZERO {
                let rng = &mut self.rng;
                let wander = steer::wander(v, &mut w.wander_side, dt, &mut || rng.f32());
                w.vehicle.apply_steering_force(v.forward + wander * 3.0, dt);
            } else {
                w.vehicle.apply_steering_force(push, dt);
                w.vehicle.apply_braking_force(self.brake, dt);
            }
            w.trail.record(self.time, w.vehicle.position);
            let p = w.vehicle.position;
            let outside = !self.room.contains(p);
            if outside {
                self.escapes += u32::from(!w.outside);
                self.outside_steps += 1;
                self.furthest_out = self.furthest_out.max(self.room.distance(p));
            }
            w.outside = outside;
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        match id {
            "wanderers" => {
                *self = Self::new(WANDERERS.clamp(value) as usize, self.look_ahead, self.brake)
            }
            "look_ahead" => self.look_ahead = LOOK_AHEAD.clamp(value),
            "brake" => self.brake = BRAKE.clamp(value),
            _ => {}
        }
    }

    pub fn view(&self) -> FrameView {
        let first = &self.wanderers[0].vehicle;
        let mut lines: Vec<_> = self
            .room
            .edges()
            .map(|(a, b)| line(a, b, color::OBSTACLE))
            .collect();
        lines
            .extend(probes(first, self.look_ahead).map(|p| line(first.position, p, color::RADIUS)));
        let push = contain(first, &self.room, self.look_ahead);
        if push != Vec2::ZERO {
            lines.push(line(first.position, first.position + push, color::PUSH));
        }
        FrameView {
            time: self.time,
            focus: [0.0, 0.0],
            view_size: 32.0,
            vehicles: self
                .wanderers
                .iter()
                .map(|w| vehicle_view(&w.vehicle, color::BODY))
                .collect(),
            lines,
            trails: self
                .wanderers
                .iter()
                .map(|w| w.trail.view(w.vehicle.position))
                .collect(),
            labels: vec![
                label(first.position, "wanderer 0", color::LABEL),
                label(
                    Vec2::new(0.0, -12.0),
                    format!(
                        "escapes: {}, wanderer-steps outside: {}, furthest out: {:.2}",
                        self.escapes, self.outside_steps, self.furthest_out
                    ),
                    color::LABEL,
                ),
            ],
            legend: legend(&[
                (color::OBSTACLE, "room walls"),
                (
                    color::RADIUS,
                    "wanderer 0's probes: ahead and 45 degrees to each side",
                ),
                (color::PUSH, "wanderer 0's containment push"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: Vec::new(),
            params: vec![
                slider(
                    "wanderers",
                    "wanderers",
                    WANDERERS,
                    self.wanderers.len() as f32,
                ),
                slider("look_ahead", "look-ahead (s)", LOOK_AHEAD, self.look_ahead),
                slider("brake", "brake while containing", BRAKE, self.brake),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minute with 5 wanderers: escapes, wanderer-steps outside, furthest out, and how many
    /// are still outside at the end.
    fn minute(look_ahead: f32, brake: f32) -> (u32, u32, f32, usize) {
        let mut demo = Containment::new(5, look_ahead, brake);
        for _ in 0..60 * 60 {
            demo.update(1.0 / 60.0);
        }
        let still_out = demo.wanderers.iter().filter(|w| w.outside).count();
        (
            demo.escapes,
            demo.outside_steps,
            demo.furthest_out,
            still_out,
        )
    }

    fn assert_minute(look_ahead: f32, brake: f32, want: (u32, u32, f32, usize)) {
        let got = minute(look_ahead, brake);
        assert!(
            (got.0, got.1, got.3) == (want.0, want.1, want.3) && (got.2 - want.2).abs() < 0.01,
            "look-ahead {look_ahead}, brake {brake}: {got:?}"
        );
    }

    /// Without braking, wanderers at full speed turn too wide to get out of a corner (radius
    /// about speed^2 / max_force = 1.8) and slip through it, then come back.
    #[test]
    fn without_braking_wanderers_slip_out_at_corners() {
        assert_minute(1.0, 0.0, (4, 364, 2.05, 0));
        assert_minute(0.5, 0.0, (32, 2029, 0.94, 1));
    }

    /// Braking while containing slows the turn into a tighter one; the default (look-ahead
    /// 1 s, rate 4) is the only setting tried that keeps everyone in.
    #[test]
    fn braking_keeps_them_in() {
        assert_minute(1.0, 1.0, (3, 86, 0.64, 0));
        assert_minute(1.0, 4.0, (0, 0, 0.0, 0));
        assert_minute(0.5, 4.0, (1, 31, 0.13, 0));
        assert_minute(2.0, 4.0, (1, 93, 0.70, 0));
    }

    #[test]
    fn wanderers_slider_restarts_with_that_many() {
        let mut demo = Containment::default();
        demo.update(1.0 / 60.0);
        demo.param("wanderers", 2.0);
        assert_eq!(demo.wanderers.len(), 2);
        assert_eq!(demo.time, 0.0);
    }
}
