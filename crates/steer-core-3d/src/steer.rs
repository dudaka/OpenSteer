//! Steering behaviors from OpenSteer's `SteerLibrary.h`, in 3D. Each returns a force for
//! `Vehicle::apply_steering_force`; callers combine them.

use crate::pathway::Pathway;
use crate::util::scalar_random_walk;
use crate::{Vec3, Vehicle};

/// Steer toward `target` (`xxxsteerForSeek`, the version the plug-ins use). The desired
/// velocity is the offset truncated to `max_speed`, so the vehicle slows once it is closer
/// than `max_speed` units.
pub fn seek(v: &Vehicle, target: Vec3) -> Vec3 {
    (target - v.position).truncate_length(v.max_speed) - v.velocity()
}

/// Steer away from `target` (`xxxsteerForFlee`): desired velocity is the offset away from it,
/// truncated to `max_speed`.
pub fn flee(v: &Vehicle, target: Vec3) -> Vec3 {
    (v.position - target).truncate_length(v.max_speed) - v.velocity()
}

/// Plain `steerForSeek`: desired velocity is the whole offset, not truncated to `max_speed`.
fn seek_plain(v: &Vehicle, target: Vec3) -> Vec3 {
    (target - v.position) - v.velocity()
}

/// Plain `steerForFlee`: desired velocity is the whole offset away from `target`.
fn flee_plain(v: &Vehicle, target: Vec3) -> Vec3 {
    (v.position - target) - v.velocity()
}

/// Where a pursuer aims (`steerForPursuit`'s estimate) and why.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intercept {
    /// The quarry's predicted position.
    pub target: Vec3,
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
pub fn pursuit(v: &Vehicle, quarry: &Vehicle, max_prediction_time: f32) -> Vec3 {
    seek_plain(v, intercept(v, quarry, max_prediction_time).target)
}

/// Steer away from where `menace` will be (`steerForEvasion`, whose C++ does not compile:
/// ported as intended). The prediction time is `distance / menace speed`, capped at
/// `max_prediction_time`; the flee is plain, so the desired speed is the distance.
pub fn evasion(v: &Vehicle, menace: &Vehicle, max_prediction_time: f32) -> Vec3 {
    let distance = (menace.position - v.position).length();
    let time = (distance / menace.speed).min(max_prediction_time);
    flee_plain(v, menace.predict_future_position(time))
}

/// The point `stay_on_path` seeks, if any: where the position predicted `prediction_time` ahead
/// maps onto the path, when that prediction is outside the path.
pub fn stay_on_path_target(v: &Vehicle, prediction_time: f32, path: &Pathway) -> Option<Vec3> {
    let m = path.map_point(v.predict_future_position(prediction_time));
    (m.outside >= 0.0).then_some(m.on_path)
}

/// Steer back into the path when the predicted position leaves it (`steerToStayOnPath`): plain
/// seek of that position's projection; zero while it stays inside.
pub fn stay_on_path(v: &Vehicle, prediction_time: f32, path: &Pathway) -> Vec3 {
    stay_on_path_target(v, prediction_time, path).map_or(Vec3::ZERO, |t| seek_plain(v, t))
}

