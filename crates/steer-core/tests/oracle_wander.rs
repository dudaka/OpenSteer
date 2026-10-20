//! `wander` and `target_speed` against the C++ `steerForWander` / `steerForTargetSpeed`.
//! Expected values are printed by `oracle/wander.cpp` (C++ `x, z` -> `x, y`), whose `rand()`
//! is the LCG replayed here.

use steer_core::{steer, Vec2, Vehicle};

/// step, position x, y, forward x, y, speed
type Checkpoint = (u32, f64, f64, f64, f64, f64);

const FORWARD: [Checkpoint; 10] = [
    (60, -0.284285, 0.588039, -0.769689, 0.638419, 1.395080),
    (120, -2.183978, 1.102868, -0.992206, 0.124605, 2.509341),
    (180, -4.823633, 2.121771, -0.750266, 0.661136, 3.000000),
    (240, -6.451249, 4.610244, -0.310207, 0.950669, 3.000000),
    (300, -7.238973, 7.500685, -0.406988, 0.913433, 3.000000),
    (360, -9.352070, 9.518489, -0.937799, 0.347179, 3.000000),
    (420, -12.266111, 9.314154, -0.920094, -0.391699, 3.000000),
    (480, -14.492463, 7.381941, -0.515197, -0.857072, 3.000000),
    (540, -15.390531, 4.534078, -0.220474, -0.975393, 3.000000),
    (600, -16.206900, 1.650484, -0.139532, -0.990218, 3.000000),
];

const TARGET_SPEED: [Checkpoint; 10] = [
    (60, -0.319598, 0.560206, -0.859329, 0.511423, 1.229453),
    (120, -1.712274, 0.568950, -0.976090, -0.217365, 1.490722),
    (180, -3.115138, 0.924444, -0.602687, 0.797977, 1.682948),
    (240, -3.369269, 2.549603, 0.328117, 0.944637, 1.690915),
    (300, -2.687915, 4.045839, 0.133774, 0.991012, 1.620200),
    (360, -3.564193, 5.336630, -0.967414, 0.253199, 1.783297),
    (420, -5.069427, 4.560371, -0.507179, -0.861841, 1.824865),
    (480, -5.045497, 2.878920, 0.543321, -0.839525, 1.758695),
    (540, -3.617851, 1.952302, 0.912661, -0.408718, 1.660162),
    (600, -2.230017, 1.152491, 0.966587, -0.256339, 1.594780),
];

/// The oracle's `rand()`, as C's `frandom01` sees it: `rand() / RAND_MAX`.
struct OracleRand(u32);

impl OracleRand {
    fn next01(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 & 0x7fff_ffff) as f32 / i32::MAX as f32
    }
}

/// MultiplePursuit's wanderer: from rest at the origin facing +y, max_force 5, max_speed 3,
/// force `push + 3 * wander`; checkpoints every 60 steps.
fn check(push: fn(&Vehicle) -> Vec2, expected: &[Checkpoint]) {
    let mut v = Vehicle {
        max_force: 5.0,
        max_speed: 3.0,
        ..Vehicle::default()
    };
    let mut rand = OracleRand(1);
    // The C++ walks side, then up (unused in 2D): keep the first draw of each pair.
    let mut random01 = || {
        let side = rand.next01();
        rand.next01();
        side
    };
    let mut wander_side = 0.0;
    let mut checkpoints = expected.iter();
    for step in 1..=600 {
        let dt = 1.0 / 60.0;
        let wander = steer::wander(&v, &mut wander_side, dt, &mut random01);
        v.apply_steering_force(push(&v) + wander * 3.0, dt);
        if step % 60 != 0 {
            continue;
        }
        let &(s, px, py, fx, fy, speed) = checkpoints.next().expect("a checkpoint per 60 steps");
        let got = [
            v.position.x,
            v.position.y,
            v.forward.x,
            v.forward.y,
            v.speed,
        ];
        let want = [px, py, fx, fy, speed];
        for (g, w) in got.iter().zip(want) {
            assert!(
                (f64::from(*g) - w).abs() < 1e-4,
                "step {s}: got {got:?}, want {want:?}"
            );
        }
    }
}

#[test]
fn wander_with_forward_push_matches_cpp() {
    check(|v| v.forward, &FORWARD);
}

#[test]
fn wander_with_target_speed_matches_cpp() {
    check(|v| steer::target_speed(v, 1.5), &TARGET_SPEED);
}
