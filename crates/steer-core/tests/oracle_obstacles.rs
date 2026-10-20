//! `avoid_obstacles` against the C++ `steerToAvoidObstacles` with `SphereObstacle`s. Expected
//! values are printed by `oracle/obstacles.cpp` (C++ `x, z` -> `x, y`).

use steer_core::obstacle::{avoid_obstacles, Circle, SeenFrom};
use steer_core::{steer, Vec2, Vehicle};

/// step, position x, y, forward x, y, speed, steps spent avoiding so far
type Checkpoint = (u32, f64, f64, f64, f64, f64, u32);

const WANDER_AMONG_OBSTACLES: [Checkpoint; 20] = [
    (90, -1.067503, 0.918198, -0.975871, 0.218347, 1.995492, 0),
    (180, -4.823633, 2.121771, -0.750266, 0.661136, 3.000000, 0),
    (270, -6.806125, 6.067233, -0.219266, 0.975665, 3.000000, 0),
    (360, -9.369728, 9.473498, -0.975871, 0.218349, 3.000000, 16),
    (
        450, -12.276021, 6.687541, -0.121264, -0.992620, 3.000000, 42,
    ),
    (540, -10.933440, 2.566097, 0.721133, -0.692796, 3.000000, 42),
    (630, -7.027716, 0.512149, 0.956349, -0.292227, 3.000000, 59),
    (720, -2.715987, 0.912468, 0.889444, 0.457045, 3.000000, 59),
    (810, 1.609931, 1.199695, 0.923482, -0.383642, 3.000000, 59),
    (900, 4.993217, -1.615965, 0.963556, -0.267507, 3.000000, 90),
    (990, 9.325569, -2.806825, 0.985352, -0.170535, 3.000000, 98),
    (
        1080, 13.687175, -2.245458, 0.748639, 0.662978, 3.000000, 127,
    ),
    (
        1170, 14.786679, 2.010740, -0.035844, 0.999357, 3.000000, 172,
    ),
    (
        1260, 13.565225, 6.299200, -0.554030, 0.832496, 3.000000, 213,
    ),
    (
        1350, 10.501571, 9.552299, -0.799439, 0.600748, 3.000000, 213,
    ),
    (
        1440, 6.339140, 10.476770, -0.924033, -0.382312, 3.000000, 213,
    ),
    (
        1530, 3.836090, 6.942266, -0.262430, -0.964951, 3.000000, 213,
    ),
    (1620, 3.333218, 2.497864, 0.156729, -0.987642, 3.000000, 213),
    (
        1710, 3.978653, -1.906927, -0.201585, -0.979471, 3.000000, 222,
    ),
    (
        1800, 2.325214, -6.071004, -0.399784, -0.916609, 3.000000, 222,
    ),
];

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

#[test]
fn wander_among_obstacles_matches_cpp() {
    let obstacles = [
        Circle::new(Vec2::ZERO, 15.0, SeenFrom::Inside),
        Circle::new(Vec2::new(0.0, 6.0), 2.0, SeenFrom::Outside),
        Circle::new(Vec2::new(-6.0, -2.0), 1.5, SeenFrom::Outside),
        Circle::new(Vec2::new(7.0, -6.0), 3.0, SeenFrom::Outside),
    ];
    let mut v = Vehicle {
        max_force: 5.0,
        max_speed: 3.0,
        ..Vehicle::default()
    };
    let mut rand = OracleRand(1);
    let mut wander_side = 0.0;
    let mut avoiding = 0;
    let mut checkpoints = WANDER_AMONG_OBSTACLES.iter();
    let dt = 1.0 / 60.0;
    for step in 1..=1800 {
        let mut force = avoid_obstacles(&v, 1.0, &obstacles);
        if force == Vec2::ZERO {
            let wander = steer::wander(&v, &mut wander_side, dt, &mut || rand.wander_draw());
            force = v.forward + wander * 3.0;
        } else {
            avoiding += 1;
        }
        v.apply_steering_force(force, dt);
        if step % 90 != 0 {
            continue;
        }
        let &(s, px, py, fx, fy, speed, avoided) = checkpoints.next().unwrap();
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
        assert_eq!(avoiding, avoided, "step {s}");
    }
}