/// The point `follow_path` seeks, if any. None when the predicted position is inside the path
/// and further along it in `direction` (+1 or -1) than the current one; otherwise the path
/// point `direction * prediction_time * speed` beyond the current path distance.
pub fn follow_path_target(
    v: &Vehicle,
    direction: f32,
    prediction_time: f32,
    path: &Pathway,
) -> Option<Vec3> {
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
pub fn follow_path(v: &Vehicle, direction: f32, prediction_time: f32, path: &Pathway) -> Vec3 {
    follow_path_target(v, direction, prediction_time, path).map_or(Vec3::ZERO, |t| seek_plain(v, t))
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

/// `steerForWander`'s two random walk values (`WanderSide`, `WanderUp`), each in `-1..=1`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WanderState {
    pub side: f32,
    pub up: f32,
}

/// A random lateral force (`steerForWander`): `side` and `up` each random-walk at `12 * dt` per
/// call (two draws, side first) and the force is `side * state.side + up * state.up`. Add it to
/// a forward force.
pub fn wander(
    v: &Vehicle,
    state: &mut WanderState,
    dt: f32,
    random01: &mut impl FnMut() -> f32,
) -> Vec3 {
    let speed = 12.0 * dt;
    state.side = scalar_random_walk(state.side, speed, -1.0, 1.0, random01);
    state.up = scalar_random_walk(state.up, speed, -1.0, 1.0, random01);
    v.side * state.side + v.up * state.up
}

/// A force along `forward` toward `target_speed` (`steerForTargetSpeed`): the speed error,
/// clipped to `max_force`.
pub fn target_speed(v: &Vehicle, target_speed: f32) -> Vec3 {
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

    /// Facing +z at speed 1, target straight up 2: want (0, 2, 0), have (0, 0, 1).
    #[test]
    fn seek_is_want_minus_have() {
        assert_eq!(
            seek(&moving(), Vec3::new(0.0, 2.0, 0.0)),
            Vec3::new(0.0, 2.0, -1.0)
        );
    }

    /// Target 6 above: want is capped at max speed 3.
    #[test]
    fn seek_caps_the_want_at_max_speed() {
        assert_eq!(
            seek(&moving(), Vec3::new(0.0, 6.0, 0.0)),
            Vec3::new(0.0, 3.0, -1.0)
        );
    }

    #[test]
    fn flee_wants_the_opposite_way() {
        assert_eq!(
            flee(&moving(), Vec3::new(0.0, 2.0, 0.0)),
            Vec3::new(0.0, -2.0, -1.0)
        );
    }

    /// Draws 1 then 0: side walks +12 dt, up walks -12 dt; the default side is
    /// (-1, 0, 0) and up (0, 1, 0).
    #[test]
    fn wander_walks_side_then_up() {
        let mut state = WanderState::default();
        let mut draws = [1.0, 0.0].into_iter();
        let force = wander(&moving(), &mut state, 1.0 / 60.0, &mut || {
            draws.next().unwrap()
        });
        let step = 12.0 * (1.0 / 60.0_f32);
        assert_eq!(
            state,
            WanderState {
                side: step,
                up: -step
            }
        );
        assert_eq!(force, Vec3::new(-step, -step, 0.0));
    }

    #[test]
    fn target_speed_is_clipped_to_max_force() {
        // speed 1, target 3, max force 0.1: push 0.1 along forward (+z)
        assert_eq!(target_speed(&moving(), 3.0), Vec3::new(0.0, 0.0, 0.1));
    }

    /// Quarry 10 straight above, heading +z like us: aside, parallel (factor 1). Our speed 1, so
    /// the direct time is 10 s, capped at 4; the quarry at speed 2 is predicted 8 further on.
    #[test]
    fn intercept_of_a_quarry_overhead() {
        let quarry = Vehicle {
            position: Vec3::new(0.0, 10.0, 0.0),
            speed: 2.0,
            ..Vehicle::default()
        };
        let i = intercept(&moving(), &quarry, 4.0);
        assert_eq!(i.case, "aside, parallel");
        assert_eq!(i.target, Vec3::new(0.0, 10.0, 8.0));
    }

    /// Menace 3 below at speed 1 heading +z, max 2 s ahead: it will be at (0, -3, 2); plain flee
    /// wants the whole offset (0, 3, -2), minus our velocity (0, 0, 1).
    #[test]
    fn evasion_flees_the_predicted_spot() {
        let menace = Vehicle {
            position: Vec3::new(0.0, -3.0, 0.0),
            speed: 1.0,
            ..Vehicle::default()
        };
        assert_eq!(evasion(&moving(), &menace, 2.0), Vec3::new(0.0, 3.0, -3.0));
    }
}
