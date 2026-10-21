//! Shared by the C++ oracle tests: a vehicle's full state compared with a printed checkpoint.

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
