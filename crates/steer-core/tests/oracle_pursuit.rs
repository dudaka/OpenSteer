//! `pursuit` and `evasion` against the C++ `steerForPursuit` and (repaired) `steerForEvasion`.
//! Expected values are printed by `oracle/pursuit.cpp` (C++ `x, z` -> `x, y`).

use steer_core::{steer, Vec2, Vehicle};

/// step, vehicle position x, y, forward x, y, speed, other vehicle position x, y
type Checkpoint = (u32, f64, f64, f64, f64, f64, f64, f64);

const PURSUIT: [Checkpoint; 10] = [
    (
        60, 7.307699, -4.322567, -0.515262, 0.857033, 3.000000, -0.284285, 0.588039,
    ),
    (
        120, 5.076945, -2.386591, -0.904015, 0.427500, 3.000000, -2.183978, 1.102868,
    ),
    (
        180, 2.264835, -1.346811, -0.916587, 0.399835, 3.000000, -4.823633, 2.121771,
    ),
    (
        240, -0.149349, 0.396649, -0.675619, 0.737251, 3.000000, -6.451249, 4.610244,
    ),
    (
        300, -1.718496, 2.939769, -0.436811, 0.899553, 3.000000, -7.238973, 7.500685,
    ),
    (
        360, -3.358300, 5.432710, -0.711186, 0.703004, 3.000000, -9.352070, 9.518489,
    ),
    (
        420, -5.996134, 6.747626, -0.990538, 0.137241, 3.000000, -12.266111, 9.314154,
    ),
    (
        480, -8.914927, 6.270599, -0.900791, -0.434252, 3.000000, -14.492463, 7.381941,
    ),
    (
        540, -11.274141, 4.436157, -0.715950, -0.698151, 3.000000, -15.390531, 4.534078,
    ),
    (
        600, -13.208429, 2.148731, -0.569743, -0.821823, 3.000000, -16.206900, 1.650484,
    ),
];

const EVASION: [Checkpoint; 10] = [
    (
        60, 0.969618, 0.898971, 0.921237, 0.389002, 1.969347, -4.259553, -3.350896,
    ),
    (
        120, 2.386702, 1.282105, 0.931410, 0.363972, 0.852539, -1.849325, -1.612340,
    ),
    (
        180, 2.752617, 1.649957, 0.536466, 0.843922, 0.375854, 0.666331, 0.001857,
    ),
    (
        240, 3.077767, 1.831221, 0.999573, 0.029205, 0.388154, 2.827554, 1.846181,
    ),
    (
        300, 3.239653, 1.536963, -0.515177, -0.857084, 0.708106, 3.966982, 2.501512,
    ),
    (
        360, 2.529357, 1.178326, -0.993478, -0.114021, 0.641410, 3.432898, 1.567003,
    ),
    (
        420, 2.103663, 1.248459, -0.668546, 0.743670, 0.389261, 1.861726, 0.760616,
    ),
    (
        480, 2.374366, 1.661504, 0.934162, 0.356849, 0.782764, 1.367542, 0.979910,
    ),
    (
        540, 2.967488, 1.696196, 0.980691, -0.195564, 0.384753, 2.614490, 1.759282,
    ),
    (
        600, 3.187800, 1.434777, -0.294719, -0.955584, 0.430641, 3.677414, 1.980673,
    ),
];

const DT: f32 = 1.0 / 60.0;

/// The oracle's `rand()` as `frandom01` sees it, two draws per wander call (side, unused up).
struct OracleRand(u32);

impl OracleRand {
    fn next01(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 & 0x7fff_ffff) as f32 / i32::MAX as f32
    }

    fn wander_draw(&mut self) -> f32 {
        let side = self.next01();
        self.next01();
        side
    }
}

/// `MpBase::reset`: at rest facing +y, max_force 5, max_speed 3.
fn mp_vehicle(x: f32, y: f32) -> Vehicle {
    Vehicle {
        position: Vec2::new(x, y),
        max_force: 5.0,
        max_speed: 3.0,
        ..Vehicle::default()
    }
}

fn assert_checkpoint(step: u32, v: &Vehicle, other: &Vehicle, expected: &Checkpoint) {
    let &(s, px, py, fx, fy, speed, ox, oy) = expected;
    assert_eq!(s, step);
    let got = [
        v.position.x,
        v.position.y,
        v.forward.x,
        v.forward.y,
        v.speed,
        other.position.x,
        other.position.y,
    ];
    let want = [px, py, fx, fy, speed, ox, oy];
    for (g, w) in got.iter().zip(want) {
        assert!(
            (f64::from(*g) - w).abs() < 1e-4,
            "step {s}: got {got:?}, want {want:?}"
        );
    }
}

#[test]
fn pursuit_of_a_wanderer_matches_cpp() {
    let mut rand = OracleRand(1);
    let mut wanderer = mp_vehicle(0.0, 0.0);
    let mut pursuer = mp_vehicle(8.0, -6.0);
    let mut wander_side = 0.0;
    let mut checkpoints = PURSUIT.iter();
    for step in 1..=600 {
        let wander = steer::wander(&wanderer, &mut wander_side, DT, &mut || rand.wander_draw());
        wanderer.apply_steering_force(wanderer.forward + wander * 3.0, DT);
        pursuer.apply_steering_force(steer::pursuit(&pursuer, &wanderer, 20.0), DT);
        if step % 60 == 0 {
            assert_checkpoint(step, &pursuer, &wanderer, checkpoints.next().unwrap());
        }
    }
}

#[test]
fn evasion_of_a_pursuer_matches_cpp() {
    let mut evader = Vehicle {
        max_speed: 2.0,
        ..mp_vehicle(0.0, 0.0)
    };
    let mut hunter = mp_vehicle(-5.0, -5.0);
    let mut checkpoints = EVASION.iter();
    for step in 1..=600 {
        evader.apply_steering_force(steer::evasion(&evader, &hunter, 2.0), DT);
        hunter.apply_steering_force(steer::pursuit(&hunter, &evader, 20.0), DT);
        if step % 60 == 0 {
            assert_checkpoint(step, &evader, &hunter, checkpoints.next().unwrap());
        }
    }
}
