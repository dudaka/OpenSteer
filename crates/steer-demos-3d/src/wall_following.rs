//! "3D Wall Following" (Reynolds 1999, not in OpenSteer, so no C++ oracle): walkers hug a
//! floating block at a fixed gap. Each steers with `forward + follow_surface`: seek the point
//! `offset` out from the surface point nearest its predicted position
//! (`steer_core_3d::wall::follow_surface`), so it slides along the surface wherever it heads.

use steer_core_3d::wall::{follow_surface, follow_surface_target, Block};
use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::{slider, Range};
use steer_demos::{color, legend};

use crate::annotation::{block_edges, cross, label, line, stem, vehicle_view, Trail};
use crate::view::Frame3View;

const OFFSET: Range = Range::new(0.5, 6.0, 0.25);
const LOOK_AHEAD: Range = Range::new(0.25, 3.0, 0.25);
const BLOCK: Block = Block::new(Vec3::new(-6.0, 1.0, -4.0), Vec3::new(6.0, 7.0, 4.0));

#[derive(Debug, Clone)]
struct Walker {
    vehicle: Vehicle,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct WallFollowing3 {
    walkers: Vec<Walker>,
    offset: f32,
    look_ahead: f32,
    time: f32,
}

impl Default for WallFollowing3 {
    fn default() -> Self {
        Self::new(2.0, 1.0)
    }
}

impl WallFollowing3 {
    /// Three walkers at rest away from the block, facing different ways.
    fn new(offset: f32, look_ahead: f32) -> Self {
        let starts = [
            (Vec3::new(-12.0, 4.0, -9.0), Vec3::new(1.0, 0.0, 0.3)),
            (Vec3::new(10.0, 12.0, 8.0), Vec3::new(-0.3, 0.0, -1.0)),
            (Vec3::new(-9.0, -4.0, 10.0), Vec3::new(0.3, 1.0, 0.3)),
        ];
        let walkers = starts
            .iter()
            .map(|&(position, heading)| {
                let mut vehicle = Vehicle {
                    position,
                    max_speed: 2.0,
                    max_force: 8.0,
                    ..Vehicle::default()
                };
                vehicle.regenerate_basis(heading.normalize());
                Walker {
                    vehicle,
                    trail: Trail::default(),
                }
            })
            .collect();
        Self {
            walkers,
            offset,
            look_ahead,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for w in &mut self.walkers {
            let v = &w.vehicle;
            let force = v.forward + follow_surface(v, &BLOCK, self.offset, self.look_ahead);
            w.vehicle.apply_steering_force(force, dt);
            w.trail.record(self.time, w.vehicle.position);
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        match id {
            "offset" => self.offset = OFFSET.clamp(value),
            "look_ahead" => self.look_ahead = LOOK_AHEAD.clamp(value),
            _ => {}
        }
    }

    pub fn view(&self) -> Frame3View {
        let first = &self.walkers[0].vehicle;
        let mut lines = block_edges(&BLOCK, color::OBSTACLE);
        let target = follow_surface_target(first, &BLOCK, self.offset, self.look_ahead);
        lines.extend(cross(target, 0.3, color::TARGET));
        let push = first.forward + follow_surface(first, &BLOCK, self.offset, self.look_ahead);
        lines.push(line(first.position, first.position + push, color::PUSH));
        lines.extend(self.walkers.iter().map(|w| stem(w.vehicle.position)));
        Frame3View {
            time: self.time,
            focus: [0.0, 4.0, 0.0],
            view_size: 40.0,
            vehicles: self
                .walkers
                .iter()
                .map(|w| vehicle_view(&w.vehicle, color::BODY))
                .collect(),
            lines,
            trails: self
                .walkers
                .iter()
                .map(|w| w.trail.view(w.vehicle.position))
                .collect(),
            labels: vec![label(
                first.position,
                format!(
                    "walker 0, {:.2} from the wall",
                    BLOCK.signed_distance(first.position)
                ),
                color::LABEL,
            )],
            legend: legend(&[
                (color::OBSTACLE, "wall: a floating block"),
                (
                    color::TARGET,
                    "walker 0's target: offset out from the surface",
                ),
                (
                    color::PUSH,
                    "walker 0's push = forward + seek of the target",
                ),
                (color::EDGE, "height: drop line to the ground (y = 0)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: Vec::new(),
            params: vec![
                slider("offset", "offset from the wall", OFFSET, self.offset),
                slider("look_ahead", "look-ahead (s)", LOOK_AHEAD, self.look_ahead),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distance from the block's surface over all walkers from 15 s to 60 s: (min, mean, max).
    fn distances(offset: f32, look_ahead: f32) -> (f32, f32, f32) {
        let mut demo = WallFollowing3::new(offset, look_ahead);
        let (mut min, mut sum, mut max, mut n) = (f32::INFINITY, 0.0, 0.0_f32, 0);
        for step in 0..60 * 60 {
            demo.update(1.0 / 60.0);
            if step < 15 * 60 {
                continue;
            }
            for w in &demo.walkers {
                let d = BLOCK.signed_distance(w.vehicle.position);
                (min, max, sum, n) = (min.min(d), max.max(d), sum + d, n + 1);
            }
        }
        (min, sum / n as f32, max)
    }

    fn assert_near(got: (f32, f32, f32), want: (f32, f32, f32)) {
        let close = |a: f32, b: f32| (a - b).abs() < 0.01;
        assert!(
            close(got.0, want.0) && close(got.1, want.1) && close(got.2, want.2),
            "{got:?}"
        );
    }

    /// The walkers never touch the block: gap 2 keeps them 1.68-3.37 from it (mean 2.37), a
    /// 3 s look-ahead rides closer (1.16-2.57), gap 4: 3.82-4.87, gap 1: 0.62-2.52. Seeking a
    /// point on the rounded offset surface turns corners smoothly (the 2D outline version
    /// overshot each corner by about 1.4).
    #[test]
    fn walkers_keep_near_the_gap() {
        assert_near(distances(2.0, 1.0), (1.683, 2.372, 3.365));
        assert_near(distances(2.0, 3.0), (1.160, 2.074, 2.575));
        assert_near(distances(4.0, 1.0), (3.825, 4.260, 4.874));
        assert_near(distances(1.0, 1.0), (0.618, 1.396, 2.517));
    }
}
