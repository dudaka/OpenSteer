//! Flocking behaviors from OpenSteer's `SteerLibrary.h`: separation, alignment, cohesion, each
//! over the neighbors inside a "boid neighborhood". `others` is every other vehicle (not
//! `v` itself), in a fixed order; each force has length 1 or 0.

use crate::{Vec2, Vehicle};

/// `inBoidNeighborhood`: closer than `3 * radius` always counts; beyond `max_distance` never;
/// in between only if the direction to `other` is within the cone `forward . dir >
/// cos_max_angle`.
pub fn in_neighborhood(
    v: &Vehicle,
    other: &Vehicle,
    max_distance: f32,
    cos_max_angle: f32,
) -> bool {
    let min_distance = v.radius * 3.0;
    let offset = other.position - v.position;
    let distance_squared = offset.length_squared();
    if distance_squared < min_distance * min_distance {
        return true;
    }
    if distance_squared > max_distance * max_distance {
        return false;
    }
    v.forward.dot(offset / distance_squared.sqrt()) > cos_max_angle
}

/// `steerForSeparation`: away from each neighbor, weighted by 1 / distance, then normalized.
pub fn separation<'a>(
    v: &Vehicle,
    max_distance: f32,
    cos_max_angle: f32,
    others: impl IntoIterator<Item = &'a Vehicle>,
) -> Vec2 {
    others
        .into_iter()
        .filter(|o| in_neighborhood(v, o, max_distance, cos_max_angle))
        .fold(Vec2::ZERO, |sum, o| {
            let offset = o.position - v.position;
            sum + offset / -offset.dot(offset)
        })
        .normalize()
}

/// `steerForAlignment`: toward the neighbors' average heading, minus our own, normalized.
pub fn alignment<'a>(
    v: &Vehicle,
    max_distance: f32,
    cos_max_angle: f32,
    others: impl IntoIterator<Item = &'a Vehicle>,
) -> Vec2 {
    average(v, max_distance, cos_max_angle, others, |o| o.forward)
        .map_or(Vec2::ZERO, |heading| (heading - v.forward).normalize())
}

/// `steerForCohesion`: toward the neighbors' average position, normalized.
pub fn cohesion<'a>(
    v: &Vehicle,
    max_distance: f32,
    cos_max_angle: f32,
    others: impl IntoIterator<Item = &'a Vehicle>,
) -> Vec2 {
    average(v, max_distance, cos_max_angle, others, |o| o.position)
        .map_or(Vec2::ZERO, |center| (center - v.position).normalize())
}

/// The mean of `value` over the neighbors, or `None` without neighbors.
fn average<'a>(
    v: &Vehicle,
    max_distance: f32,
    cos_max_angle: f32,
    others: impl IntoIterator<Item = &'a Vehicle>,
    value: impl Fn(&Vehicle) -> Vec2,
) -> Option<Vec2> {
    let (sum, count) = others
        .into_iter()
        .filter(|o| in_neighborhood(v, o, max_distance, cos_max_angle))
        .fold((Vec2::ZERO, 0), |(sum, n), o| (sum + value(o), n + 1));
    (count > 0).then(|| sum / count as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32, y: f32, forward: Vec2) -> Vehicle {
        Vehicle {
            position: Vec2::new(x, y),
            forward,
            ..Vehicle::default()
        }
    }

    const NORTH: Vec2 = Vec2::new(0.0, 1.0);
    const EAST: Vec2 = Vec2::new(1.0, 0.0);

    #[test]
    fn neighborhood_is_near_or_inside_the_cone() {
        let me = at(0.0, 0.0, NORTH);
        // within 3 * radius = 1.5: always, even behind
        assert!(in_neighborhood(&me, &at(0.0, -1.0, NORTH), 5.0, 0.9));
        // behind and farther: outside a narrow cone
        assert!(!in_neighborhood(&me, &at(0.0, -3.0, NORTH), 5.0, 0.9));
        assert!(in_neighborhood(&me, &at(0.0, 3.0, NORTH), 5.0, 0.9));
        assert!(!in_neighborhood(&me, &at(0.0, 6.0, NORTH), 5.0, 0.9));
    }

    #[test]
    fn separation_points_away_weighted_by_closeness() {
        let me = at(0.0, 0.0, NORTH);
        // (2, 0) contributes (-0.5, 0); (0, 4) contributes (0, -0.25)
        let others = [at(2.0, 0.0, NORTH), at(0.0, 4.0, NORTH)];
        let s = separation(&me, 5.0, -1.0, &others);
        assert!((s - Vec2::new(-0.5, -0.25).normalize()).length() < 1e-6);
    }

    #[test]
    fn alignment_turns_toward_the_average_heading() {
        let me = at(0.0, 0.0, NORTH);
        let others = [at(0.0, 2.0, EAST)];
        assert_eq!(
            alignment(&me, 5.0, -1.0, &others),
            (EAST - NORTH).normalize()
        );
        assert_eq!(alignment(&me, 5.0, -1.0, &[]), Vec2::ZERO);
    }

    #[test]
    fn cohesion_points_at_the_center() {
        let me = at(0.0, 0.0, NORTH);
        let others = [at(2.0, 2.0, NORTH), at(-2.0, 2.0, NORTH)];
        assert_eq!(cohesion(&me, 5.0, -1.0, &others), NORTH);
    }
}
