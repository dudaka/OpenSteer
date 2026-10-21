//! A point-mass vehicle with a local space, ported from OpenSteer's `SimpleVehicle` and
//! `LocalSpaceMixin` (right-handed: `side = forward x up`).

use crate::util::{blend_into_accumulator, interpolate, limit_max_deviation_angle};
use crate::Vec3;

/// A vehicle moves along `forward` at `speed`; its velocity is `forward * speed`. `forward`,
/// `side` and `up` are an orthonormal basis, rebuilt after every step.
#[derive(Debug, Clone, PartialEq)]
pub struct Vehicle {
    pub position: Vec3,
    pub forward: Vec3,
    pub side: Vec3,
    pub up: Vec3,
    pub speed: f32,
    pub mass: f32,
    pub radius: f32,
    /// Steering forces are clipped to this magnitude.
    pub max_force: f32,
    /// Velocity is clipped to this magnitude.
    pub max_speed: f32,
    /// Acceleration smoothed over recent steps, used for motion, banking and annotation.
    pub smoothed_acceleration: Vec3,
    /// Roll into turns like a bird (`regenerateLocalSpaceForBanking`, used by Boids) instead of
    /// keeping `up` as close to its old value as possible.
    pub banking: bool,
}

impl Default for Vehicle {
    /// `SimpleVehicle::reset`: at the origin, facing +z with +y up, at rest.
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            forward: Vec3::new(0.0, 0.0, 1.0),
            side: Vec3::new(-1.0, 0.0, 0.0),
            up: Vec3::UP,
            speed: 0.0,
            mass: 1.0,
            radius: 0.5,
            max_force: 0.1,
            max_speed: 1.0,
            smoothed_acceleration: Vec3::ZERO,
            banking: false,
        }
    }
}

impl Vehicle {
    pub fn velocity(&self) -> Vec3 {
        self.forward * self.speed
    }

    /// Where the vehicle will be after `time` seconds at its current velocity.
    pub fn predict_future_position(&self, time: f32) -> Vec3 {
        self.position + self.velocity() * time
    }

    /// Turns to a new unit `forward` (`regenerateOrthonormalBasisUF`): `side` from the new
    /// forward and the old up, then `up` from the new side and forward.
    pub fn regenerate_basis(&mut self, forward: Vec3) {
        self.forward = forward;
        self.side = forward.cross(self.up).normalize();
        self.up = self.side.cross(forward);
    }

    /// Integrates a steering `force` over `dt` seconds (`applySteeringForce`): bend it when
    /// slow, clip to `max_force`, smooth the acceleration, clip velocity to `max_speed`,
    /// Euler-integrate position, then turn to the new velocity.
    pub fn apply_steering_force(&mut self, force: Vec3, dt: f32) {
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
        if self.banking {
            self.bank(dt);
        }
        if self.speed > 0.0 {
            self.regenerate_basis(velocity / self.speed);
        }
    }

    /// Slows the vehicle along its heading (`applyBrakingForce`): speed drops by
    /// `speed * rate`, capped at `max_force`, per second. Bypasses the low-speed rule.
    pub fn apply_braking_force(&mut self, rate: f32, dt: f32) {
        let braking = (self.speed * rate).min(self.max_force);
        self.speed -= braking * dt;
    }

    /// `regenerateLocalSpaceForBanking`'s up: blended (rate `3 * dt`) toward a small global up
    /// plus the smoothed acceleration, so turning rolls the vehicle into the turn.
    fn bank(&mut self, dt: f32) {
        let bank_up = self.smoothed_acceleration * 0.05 + Vec3::new(0.0, 0.2, 0.0);
        let mut up = self.up;
        blend_into_accumulator(dt * 3.0, bank_up, &mut up);
        self.up = up.normalize();
    }

    /// `adjustRawSteeringForce`: below 20% of `max_speed`, bend the force toward `forward`,
    /// more so the slower the vehicle, so it cannot steer backward from rest.
    fn adjust_raw_steering_force(&self, force: Vec3) -> Vec3 {
        let max_adjusted_speed = 0.2 * self.max_speed;
        if self.speed > max_adjusted_speed || force == Vec3::ZERO {
            return force;
        }
        let range = self.speed / max_adjusted_speed;
        let cosine = interpolate(f64::from(range).powf(20.0) as f32, 1.0, -1.0);
        limit_max_deviation_angle(force, cosine, self.forward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_basis_is_right_handed() {
        let v = Vehicle::default();
        assert_eq!(v.forward.cross(v.up), v.side);
        assert_eq!(v.side.cross(v.forward), v.up);
    }

    /// Turning to face +x keeps up at +y: side becomes forward x up = (0, 0, 1).
    #[test]
    fn regenerate_basis_keeps_up_when_turning_flat() {
        let mut v = Vehicle::default();
        v.regenerate_basis(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(v.side, Vec3::new(0.0, 0.0, 1.0));
        assert_eq!(v.up, Vec3::UP);
    }

    #[test]
    fn braking_is_capped_at_max_force() {
        let mut v = Vehicle {
            speed: 2.0,
            max_force: 0.5,
            ..Vehicle::default()
        };
        v.apply_braking_force(1.0, 1.0);
        assert_eq!(v.speed, 1.5);
    }
}
