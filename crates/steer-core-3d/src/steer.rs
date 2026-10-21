//! Steering behaviors from OpenSteer's `SteerLibrary.h`, in 3D. Each returns a force for
//! `Vehicle::apply_steering_force`; callers combine them.

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
}
