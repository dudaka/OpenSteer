//! Sphere and rectangle obstacles and obstacle avoidance, from OpenSteer's `Obstacle.cpp`
//! (`SphereObstacle`, `RectangleObstacle`).

use crate::{Vec3, Vehicle};

/// Which side of the obstacle's surface vehicles must stay on (`seenFromState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeenFrom {
    /// A solid obstacle: stay outside.
    Outside,
    /// A container: stay inside.
    Inside,
    /// Do not cross the surface either way.
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub seen_from: SeenFrom,
}

/// `RectangleObstacle`: a `width` x `height` rectangle centered on `position` in the plane of its
/// `side` and `up` axes; `forward` is its normal and the +forward half-space its outside. Seen
/// from `Both`, it is a wall avoided from either side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    pub position: Vec3,
    pub side: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
    pub width: f32,
    pub height: f32,
    pub seen_from: SeenFrom,
}

/// An obstacle a vehicle's forward path can meet (`findIntersectionWithVehiclePath`).
pub trait Obstacle {
    fn intersect_path(&self, v: &Vehicle) -> Option<PathIntersection>;
}

/// Where a vehicle's forward path first meets an obstacle (`PathIntersection`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathIntersection {
    /// Distance along `forward` to the surface point.
    pub distance: f32,
    pub surface_point: Vec3,
    /// The direction to steer toward, away from the surface.
    pub steer_hint: Vec3,
}

impl Sphere {
    pub const fn new(center: Vec3, radius: f32, seen_from: SeenFrom) -> Self {
        Self {
            center,
            radius,
            seen_from,
        }
    }

    /// `findIntersectionWithVehiclePath`, in the vehicle's local space as the C++: the nearest
    /// point ahead where the forward axis, widened by the vehicle's radius, meets the sphere.
    /// `None` when the axis misses or both meeting points are behind.
    pub fn intersect_path(&self, v: &Vehicle) -> Option<PathIntersection> {
        let lc = v.localize_position(self.center);
        let vehicle_outside = lc.length() > self.radius;
        if vehicle_outside && self.seen_from == SeenFrom::Inside {
            return Some(PathIntersection {
                distance: 0.0,
                surface_point: v.position,
                steer_hint: (self.center - v.position).normalize(),
            });
        }
        let r = self.radius + v.radius;
        let b = -2.0 * lc.z;
        let c = lc.x * lc.x + lc.y * lc.y + lc.z * lc.z - r * r;
        let d = b * b - 4.0 * c;
        if d < 0.0 {
            return None;
        }
        let s = d.sqrt();
        let (p, q) = ((-b + s) / 2.0, (-b - s) / 2.0);
        if p < 0.0 && q < 0.0 {
            return None;
        }
        let distance = if p > 0.0 && q > 0.0 {
            p.min(q)
        } else if self.seen_from == SeenFrom::Outside {
            // Inside a solid obstacle.
            0.0
        } else if p > 0.0 {
            p
        } else {
            q
        };
        let surface_point = v.position + v.forward * distance;
        let surface_normal = (surface_point - self.center).normalize();
        let steer_hint = match self.seen_from {
            SeenFrom::Outside => surface_normal,
            SeenFrom::Inside => -surface_normal,
            SeenFrom::Both if vehicle_outside => surface_normal,
            SeenFrom::Both => -surface_normal,
        };
        Some(PathIntersection {
            distance,
            surface_point,
            steer_hint,
        })
    }
}

impl Obstacle for Sphere {
    fn intersect_path(&self, v: &Vehicle) -> Option<PathIntersection> {
        Sphere::intersect_path(self, v)
    }
}

impl Rectangle {
    /// The C++ constructor's order: width, height, side, up, forward, position, seen from.
    pub const fn new(
        width: f32,
        height: f32,
        side: Vec3,
        up: Vec3,
        forward: Vec3,
        position: Vec3,
        seen_from: SeenFrom,
    ) -> Self {
        Self {
            position,
            side,
            up,
            forward,
            width,
            height,
            seen_from,
        }
    }

    fn localize_direction(&self, d: Vec3) -> Vec3 {
        Vec3::new(d.dot(self.side), d.dot(self.up), d.dot(self.forward))
    }

    fn globalize_direction(&self, l: Vec3) -> Vec3 {
        self.side * l.x + self.up * l.y + self.forward * l.z
    }

    /// `xyPointInsideShape`: inside the rectangle widened by `radius` on every side.
    fn inside_shape(&self, point: Vec3, radius: f32) -> bool {
        let w = radius + self.width * 0.5;
        let h = radius + self.height * 0.5;
        !(point.x > w || point.x < -w || point.y > h || point.y < -h)
    }
}

