//! Shared by the C++ oracle tests: a vehicle's full state compared with a printed checkpoint.

// Each test file compiles this module on its own and uses only part of it.
#![allow(dead_code)]

use steer_core_3d::Vehicle;

/// Position, forward, side, up (x, y, z each) and speed: the order the oracles print.
pub type State = [f64; 13];

pub fn state(v: &Vehicle) -> [f32; 13] {
    let (p, f, s, u) = (v.position, v.forward, v.side, v.up);
    [
        p.x, p.y, p.z, f.x, f.y, f.z, s.x, s.y, s.z, u.x, u.y, u.z, v.speed,
    ]
}

/// Every value within 1e-4 of the C++.
pub fn check(v: &Vehicle, step: u32, want: &State) {
    let got = state(v);
    for (g, w) in got.iter().zip(want) {
        assert!(
            (f64::from(*g) - w).abs() < 1e-4,
            "step {step}: got {got:?}, want {want:?}"
        );
    }
}

/// The oracles' `rand()` (`-Drand=oracle_rand`): the LCG `seed * 1103515245 + 12345`, as
/// `frandom01() = rand() / RAND_MAX`.
pub struct OracleRand(pub u32);

impl OracleRand {
    pub fn next01(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 & 0x7fff_ffff) as f32 / i32::MAX as f32
    }
}
