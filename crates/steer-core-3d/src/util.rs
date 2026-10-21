//! Scalar and vector helpers from OpenSteer's `Utilities.h` and `Vec3.cpp`, as in `steer-core`.

use std::ops::{Add, Mul, Sub};

use crate::Vec3;

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
pub fn limit_max_deviation_angle(source: Vec3, cosine_of_cone_angle: f32, basis: Vec3) -> Vec3 {
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

/// `initial` moved by a random step in `-walk_speed..walk_speed`, clipped to `min..=max`
/// (`scalarRandomWalk`). `random01` returns a number in `0..=1`.
pub fn scalar_random_walk(
    initial: f32,
    walk_speed: f32,
    min: f32,
    max: f32,
    random01: &mut impl FnMut() -> f32,
) -> f32 {
    (initial + (random01() * 2.0 - 1.0) * walk_speed).clamp(min, max)
}

/// A random point inside the unit sphere (`RandomVectorInUnitRadiusSphere`): rejection sampling
/// in the cube, three draws (x, y, z) per try.
pub fn random_vector_in_unit_sphere(random01: &mut impl FnMut() -> f32) -> Vec3 {
    loop {
        let x = random01() * 2.0 - 1.0;
        let y = random01() * 2.0 - 1.0;
        let z = random01() * 2.0 - 1.0;
        let v = Vec3::new(x, y, z);
        if v.length() < 1.0 {
            return v;
        }
    }
}

/// A random unit vector (`RandomUnitVector`): `random_vector_in_unit_sphere`, normalized.
pub fn random_unit_vector(random01: &mut impl FnMut() -> f32) -> Vec3 {
    random_vector_in_unit_sphere(random01).normalize()
}
