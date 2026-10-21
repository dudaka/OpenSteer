//! "Wall Following" (Reynolds 1999, not in OpenSteer, so no C++ oracle): walkers circle an
//! L-shaped building at a fixed offset. Wall following is path following along the outline
//! `offset` outside the wall (`steer_core::wall::wall_path`); each walker steers with
//! `forward + steerToFollowPath`, as Pedestrian does without wander.

use steer_core::pathway::Pathway;
use steer_core::steer::{follow_path, follow_path_target};
use steer_core::wall::{wall_path, Wall};
use steer_core::{Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, vehicle_view, Trail};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LineView};

const OFFSET: Range = Range::new(0.5, 6.0, 0.25);
const LOOK_AHEAD: Range = Range::new(0.25, 3.0, 0.25);
/// Half-width of the band around the offset outline the walkers may drift in.
const PATH_RADIUS: f32 = 0.5;

#[derive(Debug, Clone)]
struct Walker {
    vehicle: Vehicle,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct WallFollowing {
    wall: Wall,
    path: Pathway,
    walkers: Vec<Walker>,
    offset: f32,
    look_ahead: f32,
    clockwise: bool,
    time: f32,
}

impl Default for WallFollowing {
    fn default() -> Self {
        Self::new(2.0, 1.0, false)
    }
}

/// A 12 x 5 bar with a 5 x 5 block on its left end, centered near the origin.
fn building() -> Wall {
    let corners = [
        (0., 0.),
        (12., 0.),
        (12., 5.),
        (5., 5.),
        (5., 10.),
        (0., 10.),
    ];
    let points: Vec<Vec2> = corners
        .iter()
        .map(|&(x, y)| Vec2::new(x - 6.0, y - 5.0))
        .collect();
    Wall::new(&points)
}

impl WallFollowing {
    /// Three walkers at rest, away from the building, facing different ways.
    fn new(offset: f32, look_ahead: f32, clockwise: bool) -> Self {
        let wall = building();
        let starts = [
            (-14.0, -10.0, 1.0, 0.0),
            (12.0, 12.0, 0.0, -1.0),
            (-12.0, 12.0, 0.0, 1.0),
        ];
        let walkers = starts
            .iter()
            .map(|&(x, y, fx, fy)| Walker {
                vehicle: Vehicle {
                    position: Vec2::new(x, y),
                    forward: Vec2::new(fx, fy),
                    max_speed: 2.0,
                    max_force: 8.0,
                    ..Vehicle::default()
                },
                trail: Trail::default(),
            })
            .collect();
        Self {
            path: wall_path(&wall, offset, PATH_RADIUS, clockwise),
            wall,
            walkers,
            offset,
            look_ahead,
            clockwise,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for w in &mut self.walkers {
            let v = &w.vehicle;
            let force = v.forward + follow_path(v, 1.0, self.look_ahead, &self.path);
            w.vehicle.apply_steering_force(force, dt);
            w.trail.record(self.time, w.vehicle.position);
        }
    }

    pub fn action(&mut self, id: &str) {
        match id {
            "counter_clockwise" => self.set_direction(false),
            "clockwise" => self.set_direction(true),
            _ => {}
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        match id {
            "offset" => {
                self.offset = OFFSET.clamp(value);
                self.path = wall_path(&self.wall, self.offset, PATH_RADIUS, self.clockwise);
            }
            "look_ahead" => self.look_ahead = LOOK_AHEAD.clamp(value),
            _ => {}
        }
    }

    pub fn view(&self) -> FrameView {
        let first = &self.walkers[0].vehicle;
        let mut lines = outline(self.wall.points(), color::OBSTACLE);
        lines.extend(outline(&self.path.points()[1..], color::PATH));
        if let Some(target) = follow_path_target(first, 1.0, self.look_ahead, &self.path) {
            lines.extend(circle(target, 0.3, color::TARGET));
        }
        let push = first.forward + follow_path(first, 1.0, self.look_ahead, &self.path);
        lines.push(line(first.position, first.position + push, color::PUSH));
        FrameView {
            time: self.time,
            focus: [0.0, 0.0],
            view_size: 34.0,
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
                    self.wall.distance(first.position)
                ),
                color::LABEL,
            )],
            legend: legend(&[
                (color::OBSTACLE, "wall"),
                (color::PATH, "the outline `offset` outside the wall"),
                (color::TARGET, "walker 0's seek target, when off course"),
                (color::PUSH, "walker 0's push = forward + path force"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![
                button("counter_clockwise", "Counter-clockwise", !self.clockwise),
                button("clockwise", "Clockwise", self.clockwise),
            ],
            params: vec![
                slider("offset", "offset from the wall", OFFSET, self.offset),
                slider("look_ahead", "look-ahead (s)", LOOK_AHEAD, self.look_ahead),
            ],
        }
    }

    /// Restarts with the walkers going the other way round.
    fn set_direction(&mut self, clockwise: bool) {
        *self = Self::new(self.offset, self.look_ahead, clockwise);
    }
}

/// A closed polygon's edges.
fn outline(points: &[Vec2], color: u32) -> Vec<LineView> {
    let n = points.len();
    (0..n)
        .map(|i| line(points[i], points[(i + 1) % n], color))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distance from the wall over all walkers from 15 s to 60 s: (min, mean, max).
    fn distances(demo: &mut WallFollowing) -> (f32, f32, f32) {
        let (mut min, mut sum, mut max, mut n) = (f32::INFINITY, 0.0, 0.0_f32, 0);
        for step in 0..60 * 60 {
            demo.update(1.0 / 60.0);
            if step < 15 * 60 {
                continue;
            }
            for w in &demo.walkers {
                let d = demo.wall.distance(w.vehicle.position);
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

    /// Offset 2, band +-0.5: the walkers overshoot each 90-degree turn of the outline by
    /// about 1.4, wide after the outer corners (3.38) and toward the wall after the inner
    /// corner (0.58). A 3 s look-ahead cuts corners and rides closer (mean 1.48).
    #[test]
    fn walkers_keep_near_the_offset_but_overshoot_corners() {
        assert_near(
            distances(&mut WallFollowing::new(2.0, 1.0, false)),
            (0.585, 2.306, 3.385),
        );
        assert_near(
            distances(&mut WallFollowing::new(2.0, 1.0, true)),
            (0.814, 2.348, 3.405),
        );
        assert_near(
            distances(&mut WallFollowing::new(2.0, 3.0, false)),
            (0.527, 1.485, 3.476),
        );
        assert_near(
            distances(&mut WallFollowing::new(4.0, 1.0, false)),
            (2.642, 4.406, 5.790),
        );
    }

    #[test]
    fn direction_buttons_restart_the_other_way() {
        let mut demo = WallFollowing::default();
        demo.update(1.0 / 60.0);
        demo.action("clockwise");
        assert!(demo.clockwise && demo.time == 0.0);
        assert_eq!(demo.path.points()[0], demo.wall.offset(2.0)[5]);
    }
}
