//! "Boids" (`plugins/Boids.cpp`, flattened to the ground): each boid steers with
//! `12 * separation + 8 * alignment + 8 * cohesion` (weights adjustable) over the neighbors in
//! its boid neighborhood, and leaves the world circle only to reenter on the opposite side.
//! Neighbor candidates come from a grid kept current as boids move (as each boid's
//! `proximityToken` is); the behaviors still filter them exactly.

use std::f32::consts::TAU;

use steer_core::flock::{alignment, cohesion, in_neighborhood, separation};
use steer_core::proximity::Grid;
use steer_core::{Vec2, Vehicle};

use crate::annotation::{circle, color, label, legend, line, vehicle_view};
use crate::controls::{slider, Range};
use crate::view::{FrameView, LineView};

const BOIDS: Range = Range::new(10.0, 300.0, 10.0);
const WEIGHT: Range = Range::new(0.0, 20.0, 1.0);
const SEED: u64 = 1;
const WORLD_RADIUS: f32 = 50.0;
/// The largest of the three neighborhood radii.
const MAX_RADIUS: f32 = 9.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Weights {
    separation: f32,
    alignment: f32,
    cohesion: f32,
}

#[derive(Debug, Clone)]
pub struct Boids {
    boids: Vec<Vehicle>,
    weights: Weights,
    time: f32,
}

impl Default for Boids {
    fn default() -> Self {
        Self::new(
            100,
            Weights {
                separation: 12.0,
                alignment: 8.0,
                cohesion: 8.0,
            },
        )
    }
}

impl Boids {
    /// `Boid::reset`: within 20 of the center, random heading, at 30% of max speed 9, max force
    /// 27.
    fn new(count: usize, weights: Weights) -> Self {
        let mut rng = fastrand::Rng::with_seed(SEED);
        let boids = (0..count)
            .map(|_| {
                let (angle, r) = (rng.f32() * TAU, rng.f32().sqrt() * 20.0);
                let heading = rng.f32() * TAU;
                Vehicle {
                    position: Vec2::new(angle.cos(), angle.sin()) * r,
                    forward: Vec2::new(heading.cos(), heading.sin()),
                    speed: 9.0 * 0.3,
                    max_force: 27.0,
                    max_speed: 9.0,
                    ..Vehicle::default()
                }
            })
            .collect();
        Self {
            boids,
            weights,
            time: 0.0,
        }
    }

