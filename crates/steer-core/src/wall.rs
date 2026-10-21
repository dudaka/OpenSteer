//! Walls for wall following and containment (Reynolds 1999, not in OpenSteer): a closed
//! polygon, its points counter-clockwise in `(x, y)` so each edge's outward normal is
//! `(t.y, -t.x)` for edge direction `t`.

use crate::pathway::Pathway;
use crate::Vec2;

#[derive(Debug, Clone, PartialEq)]
pub struct Wall {
    points: Vec<Vec2>,
}

impl Wall {
    /// A closed wall through `points` (at least three, counter-clockwise, no two adjacent
    /// equal).
    pub fn new(points: &[Vec2]) -> Self {
        Self {
            points: points.to_vec(),
        }
    }

    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    /// The edges as (start, end) pairs, the last one closing back to the first point.
    pub fn edges(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        let n = self.points.len();
        (0..n).map(move |i| (self.points[i], self.points[(i + 1) % n]))
    }

    /// The outline `distance` outside the wall (inside when negative): each corner moved along
    /// the miter of its two edges' normals, so every edge stays exactly `distance` away.
    pub fn offset(&self, distance: f32) -> Vec<Vec2> {
        let n = self.points.len();
        let normal = |i: usize| {
            let t = (self.points[(i + 1) % n] - self.points[i]).normalize();
            Vec2::new(t.y, -t.x)
        };
        (0..n)
            .map(|i| {
                let (before, after) = (normal((i + n - 1) % n), normal(i));
                self.points[i] + (before + after) * (distance / (1.0 + before.dot(after)))
            })
            .collect()
    }

    /// Distance from `point` to the nearest point of the wall.
    pub fn distance(&self, point: Vec2) -> f32 {
        self.edges()
            .map(|(a, b)| {
                let t = (b - a).normalize();
                let along = (point - a).dot(t).clamp(0.0, (b - a).length());
                (point - (a + t * along)).length()
            })
            .fold(f32::INFINITY, f32::min)
    }
}

/// The cyclic path `offset` outside `wall`, with `radius`, for `follow_path` with direction +1:
/// counter-clockwise, or clockwise (the outline reversed).
pub fn wall_path(wall: &Wall, offset: f32, radius: f32, clockwise: bool) -> Pathway {
    let mut points = wall.offset(offset);
    if clockwise {
        points.reverse();
    }
    Pathway::new(&points, radius, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An L: a 12 x 5 bar with a 5 x 5 block on its left end; (5, 5) is the inner corner.
    fn l_shape() -> Wall {
        Wall::new(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(12.0, 0.0),
            Vec2::new(12.0, 5.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(5.0, 10.0),
            Vec2::new(0.0, 10.0),
        ])
    }

    #[test]
    fn offset_moves_outer_corners_out_and_the_inner_corner_in() {
        let out = l_shape().offset(1.0);
        assert_eq!(out[0], Vec2::new(-1.0, -1.0));
        assert_eq!(out[2], Vec2::new(13.0, 6.0));
        // inner corner: both neighboring edges move 1 out, they meet at (6, 6)
        assert_eq!(out[3], Vec2::new(6.0, 6.0));
        assert_eq!(l_shape().offset(-1.0)[0], Vec2::new(1.0, 1.0));
    }

    /// Offset edges stay exactly the offset away; an outer right-angle corner is offset x
    /// sqrt(2) from the wall's corner.
    #[test]
    fn offset_edges_are_that_far_from_the_wall() {
        let wall = l_shape();
        let out = Wall::new(&wall.offset(2.0));
        for (a, b) in out.edges() {
            let middle = (a + b) * 0.5;
            assert!((wall.distance(middle) - 2.0).abs() < 1e-5, "{middle:?}");
        }
        assert_eq!(wall.distance(out.points()[0]), 8.0_f32.sqrt());
    }

    #[test]
    fn distance_to_an_edge_or_a_corner() {
        let wall = l_shape();
        assert_eq!(wall.distance(Vec2::new(6.0, -3.0)), 3.0);
        assert_eq!(wall.distance(Vec2::new(15.0, 9.0)), 5.0);
        assert_eq!(wall.distance(Vec2::new(4.0, 7.0)), 1.0);
    }

    #[test]
    fn clockwise_path_is_the_outline_reversed() {
        let wall = l_shape();
        let ccw = wall_path(&wall, 1.0, 0.5, false);
        let cw = wall_path(&wall, 1.0, 0.5, true);
        assert_eq!(ccw.points()[1], Vec2::new(13.0, -1.0));
        assert_eq!(cw.points()[0], Vec2::new(-1.0, 11.0));
        assert_eq!(cw.points()[1], Vec2::new(6.0, 11.0));
        assert_eq!(ccw.length(), cw.length());
    }
}
