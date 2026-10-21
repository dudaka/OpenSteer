//! 3D annotation helpers: the 2D demos' `annotation` with a third coordinate. Colors and
//! their meanings are shared with the 2D demos (`steer_demos::color`).

use std::collections::VecDeque;

use steer_core_3d::{Vec3, Vehicle};
use steer_demos::color;

use crate::view::{Label3View, Line3View, Trail3View, Vehicle3View};

pub fn point(p: Vec3) -> [f32; 3] {
    [p.x, p.y, p.z]
}

pub fn line(from: Vec3, to: Vec3, color: u32) -> Line3View {
    Line3View {
        from: point(from),
        to: point(to),
        color,
    }
}

pub fn label(at: Vec3, text: impl Into<String>, color: u32) -> Label3View {
    Label3View {
        at: point(at),
        text: text.into(),
        color,
    }
}

pub fn vehicle_view(v: &Vehicle, color: u32) -> Vehicle3View {
    Vehicle3View {
        position: point(v.position),
        forward: point(v.forward),
        up: point(v.up),
        radius: v.radius,
        color,
    }
}

/// Three short axis-aligned strokes through `at`, a marker readable from any angle.
pub fn cross(at: Vec3, size: f32, color: u32) -> [Line3View; 3] {
    let axes = [
        Vec3::new(size, 0.0, 0.0),
        Vec3::new(0.0, size, 0.0),
        Vec3::new(0.0, 0.0, size),
    ];
    axes.map(|a| line(at - a, at + a, color))
}

/// A vertical line from `at` down (or up) to the ground at y = 0, so heights read in 3D.
pub fn stem(at: Vec3) -> Line3View {
    line(at, at.set_y_to_zero(), color::EDGE)
}

/// `annotationVelocityAcceleration(3, 3)`: velocity and smoothed acceleration, each scaled so
/// its maximum is 3 units long.
pub fn velocity_acceleration(v: &Vehicle) -> [Line3View; 2] {
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

/// OpenSteer's default trail: 5 seconds of history in 100 samples.
const TRAIL_DURATION: f32 = 5.0;
const TRAIL_VERTEX_COUNT: usize = 100;

/// A vehicle's recent positions, sampled at a fixed interval (`recordTrailVertex`).
#[derive(Debug, Clone, Default)]
pub struct Trail {
    points: VecDeque<Vec3>,
    last_sample_time: f32,
}

impl Trail {
    pub fn record(&mut self, time: f32, position: Vec3) {
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
    pub fn view(&self, current: Vec3) -> Trail3View {
        Trail3View {
            points: self
                .points
                .iter()
                .chain([&current])
                .map(|&p| point(p))
                .collect(),
            color: color::TRAIL,
        }
    }
}
