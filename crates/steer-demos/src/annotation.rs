//! Annotation helpers ported from OpenSteer's `AnnotationMixin` and `SimpleVehicle`.

use std::collections::VecDeque;

use steer_core::{Vec2, Vehicle};

use crate::view::{LabelView, LegendView, LineView, TrailView, VehicleView};

/// One meaning per color, shared by every demo. Velocity, acceleration and trail keep
/// OpenSteer's colors; red and green are kept for the push and the want.
pub mod color {
    /// have: current velocity (OpenSteer's pinkish).
    pub const HAVE: u32 = 0xff66ff;
    /// change: smoothed acceleration (OpenSteer's bluish).
    pub const CHANGE: u32 = 0x6666ff;
    /// push: the steering force.
    pub const PUSH: u32 = 0xff5040;
    /// want: the desired velocity.
    pub const WANT: u32 = 0x3fd27a;
    pub const TARGET: u32 = 0xffd23f;
    /// The trail, `drawTrail`'s 70% gray.
    pub const TRAIL: u32 = 0xb3b3b3;
    /// Text labels: neutral, so red and green only ever mean push and want.
    pub const LABEL: u32 = 0xd0d4dc;
    pub const BODY: u32 = 0x808080;
}

pub fn legend(entries: &[(u32, &str)]) -> Vec<LegendView> {
    entries
        .iter()
        .map(|&(color, label)| LegendView {
            color,
            label: label.to_string(),
        })
        .collect()
}

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

    /// The trail up to the vehicle's current position.
    pub fn view(&self, current: Vec2) -> TrailView {
        TrailView {
            points: self
                .points
                .iter()
                .chain([&current])
                .map(|p| [p.x, p.y])
                .collect(),
            color: color::TRAIL,
        }
    }
}

pub fn point(p: Vec2) -> [f32; 2] {
    [p.x, p.y]
}

pub fn line(from: Vec2, to: Vec2, color: u32) -> LineView {
    LineView {
        from: point(from),
        to: point(to),
        color,
    }
}

pub fn label(at: Vec2, text: impl Into<String>, color: u32) -> LabelView {
    LabelView {
        at: point(at),
        text: text.into(),
        color,
    }
}

/// `drawBasic2dCircularVehicle`.
pub fn vehicle_view(v: &Vehicle, color: u32) -> VehicleView {
    VehicleView {
        position: point(v.position),
        forward: point(v.forward),
        radius: v.radius,
        color,
    }
}

/// `annotationVelocityAcceleration(3, 3)`: velocity (pinkish) and smoothed acceleration
/// (bluish), each scaled so its maximum is 3 units long.
pub fn velocity_acceleration(v: &Vehicle) -> [LineView; 2] {
    let p = v.position;
    [
        line(p, p + v.velocity() * (3.0 / v.max_speed), color::HAVE),
        line(
            p,
            p + v.smoothed_acceleration * (3.0 / v.max_force),
            color::CHANGE,
        ),
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
