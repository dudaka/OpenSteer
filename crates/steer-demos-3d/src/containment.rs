//! "3D Containment" (Reynolds 1999, not in OpenSteer, so no C++ oracle): wanderers inside a box
//! room. Each steers with `steer_core_3d::containment::contain` when its ahead probe is outside
//! the room, braking at `brake` meanwhile (`applyBrakingForce`), else `forward + 3 * wander`, as
//! the 2D Containment demo.

use steer_core_3d::containment::{contain, probes};
use steer_core_3d::steer::{self, WanderState};
use steer_core_3d::wall::Block;
use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::{slider, Range};
use steer_demos::{color, legend};

use crate::annotation::{block_edges, label, line, vehicle_view, Trail};
use crate::view::Frame3View;

const WANDERERS: Range = Range::new(1.0, 20.0, 1.0);
const LOOK_AHEAD: Range = Range::new(0.25, 3.0, 0.25);
const BRAKE: Range = Range::new(0.0, 4.0, 0.5);
const SEED: u64 = 1;
const ROOM: Block = Block::new(Vec3::new(-12.0, 0.0, -8.0), Vec3::new(12.0, 12.0, 8.0));

#[derive(Debug, Clone)]
struct Wanderer {
    vehicle: Vehicle,
    walk: WanderState,
    /// Whether the body's center was outside the room after the last step.
    outside: bool,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct Containment3 {
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

impl Default for Containment3 {
    fn default() -> Self {
        Self::new(5, 1.0, 4.0)
    }
}

impl Containment3 {
    /// `count` wanderers at rest in a row across the room at mid height, facing alternately +x
    /// and -x.
    fn new(count: usize, look_ahead: f32, brake: f32) -> Self {
        let wanderers = (0..count)
            .map(|i| {
                let x = -10.0 + 20.0 * i as f32 / count as f32;
                let mut vehicle = Vehicle {
                    position: Vec3::new(x, 6.0, 0.0),
                    max_force: 5.0,
                    max_speed: 3.0,
                    ..Vehicle::default()
                };
                let facing = if i % 2 == 0 { 1.0 } else { -1.0 };
                vehicle.regenerate_basis(Vec3::new(facing, 0.0, 0.0));
                Wanderer {
                    vehicle,
                    walk: WanderState::default(),
                    outside: false,
                    trail: Trail::default(),
                }
            })
            .collect();
        Self {
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
            let push = contain(v, &ROOM, self.look_ahead);
            if push == Vec3::ZERO {
                let rng = &mut self.rng;
                let wander = steer::wander(v, &mut w.walk, dt, &mut || rng.f32());
                w.vehicle.apply_steering_force(v.forward + wander * 3.0, dt);
            } else {
                w.vehicle.apply_steering_force(push, dt);
                w.vehicle.apply_braking_force(self.brake, dt);
            }
            w.trail.record(self.time, w.vehicle.position);
            let p = w.vehicle.position;
            let outside = !ROOM.contains(p);
            if outside {
                self.escapes += u32::from(!w.outside);
                self.outside_steps += 1;
                self.furthest_out = self.furthest_out.max(ROOM.signed_distance(p));
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

    pub fn view(&self) -> Frame3View {
        let first = &self.wanderers[0].vehicle;
        let mut lines = block_edges(&ROOM, color::OBSTACLE);
        lines
            .extend(probes(first, self.look_ahead).map(|p| line(first.position, p, color::RADIUS)));
        let push = contain(first, &ROOM, self.look_ahead);
        if push != Vec3::ZERO {
            lines.push(line(first.position, first.position + push, color::PUSH));
        }
        Frame3View {
            time: self.time,
            focus: [0.0, 6.0, 0.0],
            view_size: 40.0,
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
                    Vec3::new(0.0, ROOM.max.y + 1.5, 0.0),
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
                    "wanderer 0's probes: ahead and 45 degrees to each side, up and down",
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
        let mut demo = Containment3::new(5, look_ahead, brake);
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

    /// In this convex box room nobody escapes at a 1 s look-ahead, even without braking (the
    /// 2D L-shaped room needed braking at its corners); only a 0.5 s look-ahead without braking
    /// lets bodies out (14 escapes, at most 0.39 out, all back by the end).
    #[test]
    fn the_box_room_holds_its_wanderers() {
        assert_minute(1.0, 0.0, (0, 0, 0.0, 0));
        assert_minute(1.0, 4.0, (0, 0, 0.0, 0));
        assert_minute(0.5, 4.0, (0, 0, 0.0, 0));
        assert_minute(0.5, 0.0, (14, 812, 0.391, 0));
    }

    #[test]
    fn wanderers_slider_restarts_with_that_many() {
        let mut demo = Containment3::default();
        demo.update(1.0 / 60.0);
        demo.param("wanderers", 2.0);
        assert_eq!(demo.wanderers.len(), 2);
        assert_eq!(demo.time, 0.0);
    }
}
