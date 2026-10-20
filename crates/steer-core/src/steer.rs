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
}