impl Obstacle for Rectangle {
    /// `PlaneObstacle::findIntersectionWithVehiclePath`, in the rectangle's local space: where
    /// the forward axis crosses the rectangle's plane, if the vehicle heads toward the plane from
    /// a side it is seen from and the crossing is within the rectangle widened by the vehicle's
    /// radius. The hint points back out of the plane and toward the nearer edge (the crossing's
    /// direction from the center).
    fn intersect_path(&self, v: &Vehicle) -> Option<PathIntersection> {
        let lp = self.localize_direction(v.position - self.position);
        let ld = self.localize_direction(v.forward);
        if ld.z == 0.0 {
            return None;
        }
        if (lp.z > 0.0 && ld.z > 0.0) || (lp.z < 0.0 && ld.z < 0.0) {
            return None;
        }
        if (self.seen_from == SeenFrom::Outside && lp.z < 0.0)
            || (self.seen_from == SeenFrom::Inside && lp.z > 0.0)
        {
            return None;
        }
        let ix = lp.x - (ld.x * lp.z / ld.z);
        let iy = lp.y - (ld.y * lp.z / ld.z);
        let crossing = Vec3::new(ix, iy, 0.0);
        if !self.inside_shape(crossing, v.radius) {
            return None;
        }
        let radial = self.globalize_direction(crossing.normalize());
        let side_sign = if lp.z > 0.0 { 1.0 } else { -1.0 };
        let opposing_normal = self.forward * side_sign;
        Some(PathIntersection {
            distance: (lp - crossing).length(),
            surface_point: self.position + self.globalize_direction(crossing),
            steer_hint: opposing_normal + radial,
        })
    }
}

/// The nearest path intersection over all obstacles (`firstPathIntersectionWithObstacleGroup`);
/// on a tie the earlier obstacle wins.
pub fn nearest_intersection<O: Obstacle>(v: &Vehicle, obstacles: &[O]) -> Option<PathIntersection> {
    obstacles
        .iter()
        .filter_map(|o| o.intersect_path(v))
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

/// `steerToAvoidObstacles`: if the nearest intersection is closer than
/// `min_time_to_collision * speed`, a full `max_force` push along the part of the steer hint
/// perpendicular to `forward`; otherwise zero.
pub fn avoid_obstacles<O: Obstacle>(
    v: &Vehicle,
    min_time_to_collision: f32,
    obstacles: &[O],
) -> Vec3 {
    match nearest_intersection(v, obstacles) {
        Some(i) if i.distance < min_time_to_collision * v.speed => {
            i.steer_hint.perpendicular_component(v.forward).normalize() * v.max_force
        }
        _ => Vec3::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Facing +z at speed 1, max force 2.
    fn heading_z(x: f32, y: f32) -> Vehicle {
        Vehicle {
            position: Vec3::new(x, y, 0.0),
            speed: 1.0,
            max_force: 2.0,
            ..Vehicle::default()
        }
    }

    /// A rock of radius 1 at (0, 0, 5): widened by the vehicle radius 0.5, the path meets it 3.5
    /// ahead.
    #[test]
    fn a_rock_ahead_is_hit_at_its_widened_surface() {
        let rock = Sphere::new(Vec3::new(0.0, 0.0, 5.0), 1.0, SeenFrom::Outside);
        let hit = rock.intersect_path(&heading_z(0.0, 0.0)).unwrap();
        assert_eq!(hit.distance, 3.5);
    }

    /// Passing 1 below the rock's center (within the widened radius 1.5): the push is straight
    /// down, away from the rock.
    #[test]
    fn avoidance_pushes_away_in_3d() {
        let rock = Sphere::new(Vec3::new(0.0, 1.0, 5.0), 1.0, SeenFrom::Outside);
        let push = avoid_obstacles(&heading_z(0.0, 0.0), 10.0, &[rock]);
        assert_eq!(push, Vec3::new(0.0, -2.0, 0.0));
    }

    #[test]
    fn a_rock_above_the_path_is_missed() {
        let rock = Sphere::new(Vec3::new(0.0, 2.0, 5.0), 1.0, SeenFrom::Outside);
        assert_eq!(rock.intersect_path(&heading_z(0.0, 0.0)), None);
    }

    /// A 6x2 wall facing +z, centered 4 ahead of the origin.
    fn wall_ahead() -> Rectangle {
        Rectangle::new(
            6.0,
            2.0,
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 4.0),
            SeenFrom::Both,
        )
    }

    /// Heading at the wall's center: the path meets it 4 ahead, but the crossing is the center,
    /// so the hint has no "toward the edge" part and points straight back; its sideways part is
    /// zero, and so is the push.
    #[test]
    fn a_wall_met_dead_center_gives_no_push() {
        let v = heading_z(0.0, 0.0);
        let hit = wall_ahead().intersect_path(&v).unwrap();
        assert_eq!(
            (hit.distance, hit.steer_hint),
            (4.0, Vec3::new(0.0, 0.0, -1.0))
        );
        assert_eq!(avoid_obstacles(&v, 10.0, &[wall_ahead()]), Vec3::ZERO);
    }

    /// 1 to the +x side of the center, the hint is (1, 0, -1): back out of the plane and toward
    /// the nearer edge; the push is its sideways part at full force.
    #[test]
    fn off_center_a_wall_pushes_toward_its_nearer_edge() {
        let v = heading_z(1.0, 0.0);
        let hit = wall_ahead().intersect_path(&v).unwrap();
        assert_eq!(hit.steer_hint, Vec3::new(1.0, 0.0, -1.0));
        assert_eq!(
            avoid_obstacles(&v, 10.0, &[wall_ahead()]),
            Vec3::new(2.0, 0.0, 0.0)
        );
    }

    #[test]
    fn outside_a_container_steers_back_to_its_center() {
        let arena = Sphere::new(Vec3::ZERO, 3.0, SeenFrom::Inside);
        let hit = arena.intersect_path(&heading_z(0.0, 4.0)).unwrap();
        assert_eq!(
            (hit.distance, hit.steer_hint),
            (0.0, Vec3::new(0.0, -1.0, 0.0))
        );
    }
}
