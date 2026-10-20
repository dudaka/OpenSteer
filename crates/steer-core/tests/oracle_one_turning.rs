//! The integrator against the C++ `SimpleVehicle`, OneTurning scenario. Expected values are
//! printed by `oracle/one_turning.cpp` (C++ `x, z` -> `x, y`).

use steer_core::{Vec2, Vehicle};

/// step, position x, y, forward x, y, speed
const CPP: [(u32, f64, f64, f64, f64, f64); 10] = [
    (60, -0.070360, 1.394461, -0.117469, 0.993076, 1.282840),
    (120, -0.305645, 2.541533, -0.295722, 0.955274, 1.072303),
    (180, -0.714689, 3.446780, -0.538073, 0.842898, 0.965725),
    (240, -1.354106, 4.167627, -0.769434, 0.638726, 0.967438),
    (300, -2.187167, 4.661660, -0.926528, 0.376227, 0.988087),
    (360, -3.187250, 4.906518, -0.993689, 0.112172, 1.088772),
    (420, -4.353743, 4.901760, -0.994826, -0.101598, 1.254805),
    (480, -5.686648, 4.647386, -0.966262, -0.257561, 1.464120),
    (540, -7.185964, 4.143399, -0.929636, -0.368480, 1.700812),
    (600, -8.851691, 3.389796, -0.893904, -0.448259, 1.954960),
];

#[test]
fn one_turning_matches_cpp() {
    let mut v = Vehicle {
        speed: 1.5,
        max_force: 0.3,
        max_speed: 5.0,
        ..Vehicle::default()
    };
    let mut checkpoints = CPP.iter();
    for step in 1..=600 {
        v.apply_steering_force(Vec2::new(-2.0, -3.0), 1.0 / 60.0);
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
