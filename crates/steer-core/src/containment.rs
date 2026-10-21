//! Containment (Reynolds 1999, not in OpenSteer): keep a vehicle inside a region, the
//! "generalized obstacle avoidance" of the paper. Like `steerToAvoidObstacles`, a probe ahead
//! and a full `max_force` sideways push; two more probes, 45 degrees to each side, pick the side.

use crate::wall::Wall;
use crate::{Vec2, Vehicle};

/// The probes `radius + min_time * speed` from the center: ahead, and 45 degrees toward
/// `+side` and `-side`.
pub fn probes(v: &Vehicle, min_time: f32) -> [Vec2; 3] {
    let reach = v.radius + min_time * v.speed;
    let (f, s) = (v.forward, v.side());
    [f, (f + s).normalize(), (f - s).normalize()].map(|d| v.position + d * reach)
}

/// While the ahead probe is outside `region`: a `max_force` push along `side`, toward the side
/// probe with more room (`room`); `+side` on an exact tie. Zero while the ahead probe is
/// inside.
///
/// Only sideways: a braking push slows the vehicle into the low-speed rule, which bends the
/// push forward again. The side comes from the side probes, not the ahead probe: heading into
/// a corner or head on, any side computed at the ahead probe (an edge normal, the nearest
/// point) flips with tiny heading changes, and the alternating pushes cancel; turning toward
/// the roomier side probe makes it roomier still, so the choice holds.
pub fn contain(v: &Vehicle, region: &Wall, min_time: f32) -> Vec2 {
    let [ahead, plus, minus] = probes(v, min_time);
    if region.contains(ahead) {
        return Vec2::ZERO;
    }
    let turn = if room(region, plus) >= room(region, minus) {
        1.0
    } else {
        -1.0
    };
    v.side() * (turn * v.max_force)
}

/// Distance from `point` to the wall, negative outside the region.
pub fn room(region: &Wall, point: Vec2) -> f32 {
    let d = region.distance(point);
    if region.contains(point) {
        d
    } else {
        -d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10 x 10 room from the origin.
    fn room() -> Wall {
        Wall::new(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ])
    }

    fn at(x: f32, y: f32, forward: Vec2) -> Vehicle {
        Vehicle {
            position: Vec2::new(x, y),
            forward,
            speed: 2.0,
            max_force: 3.0,
            ..Vehicle::default()
        }
    }

    #[test]
    fn probes_reach_ahead_and_to_both_sides() {
        let v = at(5.0, 5.0, Vec2::new(1.0, 0.0));
        // radius 0.5 + 1 s * speed 2; side is (0, 1)
        let [ahead, plus, minus] = probes(&v, 1.0);
        assert_eq!(ahead, Vec2::new(7.5, 5.0));
        let r = 2.5 * std::f32::consts::FRAC_1_SQRT_2;
        assert!((plus - Vec2::new(5.0 + r, 5.0 + r)).length() < 1e-5);
        assert!((minus - Vec2::new(5.0 + r, 5.0 - r)).length() < 1e-5);
    }

    #[test]
    fn no_push_while_the_probe_is_inside() {
        let v = at(5.0, 5.0, Vec2::new(1.0, 0.0));
        assert_eq!(contain(&v, &room(), 1.0), Vec2::ZERO);
    }

    /// Heading to the east wall at 45 degrees: the ahead probe (10.77, 6.77) is outside past
    /// x = 10; the +side probe (9, 7.5) is inside, the -side probe (11.5, 5) is not: turn to
    /// +side (-0.71, 0.71).
    #[test]
    fn a_probe_outside_turns_the_vehicle_back_in() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let v = at(9.0, 5.0, Vec2::new(d, d));
        let push = contain(&v, &room(), 1.0);
        assert!((push - Vec2::new(-d, d) * 3.0).length() < 1e-5, "{push:?}");
    }

    /// Into the corner (10, 0), nearly along the bottom edge: all three probes are outside;
    /// the +side probe (11.35, 1.3) is 1.35 out, the -side one (11.18, -2.23) 2.52: +side (up).
    #[test]
    fn past_a_corner_the_turn_points_inside() {
        let forward = Vec2::new(1.0, -0.05).normalize();
        let v = at(12.0 - 2.5 * forward.x, -0.5 - 2.5 * forward.y, forward);
        let push = contain(&v, &room(), 1.0);
        assert!(push.y > 2.9, "{push:?}");
    }

    /// Head on at the east wall, mid-edge, with both side probes outside: +side.
    #[test]
    fn head_on_turns_to_plus_side() {
        let v = at(9.0, 5.0, Vec2::new(1.0, 0.0));
        assert_eq!(contain(&v, &room(), 1.0), Vec2::new(0.0, 3.0));
    }

    /// North along the west wall: side is (-1, 0). The ahead probe (1, 10.5) is out past the
    /// north wall, the +side probe (-0.77, 9.77) past the west wall; the -side probe
    /// (2.77, 9.77) is inside: turn to -side (+x).
    #[test]
    fn near_a_wall_turns_away_from_it() {
        let v = at(1.0, 8.0, Vec2::new(0.0, 1.0));
        assert_eq!(contain(&v, &room(), 1.0), Vec2::new(3.0, 0.0));
    }
}
