//! "3D Crossing": walkers spread over a sphere (a Fibonacci spiral, as
//! `oracle/neighbors_3d.cpp`) each walk to the opposite point and back, so every path crosses
//! the center. With avoidance on they steer with `forward + 10 * steerToAvoidNeighbors` when it
//! is nonzero (`Pedestrian.cpp`'s priority), otherwise `forward + seek(target)`.

use steer_core_3d::neighbors::{avoid_neighbors, threat};
use steer_core_3d::{steer, Vec3, Vehicle};
use steer_demos::controls::{button, slider, Range};
use steer_demos::{color, legend};

use crate::annotation::{label, line, sphere, vehicle_view, Trail};
use crate::view::{Frame3View, Line3View};

const WALKERS: Range = Range::new(2.0, 30.0, 1.0);
const SPHERE_RADIUS: f32 = 10.0;
/// `steerToAvoidNeighbors`' minTimeToCollision (`caLeadTime`).
const LEAD_TIME: f32 = 3.0;

#[derive(Debug, Clone)]
struct Walker {
    vehicle: Vehicle,
    target: Vec3,
    trail: Trail,
}

#[derive(Debug, Clone)]
pub struct Crossing3 {
    walkers: Vec<Walker>,
    avoid: bool,
    /// Steps on which some pair of bodies overlapped.
    overlap_steps: u32,
    time: f32,
}

impl Default for Crossing3 {
    fn default() -> Self {
        Self::new(12, true)
    }
}

/// Point `i` of `count` on the Fibonacci sphere: heights evenly spaced, golden-angle turns.
fn start(i: usize, count: usize) -> Vec3 {
    let y = 1.0 - (i as f32 + 0.5) * (2.0 / count as f32);
    let r = (1.0 - y * y).sqrt();
    let angle = i as f32 * 2.399_963_2;
    Vec3::new(r * angle.cos(), y, r * angle.sin()) * SPHERE_RADIUS
}

impl Crossing3 {
    fn new(count: usize, avoid: bool) -> Self {
        let walkers = (0..count)
            .map(|i| {
                let s = start(i, count);
                let mut vehicle = Vehicle {
                    position: s,
                    speed: 1.0,
                    max_speed: 2.0,
                    max_force: 8.0,
                    ..Vehicle::default()
                };
                vehicle.regenerate_basis((-s - s).normalize());
                Walker {
                    vehicle,
                    target: -s,
                    trail: Trail::default(),
                }
            })
            .collect();
        Self {
            walkers,
            avoid,
            overlap_steps: 0,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for i in 0..self.walkers.len() {
            let force = self.force(i);
            let w = &mut self.walkers[i];
            w.vehicle.apply_steering_force(force, dt);
            w.trail.record(self.time, w.vehicle.position);
            // Arrived: walk back.
            if (w.target - w.vehicle.position).length() < 1.0 {
                w.target = -w.target;
            }
        }
        if self.any_overlap() {
            self.overlap_steps += 1;
        }
    }

    pub fn action(&mut self, id: &str) {
        if id == "avoid" {
            *self = Self::new(self.walkers.len(), !self.avoid);
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        if id == "walkers" {
            *self = Self::new(WALKERS.clamp(value) as usize, self.avoid);
        }
    }

    pub fn view(&self) -> Frame3View {
        let first = &self.walkers[0].vehicle;
        let mut lines = sphere(Vec3::ZERO, SPHERE_RADIUS, color::PATH);
        lines.extend(self.threat_lines());
        Frame3View {
            time: self.time,
            focus: [0.0, 0.0, 0.0],
            view_size: 32.0,
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
            labels: vec![
                label(first.position, "walker 0", color::LABEL),
                label(
                    Vec3::new(0.0, SPHERE_RADIUS + 1.5, 0.0),
                    format!("steps with bodies overlapping: {}", self.overlap_steps),
                    color::LABEL,
                ),
            ],
            legend: legend(&[
                (
                    color::PATH,
                    "start sphere: each walks to the opposite point",
                ),
                (
                    color::RADIUS,
                    "walker 0: both positions at nearest approach",
                ),
                (color::PUSH, "walker 0's avoidance push (x 10)"),
                (color::TRAIL, "trail: the last 5 s"),
            ]),
            actions: vec![button("avoid", "Avoid neighbors", self.avoid)],
            params: vec![slider(
                "walkers",
                "walkers",
                WALKERS,
                self.walkers.len() as f32,
            )],
        }
    }

    fn others(&self, i: usize) -> Vec<&Vehicle> {
        self.walkers
            .iter()
            .enumerate()
            .filter_map(|(j, w)| (j != i).then_some(&w.vehicle))
            .collect()
    }

    fn avoidance(&self, i: usize) -> Vec3 {
        if !self.avoid {
            return Vec3::ZERO;
        }
        avoid_neighbors(&self.walkers[i].vehicle, LEAD_TIME, &self.others(i)) * 10.0
    }

    fn force(&self, i: usize) -> Vec3 {
        let w = &self.walkers[i];
        let avoidance = self.avoidance(i);
        if avoidance != Vec3::ZERO {
            w.vehicle.forward + avoidance
        } else {
            w.vehicle.forward + steer::seek(&w.vehicle, w.target)
        }
    }

    /// Walker 0's predicted nearest approach to its threat, and its push.
    fn threat_lines(&self) -> Vec<Line3View> {
        let me = &self.walkers[0].vehicle;
        let mut lines = Vec::new();
        if !self.avoid {
            return lines;
        }
        if let Some(t) = threat(me, LEAD_TIME, &self.others(0)) {
            lines.push(line(me.position, t.ours, color::RADIUS));
            lines.push(line(t.ours, t.theirs, color::RADIUS));
        }
        let push = self.avoidance(0);
        if push != Vec3::ZERO {
            lines.push(line(me.position, me.position + push * 0.1, color::PUSH));
        }
        lines
    }

    fn any_overlap(&self) -> bool {
        let v: Vec<&Vehicle> = self.walkers.iter().map(|w| &w.vehicle).collect();
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
        let mut demo = Crossing3::new(12, avoid);
        for _ in 0..3600 {
            demo.update(1.0 / 60.0);
        }
        demo.overlap_steps
    }

    /// 12 walkers, one minute: avoidance cuts the steps with bodies overlapping from 570 to
    /// 125 (in 2D, 8 walkers: 847 to 87).
    #[test]
    fn avoidance_reduces_overlaps() {
        assert_eq!(overlaps_in_a_minute(false), 570);
        assert_eq!(overlaps_in_a_minute(true), 125);
    }

    /// The default is the oracle's scenario (`oracle_neighbors_3d`, walker 0 at step 150).
    #[test]
    fn runs_the_oracle_scenario() {
        let mut demo = Crossing3::default();
        for _ in 0..150 {
            demo.update(1.0 / 60.0);
        }
        let p = demo.walkers[0].vehicle.position;
        assert!(
            (p - Vec3::new(2.160565, 4.955574, 0.190299)).length() < 1e-4,
            "{p:?}"
        );
    }
}
