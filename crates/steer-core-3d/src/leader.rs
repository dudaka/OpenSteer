//! Leader following in 3D (Reynolds 1999, not in OpenSteer): arrive at a point behind the
//! leader, keep apart from the other followers, and step aside when in front of the leader.
//! Built from the ported `arrive` and `separation`; the leader's path is a cylinder.

use crate::flock::separation;
use crate::steer::arrive;
use crate::{Vec3, Vehicle};

/// How a follower is steering this step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowCase {
    /// Arriving at the point behind the leader, plus separation.
    Follow,
    /// Inside the leader's path: moving straight out of it.
    StepAside,
}

/// The leader-following settings shared by every follower.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeaderFollow {
    /// How far behind the leader followers gather.
    pub behind: f32,
    /// Arrive's slowing radius around the point behind.
    pub slowing_radius: f32,
    /// Length ahead of the leader that followers keep clear of; the cylinder's radius is the
    /// leader's radius plus a follower's diameter.
    pub path_length: f32,
    /// Separation's neighborhood (as `flock::separation`) and weight.
    pub separation_distance: f32,
    pub separation_weight: f32,
}

/// The point `behind` units back along the leader's heading.
pub fn behind_point(leader: &Vehicle, behind: f32) -> Vec3 {
    leader.position - leader.forward * behind
}

/// The offset from the leader's heading axis when `v` is inside the cylinder `path_length`
/// ahead of the leader with radius `leader.radius + 2 * v.radius`; `None` outside it.
pub fn in_leader_path(v: &Vehicle, leader: &Vehicle, path_length: f32) -> Option<Vec3> {
    let offset = v.position - leader.position;
    let ahead = offset.dot(leader.forward);
    let across = offset.perpendicular_component(leader.forward);
    let radius = leader.radius + 2.0 * v.radius;
    (ahead > 0.0 && ahead < path_length && across.length() < radius).then_some(across)
}

/// The follower force and which case produced it. In the leader's path: a desired velocity of
/// `max_speed` straight out of the cylinder, away from its axis (along the leader's `side` when
/// exactly on the axis). Otherwise `arrive` at the point behind plus weighted separation from
/// `others`; weight 0 skips separation (followers can then land on one spot, where it is 0/0).
pub fn follow_leader<'a>(
    v: &Vehicle,
    leader: &Vehicle,
    settings: &LeaderFollow,
    others: impl IntoIterator<Item = &'a Vehicle>,
) -> (Vec3, FollowCase) {
    if let Some(across) = in_leader_path(v, leader, settings.path_length) {
        let away = if across == Vec3::ZERO {
            leader.side
        } else {
            across.normalize()
        };
        return (away * v.max_speed - v.velocity(), FollowCase::StepAside);
    }
    let force = arrive(
        v,
        behind_point(leader, settings.behind),
        settings.slowing_radius,
    );
    if settings.separation_weight == 0.0 {
        return (force, FollowCase::Follow);
    }
    let apart = separation(v, settings.separation_distance, -0.707, others);
    (
        force + apart * settings.separation_weight,
        FollowCase::Follow,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Facing +z, max speed 2, at rest.
    fn at(x: f32, y: f32, z: f32) -> Vehicle {
        Vehicle {
            position: Vec3::new(x, y, z),
            max_speed: 2.0,
            ..Vehicle::default()
        }
    }

    fn settings() -> LeaderFollow {
        LeaderFollow {
            behind: 2.0,
            slowing_radius: 3.0,
            path_length: 4.0,
            separation_distance: 2.0,
            separation_weight: 1.0,
        }
    }

    #[test]
    fn behind_point_is_back_along_the_heading() {
        assert_eq!(
            behind_point(&at(1.0, 2.0, 5.0), 2.0),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    /// Leader at the origin facing +z, radius 0.5; followers radius 0.5: the path is the
    /// cylinder 0 < z < 4 of radius 1.5 around the z axis.
    #[test]
    fn leader_path_is_a_cylinder_ahead() {
        let leader = at(0.0, 0.0, 0.0);
        assert_eq!(
            in_leader_path(&at(0.0, 1.0, 2.0), &leader, 4.0),
            Some(Vec3::new(0.0, 1.0, 0.0))
        );
        assert_eq!(in_leader_path(&at(1.2, 1.2, 2.0), &leader, 4.0), None);
        assert_eq!(in_leader_path(&at(0.0, 0.0, 4.5), &leader, 4.0), None);
        assert_eq!(in_leader_path(&at(0.0, 0.0, -1.0), &leader, 4.0), None);
    }

    /// One above the axis: straight up out of the cylinder at max speed.
    #[test]
    fn in_the_path_steps_out_away_from_the_axis() {
        let leader = at(0.0, 0.0, 0.0);
        let (force, case) = follow_leader(&at(0.0, 1.0, 2.0), &leader, &settings(), []);
        assert_eq!(case, FollowCase::StepAside);
        assert_eq!(force, Vec3::new(0.0, 2.0, 0.0));
        // exactly on the axis: along the leader's side (-x)
        let (force, _) = follow_leader(&at(0.0, 0.0, 2.0), &leader, &settings(), []);
        assert_eq!(force, Vec3::new(-2.0, 0.0, 0.0));
    }

    #[test]
    fn behind_the_leader_arrives_at_the_point_behind() {
        let leader = at(0.0, 3.0, 10.0);
        let v = at(0.0, 0.0, 0.0);
        let (force, case) = follow_leader(&v, &leader, &settings(), []);
        assert_eq!(case, FollowCase::Follow);
        assert_eq!(force, arrive(&v, Vec3::new(0.0, 3.0, 8.0), 3.0));
    }

    #[test]
    fn zero_weight_ignores_a_follower_on_the_same_spot() {
        let leader = at(0.0, 0.0, 10.0);
        let v = at(0.0, 0.0, 0.0);
        let s = LeaderFollow {
            separation_weight: 0.0,
            ..settings()
        };
        let (force, _) = follow_leader(&v, &leader, &s, [&at(0.0, 0.0, 0.0)]);
        assert_eq!(force, arrive(&v, Vec3::new(0.0, 0.0, 8.0), 3.0));
    }
}
