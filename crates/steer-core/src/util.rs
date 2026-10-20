//! Scalar and vector helpers from OpenSteer's `Utilities.h` and `Vec3.cpp`.

use std::ops::{Add, Mul, Sub};

use crate::Vec2;

/// Linear interpolation: `x0` at `alpha == 0`, `x1` at `alpha == 1`.
pub fn interpolate<T>(alpha: f32, x0: T, x1: T) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>,
{
    x0 + (x1 - x0) * alpha
}

/// Moves `accumulator` toward `value` by `smooth_rate` (clamped to 0..=1): an exponential
/// moving average.
pub fn blend_into_accumulator<T>(smooth_rate: f32, value: T, accumulator: &mut T)
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>,
{
    *accumulator = interpolate(smooth_rate.clamp(0.0, 1.0), *accumulator, value);
}

/// `source` bent, if needed, to lie inside the cone around unit vector `basis` whose
/// half-angle has cosine `cosine_of_cone_angle`; its length is kept.
pub fn limit_max_deviation_angle(source: Vec2, cosine_of_cone_angle: f32, basis: Vec2) -> Vec2 {
    let source_length = source.length();
    if source_length == 0.0 {
        return source;
    }
    let direction = source / source_length;
    if direction.dot(basis) >= cosine_of_cone_angle {
        return source;
    }
    let unit_perp = source.perpendicular_component(basis).normalize();
    let perp_dist = (1.0 - cosine_of_cone_angle * cosine_of_cone_angle).sqrt();
    (basis * cosine_of_cone_angle + unit_perp * perp_dist) * source_length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_and_blend() {
        assert_eq!(interpolate(0.25, 0.0, 8.0), 2.0);
        let mut acc = Vec2::ZERO;
        blend_into_accumulator(0.5, Vec2::new(2.0, 4.0), &mut acc);
        assert_eq!(acc, Vec2::new(1.0, 2.0));
        blend_into_accumulator(7.0, Vec2::new(9.0, 9.0), &mut acc);
        assert_eq!(acc, Vec2::new(9.0, 9.0));
    }

    #[test]
    fn deviation_inside_cone_is_unchanged() {
        let forward = Vec2::new(0.0, 1.0);
        let v = Vec2::new(0.1, 2.0);
        assert_eq!(limit_max_deviation_angle(v, 0.5, forward), v);
    }

    #[test]
    fn deviation_outside_cone_is_bent_onto_its_edge() {
        let forward = Vec2::new(0.0, 1.0);
        let sideways = Vec2::new(3.0, 0.0);
        let bent = limit_max_deviation_angle(sideways, 0.5, forward);
        assert!((bent.length() - 3.0).abs() < 1e-6);
        assert!((bent.normalize().dot(forward) - 0.5).abs() < 1e-6);
        assert!(bent.x > 0.0);
    }
}
