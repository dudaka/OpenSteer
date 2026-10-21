//! Steering behaviors from OpenSteer's `SteerLibrary.h`, in 3D. Each returns a force for
//! `Vehicle::apply_steering_force`; callers combine them.

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
}
