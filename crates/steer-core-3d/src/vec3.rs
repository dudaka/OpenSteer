//! 3D vector, ported from OpenSteer's `Vec3`. Y is up; the ground is the XZ plane.

use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// `a x b` (`Vec3::cross`).
    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Unit vector parallel to `self`; a zero vector is returned unchanged.
    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len > 0.0 {
            self / len
        } else {
            self
        }
    }

    /// `self` clamped to at most `max_length`, keeping its direction.
    pub fn truncate_length(self, max_length: f32) -> Vec3 {
        let max_squared = max_length * max_length;
        let squared = self.length_squared();
        if squared <= max_squared {
            self
        } else {
            self * (max_length / squared.sqrt())
        }
    }

    /// Component parallel to `unit_basis`, which must have length 1.
    pub fn parallel_component(self, unit_basis: Vec3) -> Vec3 {
        unit_basis * self.dot(unit_basis)
    }

    /// Component perpendicular to `unit_basis`, which must have length 1.
    pub fn perpendicular_component(self, unit_basis: Vec3) -> Vec3 {
        self - self.parallel_component(unit_basis)
    }

    /// Leaving a sphere teleports to the opposite side (`sphericalWrapAround`): beyond
    /// `radius` from `center`, move back by twice the radius through the center.
    pub fn spherical_wrap_around(self, center: Vec3, radius: f32) -> Vec3 {
        let offset = self - center;
        let r = offset.length();
        if r > radius {
            self + (offset / r) * radius * -2.0
        } else {
            self
        }
    }

    /// `self` on the ground plane (`setYtoZero`).
    pub fn set_y_to_zero(self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z)
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Vec3) {
        *self = *self + rhs;
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Vec3) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f32) -> Vec3 {
        Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, rhs: f32) -> Vec3 {
        Vec3::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_is_right_handed() {
        let (x, y, z) = (
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        assert_eq!(x.cross(y), z);
        assert_eq!(y.cross(z), x);
        // the default vehicle: forward +z, up +y, so side = forward x up = -x
        assert_eq!(z.cross(y), -x);
    }

    #[test]
    fn length_truncate_and_components() {
        let v = Vec3::new(2.0, 3.0, 6.0);
        assert_eq!(v.length(), 7.0);
        assert_eq!(v.truncate_length(14.0), v);
        assert_eq!(v.truncate_length(3.5), Vec3::new(1.0, 1.5, 3.0));
        let up = Vec3::UP;
        assert_eq!(v.parallel_component(up), Vec3::new(0.0, 3.0, 0.0));
        assert_eq!(v.perpendicular_component(up), v.set_y_to_zero());
        assert_eq!(Vec3::ZERO.normalize(), Vec3::ZERO);
    }
}
