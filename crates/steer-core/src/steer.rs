//! Steering behaviors from OpenSteer's `SteerLibrary.h`. Each returns a force for
//! `Vehicle::apply_steering_force`; callers combine them.

use crate::pathway::Pathway;
use crate::util::scalar_random_walk;
use crate::{Vec2, Vehicle};

/// Steer toward `target` (`xxxsteerForSeek`, the version the plug-ins use). The desired
/// velocity is the offset truncated to `max_speed`, so the vehicle slows once it is closer
/// than `max_speed` units.
pub fn seek(v: &Vehicle, target: Vec2) -> Vec2 {
    (target - v.position).truncate_length(v.max_speed) - v.velocity()
}

/// Steer away from `target` (`xxxsteerForFlee`): desired velocity is the offset away from it,
/// truncated to `max_speed`.
pub fn flee(v: &Vehicle, target: Vec2) -> Vec2 {
    (v.position - target).truncate_length(v.max_speed) - v.velocity()
}

/// Plain `steerForSeek`: desired velocity is the whole offset, not truncated to `max_speed`.
fn seek_plain(v: &Vehicle, target: Vec2) -> Vec2 {
    (target - v.position) - v.velocity()
}

/// Plain `steerForFlee`: desired velocity is the whole offset away from `target`.
fn flee_plain(v: &Vehicle, target: Vec2) -> Vec2 {
    (v.position - target) - v.velocity()
}

/// Where a pursuer aims (`steerForPursuit`'s estimate) and why.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intercept {
    /// The quarry's predicted position.
    pub target: Vec2,
    /// Multiplies the direct travel time `distance / speed`.
    pub time_factor: f32,
    /// The case of the table, e.g. "ahead, parallel".
    pub case: &'static str,
}

/// `steerForPursuit`'s intercept estimate: the direct travel time `distance / speed`, scaled by
/// a factor from a 3x3 table (quarry ahead/aside/behind x heading parallel/perpendicular/
/// anti-parallel, each split at cos 45 degrees), capped at `max_prediction_time`.
pub fn intercept(v: &Vehicle, quarry: &Vehicle, max_prediction_time: f32) -> Intercept {
    let offset = quarry.position - v.position;
    let distance = offset.length();
    let parallelness = v.forward.dot(quarry.forward);
    let forwardness = v.forward.dot(offset / distance);
    let direct_travel_time = distance / v.speed;
    let (time_factor, case) = match (
        interval_comparison(forwardness, -0.707, 0.707),
        interval_comparison(parallelness, -0.707, 0.707),
    ) {
        (1, 1) => (4.0, "ahead, parallel"),
        (1, 0) => (1.8, "ahead, perpendicular"),
        (1, _) => (0.85, "ahead, anti-parallel"),
        (0, 1) => (1.0, "aside, parallel"),
        (0, 0) => (0.8, "aside, perpendicular"),
        (0, _) => (4.0, "aside, anti-parallel"),
        (_, 1) => (0.5, "behind, parallel"),
        (_, 0) => (2.0, "behind, perpendicular"),
        (_, _) => (2.0, "behind, anti-parallel"),
    };
    let time = (direct_travel_time * time_factor).min(max_prediction_time);
    Intercept {
        target: quarry.predict_future_position(time),
        time_factor,
        case,
    }
}

/// Steer toward where `quarry` will be (`steerForPursuit`): plain seek of the intercept.
pub fn pursuit(v: &Vehicle, quarry: &Vehicle, max_prediction_time: f32) -> Vec2 {
    seek_plain(v, intercept(v, quarry, max_prediction_time).target)
}

/// Steer away from where `menace` will be (`steerForEvasion`, whose C++ does not compile:
/// ported as intended). The prediction time is `distance / menace speed`, capped at
/// `max_prediction_time`; the flee is plain, so the desired speed is the distance.
pub fn evasion(v: &Vehicle, menace: &Vehicle, max_prediction_time: f32) -> Vec2 {
    let distance = (menace.position - v.position).length();
    let time = (distance / menace.speed).min(max_prediction_time);
    flee_plain(v, menace.predict_future_position(time))
}

/// The point `stay_on_path` seeks, if any: where the position predicted `prediction_time` ahead
/// maps onto the path, when that prediction is outside the path.
pub fn stay_on_path_target(v: &Vehicle, prediction_time: f32, path: &Pathway) -> Option<Vec2> {
    let m = path.map_point(v.predict_future_position(prediction_time));
    (m.outside >= 0.0).then_some(m.on_path)
}

