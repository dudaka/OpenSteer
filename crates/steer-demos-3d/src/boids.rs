//! "3D Boids": the Boids plug-in as OpenSteer has it (`plugins/Boids.cpp`): boids fly and bank
//! inside a world sphere of radius 50, leaving it only to reenter on the opposite side. Each
//! steers with `12 * separation + 8 * alignment + 8 * cohesion` (weights adjustable) and sees
//! the whole flock (the behaviors filter by distance; no proximity grid in 3D yet).

use steer_core_3d::flock::{alignment, cohesion, in_neighborhood, separation};
use steer_core_3d::util::{random_unit_vector, random_vector_in_unit_sphere};
use steer_core_3d::{Vec3, Vehicle};
use steer_demos::controls::{slider, Range};
use steer_demos::{color, legend};

use crate::annotation::{label, line, sphere, vehicle_view};
use crate::view::{Frame3View, Line3View};

/// Every boid looks at every other one (no grid yet), so the count stays where a debug build
/// keeps up with 60 steps a second.
const BOIDS: Range = Range::new(10.0, 150.0, 10.0);
const WEIGHT: Range = Range::new(0.0, 20.0, 1.0);
const WORLD_RADIUS: f32 = 50.0;
const SEED: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Weights {
    separation: f32,
    alignment: f32,
    cohesion: f32,
}

#[derive(Debug, Clone)]
pub struct Boids3 {
    boids: Vec<Vehicle>,
    weights: Weights,
    /// Times a boid left the world and reentered opposite, since the start.
    wraps: u32,
    time: f32,
}

impl Default for Boids3 {
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

impl Boids3 {
    /// `Boid::reset` for each: max force 27, max speed 9, speed 2.7, banking, a random heading,
    /// then a random spot within 20 of the center.
    fn new(count: usize, weights: Weights) -> Self {
        let mut rng = fastrand::Rng::with_seed(SEED);
        let boids = (0..count)
            .map(|_| {
                let mut b = Vehicle {
                    max_force: 27.0,
                    max_speed: 9.0,
                    banking: true,
                    ..Vehicle::default()
                };
                b.speed = b.max_speed * 0.3;
                b.regenerate_basis(random_unit_vector(&mut || rng.f32()));
                b.position = random_vector_in_unit_sphere(&mut || rng.f32()) * 20.0;
                b
            })
            .collect();
        Self {
            boids,
            weights,
            wraps: 0,
            time: 0.0,
        }
    }

    /// Each boid in turn, seeing the others' latest positions (as `BoidsPlugIn::update`).
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for i in 0..self.boids.len() {
            let force = self.steer_to_flock(i);
            let b = &mut self.boids[i];
            b.apply_steering_force(force, dt);
            if b.position.length() > WORLD_RADIUS {
                b.position = b.position.spherical_wrap_around(Vec3::ZERO, WORLD_RADIUS);
                self.wraps += 1;
            }
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

    pub fn view(&self) -> Frame3View {
        let first = &self.boids[0];
        let mut lines = sphere(Vec3::ZERO, WORLD_RADIUS, color::EDGE);
        lines.extend(self.neighbor_links());
        let push = self.steer_to_flock(0);
        lines.push(line(
            first.position,
            first.position + push * 0.2,
            color::PUSH,
        ));
        Frame3View {
            time: self.time,
            focus: [0.0, 0.0, 0.0],
            view_size: 150.0,
            vehicles: self
                .boids
                .iter()
                .map(|b| vehicle_view(b, color::BODY))
                .collect(),
            lines,
            trails: Vec::new(),
            labels: vec![
                label(first.position, "boid 0", color::LABEL),
                label(
                    Vec3::new(0.0, WORLD_RADIUS + 4.0, 0.0),
                    format!("wrap-arounds: {}", self.wraps),
                    color::LABEL,
                ),
            ],
            legend: legend(&[
                (color::EDGE, "world sphere: leave here, reenter opposite"),
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

    /// `Boid::steerToFlock` without obstacles, over every other boid.
    fn steer_to_flock(&self, i: usize) -> Vec3 {
        let v = &self.boids[i];
        let others = || {
            self.boids
                .iter()
                .enumerate()
                .filter_map(move |(j, o)| (j != i).then_some(o))
        };
        let w = self.weights;
        separation(v, 5.0, -0.707, others()) * w.separation
            + alignment(v, 7.5, 0.7, others()) * w.alignment
            + cohesion(v, 9.0, -0.15, others()) * w.cohesion
    }

    /// Lines from boid 0 to every boid inside any of its three neighborhoods.
    fn neighbor_links(&self) -> Vec<Line3View> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 100 boids for 10 s: 93 wrap-arounds; they fly in all three dimensions (heights from
    /// -36.4 to 39.3) near top speed (mean 8.59 of 9).
    #[test]
    fn the_flock_flies_in_3d_and_wraps() {
        let mut demo = Boids3::default();
        for _ in 0..600 {
            demo.update(1.0 / 60.0);
        }
        assert_eq!(demo.wraps, 93);
        let ys = demo.boids.iter().map(|b| b.position.y);
        let (lo, hi) = ys.fold((f32::MAX, f32::MIN), |(a, b), y| (a.min(y), b.max(y)));
        assert!(
            (lo + 36.355).abs() < 0.01 && (hi - 39.309).abs() < 0.01,
            "{lo} {hi}"
        );
        let mean_speed = demo.boids.iter().map(|b| b.speed).sum::<f32>() / 100.0;
        assert!((mean_speed - 8.587).abs() < 0.01, "{mean_speed}");
    }

    #[test]
    fn weight_and_count_sliders() {
        let mut demo = Boids3::default();
        demo.param("cohesion", 3.0);
        assert_eq!(demo.weights.cohesion, 3.0);
        demo.param("boids", 500.0);
        assert_eq!(demo.boids.len(), 150);
    }
}
