//! 2D vector, ported from OpenSteer's `Vec3` (C++ `(x, z)` on the ground plane is `(x, y)`).

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Unit vector parallel to `self`; a zero vector is returned unchanged.
    pub fn normalize(self) -> Vec2 {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            self
        }
    }

    /// `self` clamped to at most `max_length`, keeping its direction.
    pub fn truncate_length(self, max_length: f32) -> Vec2 {
        if self.length_squared() <= max_length * max_length {
            self
        } else {
            self * (max_length / self.length())
        }
    }

    /// Component parallel to `unit_basis`, which must have length 1.
    pub fn parallel_component(self, unit_basis: Vec2) -> Vec2 {
        unit_basis * self.dot(unit_basis)
    }

    /// Component perpendicular to `unit_basis`, which must have length 1.
    pub fn perpendicular_component(self, unit_basis: Vec2) -> Vec2 {
        self - self.parallel_component(unit_basis)
    }

    /// `self` rotated 90 degrees from forward to side, as OpenSteer's right-handed
    /// `localRotateForwardToSide`: `(x, z) -> (-z, x)`.
    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = *self + rhs;
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Vec2) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2::new(self.x * rhs, self.y * rhs)
    }
}

impl Div<f32> for Vec2 {
    type Output = Vec2;
    fn div(self, rhs: f32) -> Vec2 {
        Vec2::new(self.x / rhs, self.y / rhs)
    }
}

impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_and_normalize() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.length(), 5.0);
        assert_eq!(v.normalize(), Vec2::new(0.6, 0.8));
        assert_eq!(Vec2::ZERO.normalize(), Vec2::ZERO);
    }

    #[test]
    fn truncate_length_only_shortens() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.truncate_length(10.0), v);
        assert_eq!(v.truncate_length(1.0), Vec2::new(0.6, 0.8));
    }

    #[test]
    fn components_split_the_vector() {
        let v = Vec2::new(3.0, 4.0);
        let x = Vec2::new(1.0, 0.0);
        assert_eq!(v.parallel_component(x), Vec2::new(3.0, 0.0));
        assert_eq!(v.perpendicular_component(x), Vec2::new(0.0, 4.0));
    }

    #[test]
    fn perp_matches_opensteer_forward_to_side() {
        // C++ forward (0, 0, 1) has side (-1, 0, 0).
        assert_eq!(Vec2::new(0.0, 1.0).perp(), Vec2::new(-1.0, 0.0));
        assert_eq!(Vec2::new(3.0, 4.0).perp().dot(Vec2::new(3.0, 4.0)), 0.0);
    }
}
