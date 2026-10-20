//! Circle obstacles and obstacle avoidance, from OpenSteer's `Obstacle.cpp` (`SphereObstacle`
//! on the ground plane).

use crate::{Vec2, Vehicle};

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
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
    pub seen_from: SeenFrom,
}

/// Where a vehicle's forward path first meets an obstacle (`PathIntersection`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathIntersection {
    /// Distance along `forward` to the surface point.
    pub distance: f32,
    pub surface_point: Vec2,
    /// The direction to steer toward, away from the surface.
    pub steer_hint: Vec2,
}

impl Circle {
    pub const fn new(center: Vec2, radius: f32, seen_from: SeenFrom) -> Self {
        Self {
            center,
            radius,
            seen_from,
        }
    }

    /// `findIntersectionWithVehiclePath`: the nearest point ahead where the vehicle's forward
    /// line, widened by its radius, meets the circle. `None` when the line misses or both
    /// meeting points are behind.
    pub fn intersect_path(&self, v: &Vehicle) -> Option<PathIntersection> {
        let offset = self.center - v.position;
        let vehicle_outside = offset.length() > self.radius;
        if vehicle_outside && self.seen_from == SeenFrom::Inside {
            return Some(PathIntersection {
                distance: 0.0,
                surface_point: v.position,
                steer_hint: offset.normalize(),
            });
        }
        // The line is the forward axis; `local_z` is the center's forward coordinate.
        let local_z = offset.dot(v.forward);
        let r = self.radius + v.radius;
        let b = -2.0 * local_z;
        let c = offset.length_squared() - r * r;
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

/// The nearest path intersection over all obstacles (`firstPathIntersectionWithObstacleGroup`).
pub fn nearest_intersection(v: &Vehicle, obstacles: &[Circle]) -> Option<PathIntersection> {
    obstacles
        .iter()
        .filter_map(|o| o.intersect_path(v))
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
}

/// `steerToAvoidObstacles`: if the nearest intersection is closer than
/// `min_time_to_collision * speed`, a full `max_force` sideways push toward the steer hint;
/// otherwise zero.
pub fn avoid_obstacles(v: &Vehicle, min_time_to_collision: f32, obstacles: &[Circle]) -> Vec2 {
    match nearest_intersection(v, obstacles) {
        Some(i) if i.distance < min_time_to_collision * v.speed => {
            i.steer_hint.perpendicular_component(v.forward).normalize() * v.max_force
        }
        _ => Vec2::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading_north(x: f32, speed: f32) -> Vehicle {
        Vehicle {
            position: Vec2::new(x, 0.0),
            speed,
            max_force: 2.0,
            ..Vehicle::default()
        }
    }

    const ROCK: Circle = Circle::new(Vec2::new(0.0, 5.0), 1.0, SeenFrom::Outside);

    #[test]
    fn path_meets_the_near_side_widened_by_vehicle_radius() {
        let i = ROCK.intersect_path(&heading_north(0.0, 1.0)).unwrap();
        // radius 1 + vehicle 0.5: surface at y = 3.5
        assert_eq!(i.distance, 3.5);
        assert_eq!(i.steer_hint, Vec2::new(0.0, -1.0));
    }

    #[test]
    fn misses_and_obstacles_behind_are_ignored() {
        assert_eq!(ROCK.intersect_path(&heading_north(2.0, 1.0)), None);
        let behind = Circle::new(Vec2::new(0.0, -5.0), 1.0, SeenFrom::Outside);
        assert_eq!(behind.intersect_path(&heading_north(0.0, 1.0)), None);
    }

    #[test]
    fn avoidance_is_sideways_at_max_force_only_when_close() {
        // Slightly right of center: the hint points right, push right.
        let v = heading_north(0.5, 1.0);
        assert_eq!(avoid_obstacles(&v, 1.0, &[ROCK]), Vec2::ZERO);
        let push = avoid_obstacles(&v, 5.0, &[ROCK]);
        assert!((push - Vec2::new(2.0, 0.0)).length() < 1e-6, "{push:?}");
    }

    #[test]
    fn a_container_pulls_back_a_vehicle_outside_it() {
        let arena = Circle::new(Vec2::ZERO, 3.0, SeenFrom::Inside);
        let v = heading_north(5.0, 1.0);
        let i = arena.intersect_path(&v).unwrap();
        assert_eq!((i.distance, i.steer_hint), (0.0, Vec2::new(-1.0, 0.0)));
    }

    #[test]
    fn inside_a_solid_obstacle_the_surface_is_here() {
        let mut rock = Circle::new(Vec2::new(0.0, 1.0), 2.0, SeenFrom::Outside);
        let v = heading_north(0.0, 1.0);
        assert_eq!(rock.intersect_path(&v).unwrap().distance, 0.0);
        // A two-sided obstacle takes the exit ahead: 1 + 2 + 0.5.
        rock.seen_from = SeenFrom::Both;
        assert_eq!(rock.intersect_path(&v).unwrap().distance, 3.5);
    }

    #[test]
    fn nearest_of_several_wins() {
        let far = Circle::new(Vec2::new(0.0, 9.0), 1.0, SeenFrom::Outside);
        let i = nearest_intersection(&heading_north(0.0, 1.0), &[far, ROCK]).unwrap();
        assert_eq!(i.distance, 3.5);
    }
}
