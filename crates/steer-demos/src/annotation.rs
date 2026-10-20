//! Annotation helpers ported from OpenSteer's `AnnotationMixin` and `SimpleVehicle`.

use std::collections::VecDeque;

use steer_core::{Vec2, Vehicle};

use crate::view::{LineView, TrailView};

/// OpenSteer's default trail: 5 seconds of history in 100 samples.
const TRAIL_DURATION: f32 = 5.0;
const TRAIL_VERTEX_COUNT: usize = 100;

/// A vehicle's recent positions, sampled at a fixed interval (`recordTrailVertex`).
#[derive(Debug, Clone, Default)]
pub struct Trail {
    points: VecDeque<Vec2>,
    last_sample_time: f32,
}

impl Trail {
    pub fn record(&mut self, time: f32, position: Vec2) {
        if time - self.last_sample_time <= TRAIL_DURATION / TRAIL_VERTEX_COUNT as f32 {
            return;
        }
        if self.points.len() == TRAIL_VERTEX_COUNT {
            self.points.pop_front();
        }
        self.points.push_back(position);
        self.last_sample_time = time;
    }

    /// The trail up to the vehicle's current position, in `drawTrail`'s 70% gray.
    pub fn view(&self, current: Vec2) -> TrailView {
        TrailView {
            points: self
                .points
                .iter()
                .chain([&current])
                .map(|p| [p.x, p.y])
                .collect(),
            color: 0xb3b3b3,
        }
    }
}

/// `annotationVelocityAcceleration(3, 3)`: velocity (pinkish) and smoothed acceleration
/// (bluish), each scaled so its maximum is 3 units long.
pub fn velocity_acceleration(v: &Vehicle) -> [LineView; 2] {
    let line = |offset: Vec2, color| LineView {
        from: [v.position.x, v.position.y],
        to: [v.position.x + offset.x, v.position.y + offset.y],
        color,
    };
    [
        line(v.velocity() * (3.0 / v.max_speed), 0xff66ff),
        line(v.smoothed_acceleration * (3.0 / v.max_force), 0x6666ff),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trail_samples_every_005_s_and_keeps_the_last_100() {
        let mut trail = Trail::default();
        for step in 1..=600 {
            trail.record(step as f32 / 60.0, Vec2::new(step as f32, 0.0));
        }
        let view = trail.view(Vec2::new(601.0, 0.0));
        assert_eq!(view.points.len(), TRAIL_VERTEX_COUNT + 1);
        assert_eq!(view.points.last(), Some(&[601.0, 0.0]));
        // A sample every 0.05 s is every 3 or 4 steps of 1/60 s.
        let samples = &view.points[..TRAIL_VERTEX_COUNT];
        assert!(samples
            .windows(2)
            .all(|w| matches!(w[1][0] - w[0][0], 3.0 | 4.0)));
    }

    #[test]
    fn annotation_lines_are_scaled_to_three_units_at_max() {
        let v = Vehicle {
            speed: 5.0,
            max_speed: 5.0,
            ..Vehicle::default()
        };
        let [velocity, _] = velocity_acceleration(&v);
        assert_eq!(velocity.to, [0.0, 3.0]);
    }
}
