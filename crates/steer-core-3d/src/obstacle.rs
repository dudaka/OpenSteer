//! Sphere obstacles and obstacle avoidance, from OpenSteer's `Obstacle.cpp` (`SphereObstacle`).

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

/// The nearest path intersection over all obstacles (`firstPathIntersectionWithObstacleGroup`);
/// on a tie the earlier obstacle wins.
pub fn nearest_intersection(v: &Vehicle, obstacles: &[Sphere]) -> Option<PathIntersection> {
    obstacles
        .iter()
        .filter_map(|o| o.intersect_path(v))
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

/// `steerToAvoidObstacles`: if the nearest intersection is closer than
/// `min_time_to_collision * speed`, a full `max_force` push along the part of the steer hint
/// perpendicular to `forward`; otherwise zero.
pub fn avoid_obstacles(v: &Vehicle, min_time_to_collision: f32, obstacles: &[Sphere]) -> Vec3 {
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
