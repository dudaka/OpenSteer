//! "Pedestrians Crossing": pedestrians on a circle each walk to the opposite point and back
//! (`oracle/neighbors.cpp`). With avoidance on they steer with `forward + 10 *
//! steerToAvoidNeighbors` when it is nonzero (`Pedestrian.cpp`'s priority), otherwise
//! `forward + seek(target)`.

use std::f32::consts::TAU;

use steer_core::neighbors::{avoid_neighbors, threat};
use steer_core::{steer, Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, vehicle_view, Trail};
use crate::controls::{button, slider, Range};
use crate::view::{FrameView, LineView};

const PEDESTRIANS: Range = Range::new(2.0, 30.0, 1.0);
const CIRCLE_RADIUS: f32 = 10.0;
/// `steerToAvoidNeighbors`' minTimeToCollision (`caLeadTime`).
const LEAD_TIME: f32 = 3.0;

#[derive(Debug, Clone)]
struct Pedestrian {
    vehicle: Vehicle,
    target: Vec2,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct Crossing {
    pedestrians: Vec<Pedestrian>,
    avoid: bool,
    /// Steps on which some pair of bodies overlapped.
    overlap_steps: u32,
    time: f32,
}

impl Default for Crossing {
    fn default() -> Self {
        Self::new(8, true)
    }
}

impl Crossing {
    fn new(count: usize, avoid: bool) -> Self {
        let pedestrians = (0..count)
            .map(|i| {
                let angle = i as f32 * (TAU / count as f32);
                let start = Vec2::new(angle.cos(), angle.sin()) * CIRCLE_RADIUS;
                Pedestrian {
                    vehicle: Vehicle {
                        position: start,
                        forward: (-start - start).normalize(),
                        speed: 1.0,
                        max_speed: 2.0,
                        max_force: 8.0,
                        radius: 0.5,
                        ..Vehicle::default()
                    },
                    target: -start,
                    trail: Trail::default(),
                }
            })
            .collect();
        Self {
            pedestrians,
            avoid,
            overlap_steps: 0,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for i in 0..self.pedestrians.len() {
            let force = self.force(i);
            let p = &mut self.pedestrians[i];
            p.vehicle.apply_steering_force(force, dt);
            p.trail.record(self.time, p.vehicle.position);
            // Arrived: walk back.
            if (p.target - p.vehicle.position).length() < 1.0 {
                p.target = -p.target;
            }
        }
        if self.any_overlap() {
            self.overlap_steps += 1;
        }
    }

    pub fn action(&mut self, id: &str) {
        if id == "avoid" {
            *self = Self::new(self.pedestrians.len(), !self.avoid);
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        if id == "pedestrians" {
            *self = Self::new(PEDESTRIANS.clamp(value) as usize, self.avoid);
        }
    }

    pub fn view(&self) -> FrameView {
        let first = &self.pedestrians[0].vehicle;
        let mut lines = circle(Vec2::ZERO, CIRCLE_RADIUS, color::PATH);
        lines.extend(self.threat_lines());
        FrameView {
            time: self.time,
            focus: [0.0, 0.0],
            view_size: 26.0,
            vehicles: self
                .pedestrians
                .iter()
                .map(|p| vehicle_view(&p.vehicle, color::BODY))
                .collect(),
            lines,
            trails: self
                .pedestrians
                .iter()
                .map(|p| p.trail.view(p.vehicle.position))
                .collect(),
            labels: vec![
                label(first.position, "pedestrian 0", color::LABEL),
                label(
                    Vec2::new(0.0, -CIRCLE_RADIUS - 1.5),
                    format!("steps with bodies overlapping: {}", self.overlap_steps),
                    color::LABEL,
                ),
            ],
            legend: legend(&[
                (
                    color::PATH,
                    "start circle: each walks to the opposite point",
                ),
                (
                    color::RADIUS,
                    "pedestrian 0: both positions at nearest approach",
                ),
                (color::PUSH, "pedestrian 0's avoidance push (x 10)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![button("avoid", "Avoid neighbors", self.avoid)],
            params: vec![slider(
                "pedestrians",
                "pedestrians",
                PEDESTRIANS,
                self.pedestrians.len() as f32,
            )],
        }
    }

    fn others(&self, i: usize) -> Vec<&Vehicle> {
        self.pedestrians
            .iter()
            .enumerate()
            .filter_map(|(j, p)| (j != i).then_some(&p.vehicle))
            .collect()
    }

    fn avoidance(&self, i: usize) -> Vec2 {
        if !self.avoid {
            return Vec2::ZERO;
        }
        avoid_neighbors(&self.pedestrians[i].vehicle, LEAD_TIME, &self.others(i)) * 10.0
    }

    fn force(&self, i: usize) -> Vec2 {
        let p = &self.pedestrians[i];
        let avoidance = self.avoidance(i);
        if avoidance != Vec2::ZERO {
            p.vehicle.forward + avoidance
        } else {
            p.vehicle.forward + steer::seek(&p.vehicle, p.target)
        }
    }

    /// Pedestrian 0's predicted nearest approach to its threat, and its push.
    fn threat_lines(&self) -> Vec<LineView> {
        let me = &self.pedestrians[0].vehicle;
        let mut lines = Vec::new();
        if !self.avoid {
            return lines;
        }
        if let Some(t) = threat(me, LEAD_TIME, &self.others(0)) {
            lines.push(line(me.position, t.ours, color::RADIUS));
            lines.push(line(t.ours, t.theirs, color::RADIUS));
        }
        let push = self.avoidance(0);
        if push != Vec2::ZERO {
            lines.push(line(me.position, me.position + push * 0.1, color::PUSH));
        }
        lines
    }

    fn any_overlap(&self) -> bool {
        let v: Vec<&Vehicle> = self.pedestrians.iter().map(|p| &p.vehicle).collect();
        v.iter().enumerate().any(|(i, a)| {
            v[i + 1..]
                .iter()
                .any(|b| (a.position - b.position).length() < a.radius + b.radius)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlaps_in_a_minute(avoid: bool) -> u32 {
        let mut demo = Crossing::new(8, avoid);
        for _ in 0..3600 {
            demo.update(1.0 / 60.0);
        }
        demo.overlap_steps
    }

    /// Avoidance cuts overlapping steps from 847 to 57 in a minute, not to zero: it only
    /// reacts to the earliest threat, and overlaps get a sideways push only.
    #[test]
    fn avoidance_reduces_overlaps() {
        assert_eq!(overlaps_in_a_minute(false), 847);
        assert_eq!(overlaps_in_a_minute(true), 57);
    }

    /// Pedestrian 0 crosses (flip at 12.90 s) and comes back (23.78 s) within 30 s.
    #[test]
    fn pedestrians_walk_there_and_back() {
        let mut demo = Crossing::default();
        let mut flips = 0;
        let mut last = demo.pedestrians[0].target;
        for _ in 0..60 * 30 {
            demo.update(1.0 / 60.0);
            if demo.pedestrians[0].target != last {
                flips += 1;
                last = demo.pedestrians[0].target;
            }
        }
        assert_eq!(flips, 2);
    }

    #[test]
    fn toggling_restarts() {
        let mut demo = Crossing::default();
        demo.update(1.0 / 60.0);
        demo.action("avoid");
        assert!(!demo.avoid);
        assert_eq!(demo.time, 0.0);
    }
}
