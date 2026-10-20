//! A point-mass vehicle and its integrator, ported from OpenSteer's `SimpleVehicle`.

use crate::util::{blend_into_accumulator, interpolate, limit_max_deviation_angle};
use crate::Vec2;

/// A vehicle moves along `forward` at `speed`; its velocity is `forward * speed`.
#[derive(Debug, Clone, PartialEq)]
pub struct Vehicle {
    pub position: Vec2,
    /// Unit heading; realigned with the velocity after every step.
    pub forward: Vec2,
    pub speed: f32,
    pub mass: f32,
    pub radius: f32,
    /// Steering forces are clipped to this magnitude.
    pub max_force: f32,
    /// Velocity is clipped to this magnitude.
    pub max_speed: f32,
    /// Acceleration smoothed over recent steps, used for motion and annotation.
    pub smoothed_acceleration: Vec2,
}

impl Default for Vehicle {
    /// `SimpleVehicle::reset`: at the origin, facing C++ +z, at rest.
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            forward: Vec2::new(0.0, 1.0),
            speed: 0.0,
            mass: 1.0,
            radius: 0.5,
            max_force: 0.1,
            max_speed: 1.0,
            smoothed_acceleration: Vec2::ZERO,
        }
    }
}

impl Vehicle {
    pub fn velocity(&self) -> Vec2 {
        self.forward * self.speed
    }

    /// Unit vector 90 degrees from `forward`, as OpenSteer's `side()`.
    pub fn side(&self) -> Vec2 {
        self.forward.perp()
    }

    /// Integrates a steering `force` over `dt` seconds (`applySteeringForce`): clip to
    /// `max_force`, smooth the acceleration, clip velocity to `max_speed`, Euler-integrate
    /// position, then turn `forward` to the new velocity.
    pub fn apply_steering_force(&mut self, force: Vec2, dt: f32) {
        let clipped = self
            .adjust_raw_steering_force(force)
            .truncate_length(self.max_force);
        let acceleration = clipped / self.mass;
        if dt > 0.0 {
            let smooth_rate = (9.0 * dt).clamp(0.15, 0.4);
            blend_into_accumulator(smooth_rate, acceleration, &mut self.smoothed_acceleration);
        }
        let velocity =
            (self.velocity() + self.smoothed_acceleration * dt).truncate_length(self.max_speed);
        self.speed = velocity.length();
        self.position += velocity * dt;
        if self.speed > 0.0 {
            self.forward = velocity / self.speed;
        }
    }

    /// Slows the vehicle along its heading (`applyBrakingForce`): speed drops by
    /// `speed * rate`, capped at `max_force`, per second. Bypasses the low-speed rule.
    pub fn apply_braking_force(&mut self, rate: f32, dt: f32) {
        let braking = (self.speed * rate).min(self.max_force);
        self.speed -= braking * dt;
    }

    /// `adjustRawSteeringForce`: below 20% of `max_speed`, bend the force toward `forward`,
    /// more so the slower the vehicle, so it cannot steer backward from rest.
    fn adjust_raw_steering_force(&self, force: Vec2) -> Vec2 {
        let max_adjusted_speed = 0.2 * self.max_speed;
        if self.speed > max_adjusted_speed || force == Vec2::ZERO {
            return force;
        }
        let range = self.speed / max_adjusted_speed;
        let cosine = interpolate(range.powi(20), 1.0, -1.0);
        limit_max_deviation_angle(force, cosine, self.forward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn moving() -> Vehicle {
        Vehicle {
            speed: 1.5,
            max_force: 0.3,
            max_speed: 5.0,
            ..Vehicle::default()
        }
    }

    #[test]
    fn speed_never_exceeds_max_and_forward_stays_unit() {
        let mut v = moving();
        for _ in 0..2000 {
            v.apply_steering_force(Vec2::new(0.0, 100.0), DT);
            assert!(v.speed <= v.max_speed + 1e-5, "speed {}", v.speed);
            assert!((v.forward.length() - 1.0).abs() < 1e-5);
        }
        assert!((v.speed - v.max_speed).abs() < 1e-3);
    }

    #[test]
    fn zero_force_coasts_in_a_straight_line() {
        let mut v = moving();
        v.apply_steering_force(Vec2::ZERO, 1.0);
        assert_eq!(v.position, Vec2::new(0.0, 1.5));
        assert_eq!(v.forward, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn zero_dt_does_not_move() {
        let mut v = moving();
        v.apply_steering_force(Vec2::new(1.0, 0.0), 0.0);
        assert_eq!(v.position, Vec2::ZERO);
        assert_eq!(v.smoothed_acceleration, Vec2::ZERO);
    }

    #[test]
    fn at_rest_a_backward_force_is_bent_forward() {
        let v = Vehicle::default();
        let adjusted = v.adjust_raw_steering_force(Vec2::new(0.05, -1.0));
        assert!(adjusted.normalize().dot(v.forward) > 0.999);
    }

    #[test]
    fn braking_is_proportional_to_speed_and_capped_at_max_force() {
        let mut v = moving();
        // speed 1.5 * rate 0.1 = 0.15 < max_force 0.3: 1.5 - 0.15 * 0.5
        v.apply_braking_force(0.1, 0.5);
        assert!((v.speed - 1.425).abs() < 1e-6);
        // 1.425 * 1 = 1.425 capped at 0.3: 1.425 - 0.3 * 0.5
        v.apply_braking_force(1.0, 0.5);
        assert!((v.speed - 1.275).abs() < 1e-6);
    }

    #[test]
    fn side_is_perpendicular_to_forward() {
        assert_eq!(Vehicle::default().side(), Vec2::new(-1.0, 0.0));
    }
}