/// Steer back into the path when the predicted position leaves it (`steerToStayOnPath`): plain
/// seek of that position's projection; zero while it stays inside.
pub fn stay_on_path(v: &Vehicle, prediction_time: f32, path: &Pathway) -> Vec2 {
    stay_on_path_target(v, prediction_time, path).map_or(Vec2::ZERO, |t| seek_plain(v, t))
}

/// The point `follow_path` seeks, if any. None when the predicted position is inside the path
/// and further along it in `direction` (+1 or -1) than the current one; otherwise the path
/// point `direction * prediction_time * speed` beyond the current path distance.
pub fn follow_path_target(
    v: &Vehicle,
    direction: f32,
    prediction_time: f32,
    path: &Pathway,
) -> Option<Vec2> {
    let offset = direction * prediction_time * v.speed;
    let now = path.map_point(v.position).distance;
    let future = path.map_point(v.predict_future_position(prediction_time));
    let right_way = if offset > 0.0 {
        now < future.distance
    } else {
        now > future.distance
    };
    if future.outside < 0.0 && right_way {
        None
    } else {
        Some(path.point_at(now + offset))
    }
}

/// Move along the path in `direction` (`steerToFollowPath`): plain seek of
/// `follow_path_target`; zero while on course.
pub fn follow_path(v: &Vehicle, direction: f32, prediction_time: f32, path: &Pathway) -> Vec2 {
    follow_path_target(v, direction, prediction_time, path).map_or(Vec2::ZERO, |t| seek_plain(v, t))
}

/// -1 below `lower`, +1 above `upper`, 0 inside (`intervalComparison`).
fn interval_comparison(x: f32, lower: f32, upper: f32) -> i32 {
    if x < lower {
        -1
    } else if x > upper {
        1
    } else {
        0
    }
}

/// Steer toward `target` and come to rest on it (Reynolds 1999, not in OpenSteer). Outside
/// `slowing_radius` the desired speed is `max_speed`; inside it falls linearly to zero at the
/// target.
pub fn arrive(v: &Vehicle, target: Vec2, slowing_radius: f32) -> Vec2 {
    let offset = target - v.position;
    let distance = offset.length();
    if distance == 0.0 {
        return -v.velocity();
    }
    let desired_speed = v.max_speed * (distance / slowing_radius).min(1.0);
    offset / distance * desired_speed - v.velocity()
}

/// A random sideways force (`steerForWander`): `wander_side` random-walks in `-1..=1` at
/// `12 * dt` per call and the force is `side * wander_side`. Add it to a forward force. The C++
/// also walks an `up` value; on the ground plane it has no effect, so it is not ported.
pub fn wander(
    v: &Vehicle,
    wander_side: &mut f32,
    dt: f32,
    random01: &mut impl FnMut() -> f32,
) -> Vec2 {
    *wander_side = scalar_random_walk(*wander_side, 12.0 * dt, -1.0, 1.0, random01);
    v.side() * *wander_side
}

