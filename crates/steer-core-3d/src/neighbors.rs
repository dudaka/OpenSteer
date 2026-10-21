//! Unaligned collision avoidance from OpenSteer's `SteerLibrary.h`, in 3D: steer around other vehicles
//! moving in any direction (`steerToAvoidNeighbors`). `others` is every other vehicle (not `v`
//! itself), in a fixed order.

use crate::{Vec3, Vehicle};

/// The neighbor `avoid_neighbors` reacts to, for annotation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threat {
    /// Index of the threat in `others`.
    pub index: usize,
    /// Our position and the threat's at their nearest approach.
    pub ours: Vec3,
    pub theirs: Vec3,
    /// -1 or +1 along `side`, 0 when a perpendicular threat is faster (it gives way instead).
    pub steer: f32,
}

/// Seconds until `v` and `other` are closest if both keep their velocities
/// (`predictNearestApproachTime`); negative if that was in the past, 0 at equal velocities.
pub fn nearest_approach_time(v: &Vehicle, other: &Vehicle) -> f32 {
    let rel_velocity = other.velocity() - v.velocity();
    let rel_speed = rel_velocity.length();
    if rel_speed == 0.0 {
        return 0.0;
    }
    let rel_tangent = rel_velocity / rel_speed;
    rel_tangent.dot(v.position - other.position) / rel_speed
}

/// Where `v` and `other` will be after `time` seconds at their velocities
/// (`computeNearestApproachPositions`).
pub fn approach_positions(v: &Vehicle, other: &Vehicle, time: f32) -> (Vec3, Vec3) {
    (
        v.position + v.forward * v.speed * time,
        other.position + other.forward * other.speed * time,
    )
}

/// `steerToAvoidCloseNeighbors`: away from the first vehicle closer than
/// `min_separation + both radii`, sideways only; zero if none.
pub fn avoid_close_neighbors<'a>(
    v: &Vehicle,
    min_separation: f32,
    others: impl IntoIterator<Item = &'a Vehicle>,
) -> Vec3 {
    others
        .into_iter()
        .find(|o| (o.position - v.position).length() < min_separation + v.radius + o.radius)
        .map_or(Vec3::ZERO, |o| {
            (v.position - o.position).perpendicular_component(v.forward)
        })
}

/// The first neighbor (in `others` order) predicted to come within two of our radii sooner
/// than `min_time_to_collision`, and the side to steer to (`steerToAvoidNeighbors`' search).
pub fn threat(v: &Vehicle, min_time_to_collision: f32, others: &[&Vehicle]) -> Option<Threat> {
    let danger = v.radius * 2.0;
    let mut min_time = min_time_to_collision;
    let mut found = None;
    for (index, other) in others.iter().enumerate() {
        let time = nearest_approach_time(v, other);
        if time < 0.0 || time >= min_time {
            continue;
        }
        let (ours, theirs) = approach_positions(v, other, time);
        if (ours - theirs).length() < danger {
            min_time = time;
            found = Some((index, ours, theirs));
        }
    }
    let (index, ours, theirs) = found?;
    let other = others[index];
    let away_from = |offset: Vec3| {
        if offset.dot(v.side) > 0.0 {
            -1.0
        } else {
            1.0
        }
    };
    let parallelness = v.forward.dot(other.forward);
    let steer = if parallelness < -0.707 {
        // head on: away from where it will be
        away_from(theirs - v.position)
    } else if parallelness > 0.707 {
        // parallel: away from where it is
        away_from(other.position - v.position)
    } else if other.speed <= v.speed {
        // perpendicular: the faster one (or equal) passes behind the other; the C++ comment
        // says "the slower", but its code tests `threat->speed() <= speed()`
        away_from(other.velocity())
    } else {
        0.0
    };
    Some(Threat {
        index,
        ours,
        theirs,
        steer,
    })
}

/// `steerToAvoidNeighbors`: first `avoid_close_neighbors(0)`; otherwise a unit sideways push
/// from `threat`, or zero.
pub fn avoid_neighbors(v: &Vehicle, min_time_to_collision: f32, others: &[&Vehicle]) -> Vec3 {
    let separation = avoid_close_neighbors(v, 0.0, others.iter().copied());
    if separation != Vec3::ZERO {
        return separation;
    }
    threat(v, min_time_to_collision, others).map_or(Vec3::ZERO, |t| v.side * t.steer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Moving along `forward` at `speed`.
    fn at(x: f32, y: f32, z: f32, forward: Vec3, speed: f32) -> Vehicle {
        let mut v = Vehicle {
            position: Vec3::new(x, y, z),
            speed,
            ..Vehicle::default()
        };
        v.regenerate_basis(forward);
        v
    }

    const AHEAD: Vec3 = Vec3::new(0.0, 0.0, 1.0);
    const BACK: Vec3 = Vec3::new(0.0, 0.0, -1.0);

    #[test]
    fn approach_time_of_two_closing_vehicles() {
        let me = at(0.0, 0.0, 0.0, AHEAD, 1.0);
        // 10 apart closing at 2: closest in 5 s, even with a height offset
        assert_eq!(
            nearest_approach_time(&me, &at(0.0, 3.0, 10.0, BACK, 1.0)),
            5.0
        );
        assert!(nearest_approach_time(&me, &at(0.0, 0.0, -10.0, BACK, 1.0)) < 0.0);
    }

    /// Head on with the other slightly to our side (side is -x when facing +z): steer the other
    /// way, to +x.
    #[test]
    fn head_on_steers_away_from_the_meeting_point() {
        let me = at(0.0, 0.0, 0.0, AHEAD, 1.0);
        let other = at(-0.2, 0.0, 10.0, BACK, 1.0);
        let t = threat(&me, 10.0, &[&other]).unwrap();
        assert_eq!(t.steer, -1.0);
        assert_eq!(
            avoid_neighbors(&me, 10.0, &[&other]),
            Vec3::new(1.0, 0.0, 0.0)
        );
    }

    /// A threat passing straight overhead still counts: within two radii at the closest point.
    #[test]
    fn a_threat_from_above_is_seen() {
        let me = at(0.0, 0.0, 0.0, AHEAD, 1.0);
        let other = at(0.0, 0.5, 10.0, BACK, 1.0);
        assert!(threat(&me, 10.0, &[&other]).is_some());
        assert!(threat(&me, 10.0, &[&at(0.0, 1.5, 10.0, BACK, 1.0)]).is_none());
    }
}
