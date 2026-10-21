//! Walls in 3D for wall following and containment (Reynolds 1999, not in OpenSteer): an
//! axis-aligned box, used as a solid block to follow around or as a room to stay inside.

use crate::steer::seek;
use crate::{Vec3, Vehicle};

/// An axis-aligned box from `min` to `max`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub min: Vec3,
    pub max: Vec3,
}

/// A point on a block's surface and the outward normal there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfacePoint {
    pub point: Vec3,
    pub normal: Vec3,
}

impl Block {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, p: Vec3) -> bool {
        (self.min.x..=self.max.x).contains(&p.x)
            && (self.min.y..=self.max.y).contains(&p.y)
            && (self.min.z..=self.max.z).contains(&p.z)
    }

    /// The nearest surface point to `p`. Outside: `p` clamped into the box, with the normal
    /// pointing from it to `p` (diagonal near edges and corners, so a constant offset from it
    /// is a rounded box). Inside: the nearest face, with that face's outward normal.
    pub fn nearest(&self, p: Vec3) -> SurfacePoint {
        let clamped = Vec3::new(
            p.x.clamp(self.min.x, self.max.x),
            p.y.clamp(self.min.y, self.max.y),
            p.z.clamp(self.min.z, self.max.z),
        );
        if clamped != p {
            return SurfacePoint {
                point: clamped,
                normal: (p - clamped).normalize(),
            };
        }
        let faces = [
            (p.x - self.min.x, Vec3::new(-1.0, 0.0, 0.0)),
            (self.max.x - p.x, Vec3::new(1.0, 0.0, 0.0)),
            (p.y - self.min.y, Vec3::new(0.0, -1.0, 0.0)),
            (self.max.y - p.y, Vec3::new(0.0, 1.0, 0.0)),
            (p.z - self.min.z, Vec3::new(0.0, 0.0, -1.0)),
            (self.max.z - p.z, Vec3::new(0.0, 0.0, 1.0)),
        ];
        let (depth, normal) = faces
            .into_iter()
            .reduce(|best, f| if f.0 < best.0 { f } else { best })
            .expect("six faces");
        SurfacePoint {
            point: p + normal * depth,
            normal,
        }
    }

    /// Distance from `p` to the surface, negative inside.
    pub fn signed_distance(&self, p: Vec3) -> f32 {
        let d = (p - self.nearest(p).point).length();
        if self.contains(p) {
            -d
        } else {
            d
        }
    }
}

/// The point wall following seeks: the position predicted `prediction_time` ahead, moved to
/// the nearest surface point and out along its normal by `offset`.
pub fn follow_surface_target(
    v: &Vehicle,
    block: &Block,
    offset: f32,
    prediction_time: f32,
) -> Vec3 {
    let s = block.nearest(v.predict_future_position(prediction_time));
    s.point + s.normal * offset
}

/// Steer to stay `offset` from the block's surface (`seek` of `follow_surface_target`); add a
/// forward push to move along it.
pub fn follow_surface(v: &Vehicle, block: &Block, offset: f32, prediction_time: f32) -> Vec3 {
    seek(v, follow_surface_target(v, block, offset, prediction_time))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block() -> Block {
        Block::new(Vec3::new(-2.0, 0.0, -1.0), Vec3::new(2.0, 2.0, 1.0))
    }

    #[test]
    fn nearest_point_outside_a_face_edge_and_corner() {
        let b = block();
        let face = b.nearest(Vec3::new(0.0, 5.0, 0.0));
        assert_eq!(
            (face.point, face.normal),
            (Vec3::new(0.0, 2.0, 0.0), Vec3::UP)
        );
        // beyond the top +x edge: diagonal normal
        let edge = b.nearest(Vec3::new(3.0, 3.0, 0.0));
        assert_eq!(edge.point, Vec3::new(2.0, 2.0, 0.0));
        assert!((edge.normal - Vec3::new(1.0, 1.0, 0.0).normalize()).length() < 1e-6);
        assert_eq!(b.signed_distance(Vec3::new(4.0, 4.0, 3.0)), 12.0_f32.sqrt());
    }

    /// Inside, 0.5 below the top: the top face, distance -0.5.
    #[test]
    fn nearest_point_inside_is_the_nearest_face() {
        let s = block().nearest(Vec3::new(0.0, 1.5, 0.0));
        assert_eq!((s.point, s.normal), (Vec3::new(0.0, 2.0, 0.0), Vec3::UP));
        assert_eq!(block().signed_distance(Vec3::new(0.0, 1.5, 0.0)), -0.5);
    }

    /// Above the top face at rest: the target is 1 above the surface below the predicted
    /// (= current) position.
    #[test]
    fn target_is_offset_from_the_predicted_spot() {
        let v = Vehicle {
            position: Vec3::new(1.0, 5.0, 0.5),
            ..Vehicle::default()
        };
        assert_eq!(
            follow_surface_target(&v, &block(), 1.0, 2.0),
            Vec3::new(1.0, 3.0, 0.5)
        );
    }
}
