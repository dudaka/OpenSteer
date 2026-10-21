//! Leader following (Reynolds 1999, not in OpenSteer): arrive at a point behind the leader,
//! keep apart from the other followers, and step aside when in front of the leader. Built from
//! the ported `arrive` and `separation`.

use crate::flock::separation;
use crate::steer::arrive;
use crate::{Vec2, Vehicle};

/// How a follower is steering this step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowCase {
    /// Arriving at the point behind the leader, plus separation.
    Follow,
    /// Inside the leader's path: moving sideways out of it.
    StepAside,
}

/// The leader-following settings shared by every follower.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeaderFollow {
    /// How far behind the leader followers gather.
    pub behind: f32,
    /// Arrive's slowing radius around the point behind.
    pub slowing_radius: f32,
    /// Length ahead of the leader that followers keep clear of; its width is the leader's
    /// diameter plus a follower's on each side.
    pub path_length: f32,
    /// Separation's neighborhood (as `flock::separation`) and weight.
    pub separation_distance: f32,
    pub separation_weight: f32,
}

/// The point `behind` units back along the leader's heading.
pub fn behind_point(leader: &Vehicle, behind: f32) -> Vec2 {
    leader.position - leader.forward * behind
}

/// The signed sideways offset from the leader's heading line when `v` is in the rectangle
/// `path_length` ahead of the leader and `leader.radius + 2 * v.radius` to each side;
/// `None` outside it.
pub fn in_leader_path(v: &Vehicle, leader: &Vehicle, path_length: f32) -> Option<f32> {
    let offset = v.position - leader.position;
    let ahead = offset.dot(leader.forward);
    let across = offset.dot(leader.side());
    let half_width = leader.radius + 2.0 * v.radius;
    (ahead > 0.0 && ahead < path_length && across.abs() < half_width).then_some(across)
}

/// The follower force and which case produced it. In the leader's path: a desired velocity
/// of `max_speed` straight out of the path, sideways from the leader's heading (toward
/// `+side` when exactly on the line). Otherwise `arrive` at the point behind plus weighted
/// separation from `others` (the other followers and the leader); weight 0 skips separation.
pub fn follow_leader<'a>(
    v: &Vehicle,
    leader: &Vehicle,
    settings: &LeaderFollow,
    others: impl IntoIterator<Item = &'a Vehicle>,
) -> (Vec2, FollowCase) {
    if let Some(across) = in_leader_path(v, leader, settings.path_length) {
        let away = if across < 0.0 { -1.0 } else { 1.0 };
        let desired = leader.side() * (away * v.max_speed);
        return (desired - v.velocity(), FollowCase::StepAside);
    }
    let force = arrive(
        v,
        behind_point(leader, settings.behind),
        settings.slowing_radius,
    );
    if settings.separation_weight == 0.0 {
        // Without separation followers can land on the same point, where separation is 0/0.
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

    const EAST: Vec2 = Vec2::new(1.0, 0.0);

    fn at(x: f32, y: f32) -> Vehicle {
        Vehicle {
            position: Vec2::new(x, y),
            forward: EAST,
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
        assert_eq!(behind_point(&at(5.0, 1.0), 2.0), Vec2::new(3.0, 1.0));
    }

    /// Leader at the origin facing east, radius 0.5; followers radius 0.5: the path is
    /// 0 < x < 4 and |y| < 1.5. The leader's side is (0, 1).
    #[test]
    fn leader_path_is_a_rectangle_ahead() {
        let leader = at(0.0, 0.0);
        assert_eq!(in_leader_path(&at(2.0, 1.0), &leader, 4.0), Some(1.0));
        assert_eq!(in_leader_path(&at(2.0, -1.0), &leader, 4.0), Some(-1.0));
        assert_eq!(in_leader_path(&at(2.0, 1.6), &leader, 4.0), None);
        assert_eq!(in_leader_path(&at(4.5, 0.0), &leader, 4.0), None);
        assert_eq!(in_leader_path(&at(-1.0, 0.0), &leader, 4.0), None);
    }

    #[test]
    fn in_the_path_steps_sideways_away() {
        let leader = at(0.0, 0.0);
        let (force, case) = follow_leader(&at(2.0, -1.0), &leader, &settings(), []);
        assert_eq!(case, FollowCase::StepAside);
        // at rest: the force is the desired velocity, max speed toward -side
        assert_eq!(force, Vec2::new(0.0, -2.0));
    }

    #[test]
    fn behind_the_leader_arrives_at_the_point_behind() {
        let leader = at(10.0, 0.0);
        let v = at(0.0, 0.0);
        let (force, case) = follow_leader(&v, &leader, &settings(), []);
        assert_eq!(case, FollowCase::Follow);
        assert_eq!(force, arrive(&v, Vec2::new(8.0, 0.0), 3.0));
    }

    #[test]
    fn separation_adds_a_push_away_from_close_followers() {
        let leader = at(10.0, 0.0);
        let v = at(0.0, 0.0);
        let close = at(0.0, 1.0);
        let (force, _) = follow_leader(&v, &leader, &settings(), [&close]);
        let (alone, _) = follow_leader(&v, &leader, &settings(), []);
        assert_eq!(force - alone, Vec2::new(0.0, -1.0));
    }

    #[test]
    fn zero_weight_ignores_a_follower_on_the_same_spot() {
        let leader = at(10.0, 0.0);
        let v = at(0.0, 0.0);
        let same = at(0.0, 0.0);
        let s = LeaderFollow {
            separation_weight: 0.0,
            ..settings()
        };
        let (force, _) = follow_leader(&v, &leader, &s, [&same]);
        assert_eq!(force, arrive(&v, Vec2::new(8.0, 0.0), 3.0));
    }
}