    /// Each boid in turn, seeing the others' latest positions (as `BoidsPlugIn::update`).
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        let mut positions: Vec<Vec2> = self.boids.iter().map(|b| b.position).collect();
        let mut grid = Grid::new(&positions, MAX_RADIUS);
        for i in 0..self.boids.len() {
            // A little past the largest radius: the behaviors count a boid at exactly 9.
            let candidates = grid.neighbors(&positions, positions[i], MAX_RADIUS + 0.1);
            let force = self.steer_to_flock(i, &candidates);
            let b = &mut self.boids[i];
            b.apply_steering_force(force, dt);
            b.position = wrap_around(b.position, WORLD_RADIUS);
            grid.move_point(i, positions[i], b.position);
            positions[i] = b.position;
        }
    }

    pub fn param(&mut self, id: &str, value: f32) {
        let w = &mut self.weights;
        match id {
            "separation" => w.separation = WEIGHT.clamp(value),
            "alignment" => w.alignment = WEIGHT.clamp(value),
            "cohesion" => w.cohesion = WEIGHT.clamp(value),
            "boids" => *self = Self::new(BOIDS.clamp(value) as usize, self.weights),
            _ => {}
        }
    }

    pub fn view(&self) -> FrameView {
        let first = &self.boids[0];
        let mut lines = circle(Vec2::ZERO, WORLD_RADIUS, color::EDGE);
        lines.extend(self.neighbor_links());
        let everyone: Vec<usize> = (0..self.boids.len()).collect();
        let push = self.steer_to_flock(0, &everyone);
        lines.push(line(
            first.position,
            first.position + push * 0.2,
            color::PUSH,
        ));
        FrameView {
            time: self.time,
            focus: [0.0, 0.0],
            view_size: 110.0,
            vehicles: self
                .boids
                .iter()
                .map(|b| vehicle_view(b, color::BODY))
                .collect(),
            lines,
            trails: Vec::new(),
            labels: vec![label(first.position, "boid 0", color::LABEL)],
            legend: legend(&[
                (color::EDGE, "world edge: leave here, reenter opposite"),
                (color::RADIUS, "boid 0's flockmates (in any neighborhood)"),
                (color::PUSH, "boid 0's push, drawn at 1/5"),
            ]),
            actions: Vec::new(),
            params: vec![
                slider("boids", "boids", BOIDS, self.boids.len() as f32),
                slider("separation", "separation", WEIGHT, self.weights.separation),
                slider("alignment", "alignment", WEIGHT, self.weights.alignment),
                slider("cohesion", "cohesion", WEIGHT, self.weights.cohesion),
            ],
        }
    }

    /// `Boid::steerToFlock` without obstacles, over the candidate indices (`i` itself skipped).
    fn steer_to_flock(&self, i: usize, candidates: &[usize]) -> Vec2 {
        let v = &self.boids[i];
        let others = || {
            candidates
                .iter()
                .filter(move |&&j| j != i)
                .map(|&j| &self.boids[j])
        };
        let w = self.weights;
        separation(v, 5.0, -0.707, others()) * w.separation
            + alignment(v, 7.5, 0.7, others()) * w.alignment
            + cohesion(v, 9.0, -0.15, others()) * w.cohesion
    }

    /// Lines from boid 0 to every boid inside any of its three neighborhoods.
    fn neighbor_links(&self) -> Vec<LineView> {
        let me = &self.boids[0];
        self.boids[1..]
            .iter()
            .filter(|o| {
                in_neighborhood(me, o, 5.0, -0.707)
                    || in_neighborhood(me, o, 7.5, 0.7)
                    || in_neighborhood(me, o, 9.0, -0.15)
            })
            .map(|o| line(me.position, o.position, color::RADIUS))
            .collect()
    }
}

/// `sphericalWrapAround` on the ground: leaving the circle reenters on the opposite side.
fn wrap_around(p: Vec2, radius: f32) -> Vec2 {
    let r = p.length();
    if r > radius {
        p + (p / r) * radius * -2.0
    } else {
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// The same steps with every boid as a candidate: the grid must not change anything.
    fn update_with_whole_flock(demo: &mut Boids) {
        let everyone: Vec<usize> = (0..demo.boids.len()).collect();
        for i in 0..demo.boids.len() {
            let force = demo.steer_to_flock(i, &everyone);
            let b = &mut demo.boids[i];
            b.apply_steering_force(force, DT);
            b.position = wrap_around(b.position, WORLD_RADIUS);
        }
    }

    #[test]
    fn grid_neighbors_give_the_same_flock_as_brute_force() {
        let (mut grid, mut brute) = (Boids::default(), Boids::default());
        for _ in 0..600 {
            grid.update(DT);
            update_with_whole_flock(&mut brute);
        }
        assert_eq!(grid.boids, brute.boids);
    }

    #[test]
    fn boids_stay_inside_the_world() {
        let mut demo = Boids::default();
        for _ in 0..1800 {
            demo.update(DT);
            assert!(demo
                .boids
                .iter()
                .all(|b| b.position.length() <= WORLD_RADIUS + 0.2));
        }
    }

    /// Alignment makes headings agree: the mean heading's length grows from a random start.
    #[test]
    fn the_flock_lines_up() {
        let order = |d: &Boids| {
            let sum = d.boids.iter().fold(Vec2::ZERO, |s, b| s + b.forward);
            sum.length() / d.boids.len() as f32
        };
        let mut demo = Boids::default();
        let start = order(&demo);
        for _ in 0..1800 {
            demo.update(DT);
        }
        assert!(order(&demo) > start + 0.3, "{start} -> {}", order(&demo));
    }

    #[test]
    fn sliders_set_weights_and_count() {
        let mut demo = Boids::default();
        demo.param("alignment", 3.0);
        demo.param("boids", 40.0);
        assert_eq!((demo.weights.alignment, demo.boids.len()), (3.0, 40));
    }
}