/// A force along `forward` toward `target_speed` (`steerForTargetSpeed`): the speed error,
/// clipped to `max_force`.
pub fn target_speed(v: &Vehicle, target_speed: f32) -> Vec2 {
    v.forward * (target_speed - v.speed).clamp(-v.max_force, v.max_force)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moving() -> Vehicle {
        Vehicle {
            speed: 1.0,
            max_speed: 3.0,
            ..Vehicle::default()
        }
    }

    #[test]
    fn seek_far_target_asks_for_max_speed_toward_it() {
        let v = moving();
        // desired (3, 0) minus velocity (0, 1)
        assert_eq!(seek(&v, Vec2::new(10.0, 0.0)), Vec2::new(3.0, -1.0));
    }

    #[test]
    fn seek_near_target_asks_for_its_distance_as_speed() {
        let v = moving();
        assert_eq!(seek(&v, Vec2::new(0.0, 1.0)), Vec2::ZERO);
    }

    #[test]
    fn flee_is_seek_mirrored() {
        let v = moving();
        assert_eq!(flee(&v, Vec2::new(10.0, 0.0)), Vec2::new(-3.0, -1.0));
    }

    #[test]
    fn wander_is_sideways_and_bounded() {
        let v = moving();
        let mut side = 0.95;
        let force = wander(&v, &mut side, 1.0 / 60.0, &mut || 1.0);
        assert_eq!(side, 1.0);
        assert_eq!(force, v.side());
        assert_eq!(force.dot(v.forward), 0.0);
    }

    #[test]
    fn target_speed_pushes_along_forward_clipped() {
        let v = moving();
        // speed 1, max_force 0.1
        assert_eq!(target_speed(&v, 1.0625), Vec2::new(0.0, 0.0625));
        assert_eq!(target_speed(&v, 9.0), Vec2::new(0.0, 0.1));
        assert_eq!(target_speed(&v, 0.0), Vec2::new(0.0, -0.1));
    }

    fn at(x: f32, y: f32, forward: Vec2, speed: f32) -> Vehicle {
        Vehicle {
            position: Vec2::new(x, y),
            forward,
            speed,
            ..Vehicle::default()
        }
    }

    #[test]
    fn intercept_uses_the_table() {
        let north = Vec2::new(0.0, 1.0);
        let east = Vec2::new(1.0, 0.0);
        let me = at(0.0, 0.0, north, 1.0);
        // Ahead (0, 2), same heading: 2 s * 4 = 8 s, quarry moves 8 north.
        let i = intercept(&me, &at(0.0, 2.0, north, 1.0), 20.0);
        assert_eq!(
            (i.case, i.target),
            ("ahead, parallel", Vec2::new(0.0, 10.0))
        );
        // Behind, perpendicular: 2 s * 2 = 4 s, capped at 3.
        let i = intercept(&me, &at(0.0, -2.0, east, 1.0), 3.0);
        assert_eq!(
            (i.case, i.target),
            ("behind, perpendicular", Vec2::new(3.0, -2.0))
        );
    }

    #[test]
    fn pursuit_seeks_the_intercept_without_truncation() {
        let north = Vec2::new(0.0, 1.0);
        let me = at(0.0, 0.0, north, 1.0);
        // aim (0, 10) - velocity (0, 1)
        assert_eq!(
            pursuit(&me, &at(0.0, 2.0, north, 1.0), 20.0),
            Vec2::new(0.0, 9.0)
        );
    }

    #[test]
    fn evasion_flees_the_predicted_position() {
        let me = at(0.0, 0.0, Vec2::new(0.0, 1.0), 0.0);
        // menace at (4, 0) heading west at 2: time 4 / 2 = 2 s, capped at 1 -> (2, 0)
        let menace = at(4.0, 0.0, Vec2::new(-1.0, 0.0), 2.0);
        assert_eq!(evasion(&me, &menace, 1.0), Vec2::new(-2.0, 0.0));
    }

    fn straight_path() -> Pathway {
        Pathway::new(&[Vec2::ZERO, Vec2::new(0.0, 20.0)], 1.0, false)
    }

    #[test]
    fn on_course_needs_no_path_steering() {
        let v = at(0.0, 2.0, Vec2::new(0.0, 1.0), 1.0);
        assert_eq!(stay_on_path(&v, 3.0, &straight_path()), Vec2::ZERO);
        assert_eq!(follow_path(&v, 1.0, 3.0, &straight_path()), Vec2::ZERO);
    }

    #[test]
    fn heading_the_wrong_way_turns_toward_the_path_ahead() {
        // At y = 2 heading -y at speed 1: target is 3 further along, (0, 5).
        let v = at(0.0, 2.0, Vec2::new(0.0, -1.0), 1.0);
        assert_eq!(
            follow_path_target(&v, 1.0, 3.0, &straight_path()),
            Some(Vec2::new(0.0, 5.0))
        );
        // stay_on_path does not care about direction while the prediction stays inside.
        assert_eq!(stay_on_path(&v, 1.0, &straight_path()), Vec2::ZERO);
    }

    #[test]
    fn drifting_out_seeks_the_projection() {
        // At (0, 5) heading +x at 1: in 3 s at (3, 5), outside by 2; seek (0, 5).
        let v = at(0.0, 5.0, Vec2::new(1.0, 0.0), 1.0);
        assert_eq!(
            stay_on_path_target(&v, 3.0, &straight_path()),
            Some(Vec2::new(0.0, 5.0))
        );
    }

    #[test]
    fn arrive_outside_radius_is_seek() {
        let v = moving();
        let target = Vec2::new(10.0, 0.0);
        assert_eq!(arrive(&v, target, 9.0), seek(&v, target));
    }

    #[test]
    fn arrive_inside_radius_ramps_speed_with_distance() {
        let v = moving();
        // distance 3 of radius 9: desired speed 3 * 3/9 = 1 toward +x, minus velocity (0, 1)
        let force = arrive(&v, Vec2::new(3.0, 0.0), 9.0);
        assert!((force - Vec2::new(1.0, -1.0)).length() < 1e-6);
    }

    #[test]
    fn arrive_on_target_cancels_velocity() {
        let v = moving();
        assert_eq!(arrive(&v, Vec2::ZERO, 9.0), Vec2::new(0.0, -1.0));
    }
}
