//! Steering behaviors from OpenSteer's `SteerLibrary.h`. Each returns a force for
//! `Vehicle::apply_steering_force`; callers combine them.

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
