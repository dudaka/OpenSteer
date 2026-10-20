//! `seek` and `flee` against the C++ `xxxsteerForSeek` / `xxxsteerForFlee`. Expected values
//! are printed by `oracle/seek_flee.cpp` (C++ `x, z` -> `x, y`).

use steer_core::{steer, Vec2, Vehicle};

/// step, position x, y, forward x, y, speed
type Checkpoint = (u32, f64, f64, f64, f64, f64);

const SEEK: [Checkpoint; 10] = [
    (30, 0.000015, 0.090796, 0.000929, 1.000000, 0.406274),
    (60, 0.043698, 0.365266, 0.423702, 0.905802, 0.573344),
    (90, 0.266506, 0.555226, 0.945162, 0.326603, 0.648724),
    (120, 0.655461, 0.561813, 0.982667, -0.185382, 0.945487),
    (150, 1.197703, 0.371005, 0.904916, -0.425589, 1.355637),
    (180, 1.883402, -0.024459, 0.834141, -0.551552, 1.801631),
    (210, 2.699815, -0.632140, 0.775849, -0.630919, 2.250025),
    (240, 3.614540, -1.430163, 0.736041, -0.676937, 2.567893),
    (270, 4.572762, -2.351717, 0.708082, -0.706131, 2.686534),
    (300, 5.453978, -3.257071, 0.688180, -0.725540, 2.317132),
];

const FLEE: [Checkpoint; 10] = [
    (30, -0.000015, 0.090796, -0.000929, 1.000000, 0.406274),
    (60, -0.041564, 0.364308, -0.407602, 0.913160, 0.574824),
    (90, -0.255554, 0.555990, -0.941359, 0.337407, 0.624373),
    (120, -0.635446, 0.584044, -0.997757, -0.066933, 0.925073),
    (150, -1.191938, 0.508773, -0.985558, -0.169338, 1.322076),
    (180, -1.961093, 0.362569, -0.980604, -0.195999, 1.803370),
    (210, -2.968678, 0.156356, -0.979141, -0.203184, 2.279100),
    (240, -4.169687, -0.094645, -0.978674, -0.205422, 2.589859),
    (270, -5.485731, -0.371530, -0.978512, -0.206191, 2.767060),
    (300, -6.867227, -0.662902, -0.978448, -0.206494, 2.867707),
];

/// From rest at the origin facing +y; max_force 1, max_speed 3; checkpoints every 30 steps.
fn check(behavior: fn(&Vehicle, Vec2) -> Vec2, point: Vec2, expected: &[Checkpoint]) {
    let mut v = Vehicle {
        max_force: 1.0,
        max_speed: 3.0,
        ..Vehicle::default()
    };
    let mut checkpoints = expected.iter();
    for step in 1..=300 {
        v.apply_steering_force(behavior(&v, point), 1.0 / 60.0);
        if step % 30 != 0 {
            continue;
        }
        let &(s, px, py, fx, fy, speed) = checkpoints.next().expect("a checkpoint per 30 steps");
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
fn seek_matches_cpp() {
    check(steer::seek, Vec2::new(6.0, -4.0), &SEEK);
}

#[test]
fn flee_matches_cpp() {
    check(steer::flee, Vec2::new(1.0, 1.0), &FLEE);
}
