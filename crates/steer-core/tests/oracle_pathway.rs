//! `Pathway` against the C++ `PolylineSegmentedPathwaySingleRadius` mappings. Expected values
//! are printed by `oracle/pathway.cpp` (C++ `x, z` -> `x, y`).

use steer_core::pathway::Pathway;
use steer_core::Vec2;

/// query point, point on path, tangent, outside, distance along the path
type PointCase = ([f64; 2], [f64; 2], [f64; 2], f64, f64);
/// distance along the path, point
type DistanceCase = (f64, [f64; 2]);

const OPEN_POINTS: [PointCase; 12] = [
    ([5.0, 1.0], [5.0, 0.0], [1.0, 0.0], -0.5, 5.0),
    ([5.0, -3.0], [5.0, 0.0], [1.0, 0.0], 1.5, 5.0),
    ([11.0, -1.0], [10.0, 0.0], [1.0, 0.0], -0.085786, 10.0),
    ([12.0, 3.0], [10.0, 3.0], [0.0, 1.0], 0.5, 13.0),
    ([9.0, 1.0], [9.0, 0.0], [1.0, 0.0], -0.5, 9.0),
    ([-2.0, -2.0], [0.0, 0.0], [1.0, 0.0], 1.328427, 0.0),
    (
        [7.0, 9.0],
        [6.538461, 8.307693],
        [-0.83205, 0.5547],
        -0.66795,
        20.160252,
    ),
    ([2.0, 5.0], [2.0, 0.0], [1.0, 0.0], 3.5, 2.0),
    (
        [4.0, 10.0],
        [4.0, 10.0],
        [-0.83205, 0.5547],
        -1.5,
        23.211102,
    ),
    (
        [3.0, 11.0],
        [4.0, 10.0],
        [-0.83205, 0.5547],
        -0.085786,
        23.211102,
    ),
    ([10.0, 6.0], [10.0, 6.0], [0.0, 1.0], -1.5, 16.0),
    ([6.0, 3.0], [6.0, 0.0], [1.0, 0.0], 1.5, 6.0),
];

const CYCLIC_POINTS: [PointCase; 12] = [
    ([5.0, 1.0], [5.0, 0.0], [1.0, 0.0], -0.5, 5.0),
    ([5.0, -3.0], [5.0, 0.0], [1.0, 0.0], 1.5, 5.0),
    ([11.0, -1.0], [10.0, 0.0], [1.0, 0.0], -0.085786, 10.0),
    ([12.0, 3.0], [10.0, 3.0], [0.0, 1.0], 0.5, 13.0),
    ([9.0, 1.0], [9.0, 0.0], [1.0, 0.0], -0.5, 9.0),
    ([-2.0, -2.0], [0.0, 0.0], [1.0, 0.0], 1.328427, 0.0),
    (
        [7.0, 9.0],
        [6.538461, 8.307693],
        [-0.83205, 0.5547],
        -0.66795,
        20.160252,
    ),
    (
        [2.0, 5.0],
        [2.0, 5.0],
        [-0.371391, -0.928477],
        -1.5,
        28.596266,
    ),
    (
        [4.0, 10.0],
        [4.0, 10.0],
        [-0.83205, 0.5547],
        -1.5,
        23.211102,
    ),
    (
        [3.0, 11.0],
        [4.0, 10.0],
        [-0.83205, 0.5547],
        -0.085786,
        23.211102,
    ),
    ([10.0, 6.0], [10.0, 6.0], [0.0, 1.0], -1.5, 16.0),
    ([6.0, 3.0], [6.0, 0.0], [1.0, 0.0], 1.5, 6.0),
];

const OPEN_DISTANCES: [DistanceCase; 9] = [
    (-3.0, [0.0, 0.0]),
    (0.0, [0.0, 0.0]),
    (4.0, [4.0, 0.0]),
    (10.0, [10.0, 0.0]),
    (13.0, [10.0, 3.0]),
    (20.0, [6.671799, 8.218801]),
    (23.2111, [4.000002, 9.999998]),
    (25.0, [4.0, 10.0]),
    (40.0, [4.0, 10.0]),
];

const CYCLIC_DISTANCES: [DistanceCase; 9] = [
    (-3.0, [0.0, 0.0]),
    (0.0, [0.0, 0.0]),
    (4.0, [4.0, 0.0]),
    (10.0, [10.0, 0.0]),
    (13.0, [10.0, 3.0]),
    (20.0, [6.671799, 8.218801]),
    (23.2111, [4.000002, 9.999998]),
    (25.0, [3.33562, 8.33905]),
    (40.0, [6.01857, 0.0]),
];

fn path(cyclic: bool) -> Pathway {
    let points = [
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(10.0, 6.0),
        Vec2::new(4.0, 10.0),
    ];
    Pathway::new(&points, 1.5, cyclic)
}

fn close(got: f32, want: f64) -> bool {
    (f64::from(got) - want).abs() < 1e-4
}

fn check(cyclic: bool, length: f64, points: &[PointCase], distances: &[DistanceCase]) {
    let path = path(cyclic);
    assert!(close(path.length(), length));
    for &(q, on, tangent, outside, distance) in points {
        let m = path.map_point(Vec2::new(q[0] as f32, q[1] as f32));
        let got = [
            m.on_path.x,
            m.on_path.y,
            m.tangent.x,
            m.tangent.y,
            m.outside,
            m.distance,
        ];
        let want = [on[0], on[1], tangent[0], tangent[1], outside, distance];
        assert!(
            got.iter().zip(want).all(|(g, w)| close(*g, w)),
            "point {q:?}: got {got:?}, want {want:?}"
        );
    }
    for &(d, p) in distances {
        let got = path.point_at(d as f32);
        assert!(
            close(got.x, p[0]) && close(got.y, p[1]),
            "distance {d}: got {got:?}, want {p:?}"
        );
    }
}

#[test]
fn open_path_matches_cpp() {
    check(false, 23.211102, &OPEN_POINTS, &OPEN_DISTANCES);
}

#[test]
fn cyclic_path_matches_cpp() {
    check(true, 33.981430, &CYCLIC_POINTS, &CYCLIC_DISTANCES);
}
