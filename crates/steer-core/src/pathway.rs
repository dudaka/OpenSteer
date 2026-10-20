//! A polyline path with one radius, from OpenSteer's `PolylineSegmentedPathwaySingleRadius`
//! (mapping logic in `QueryPathAlike.h`).

use crate::Vec2;

/// A point mapped onto the path (`PointToPathMapping`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathMapping {
    /// The nearest point on the center line.
    pub on_path: Vec2,
    /// Direction of the segment holding `on_path`.
    pub tangent: Vec2,
    /// Distance outside the path's surface: negative inside the radius.
    pub outside: f32,
    /// Distance along the path to `on_path`.
    pub distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pathway {
    /// The polyline; a cyclic path repeats its first point at the end.
    points: Vec<Vec2>,
    tangents: Vec<Vec2>,
    lengths: Vec<f32>,
    pub radius: f32,
    cyclic: bool,
}

impl Pathway {
    /// A path through `points` (at least two, no two adjacent equal), closed back to the first
    /// point when `cyclic`.
    pub fn new(points: &[Vec2], radius: f32, cyclic: bool) -> Self {
        let mut points = points.to_vec();
        if cyclic {
            points.push(points[0]);
        }
        let (tangents, lengths) = points
            .windows(2)
            .map(|w| {
                let segment = w[1] - w[0];
                let length = segment.length();
                (segment / length, length)
            })
            .unzip();
        Self {
            points,
            tangents,
            lengths,
            radius,
            cyclic,
        }
    }

    /// The polyline's points; a cyclic path ends with its first point again.
    pub fn points(&self) -> &[Vec2] {
        &self.points
    }

    pub fn length(&self) -> f32 {
        self.lengths.iter().sum()
    }

    /// The nearest point on the center line (`mapPointToPath` / `mapPointToPathDistance`). On
    /// a tie the earlier segment wins.
    pub fn map_point(&self, point: Vec2) -> PathMapping {
        let mut best: Option<(f32, PathMapping)> = None;
        let mut distance_before = 0.0;
        for (i, (&tangent, &length)) in self.tangents.iter().zip(&self.lengths).enumerate() {
            let start = self.points[i];
            let along = (point - start).dot(tangent).clamp(0.0, length);
            let on_path = start + tangent * along;
            let outside = (point - on_path).length() - self.radius;
            if best.is_none_or(|(min, _)| outside < min) {
                let mapping = PathMapping {
                    on_path,
                    tangent,
                    outside,
                    distance: distance_before + along,
                };
                best = Some((outside, mapping));
            }
            distance_before += length;
        }
        best.expect("a path has at least one segment").1
    }

    /// The center-line point at `distance` along the path (`mapPathDistanceToPoint`): clamped to
    /// the ends; a cyclic path wraps first with `fmod`, so negative distances clamp to the start.
    pub fn point_at(&self, distance: f32) -> Vec2 {
        let length = self.length();
        let mut remaining = if self.cyclic {
            distance % length
        } else {
            distance
        }
        .clamp(0.0, length);
        let mut i = 0;
        while i < self.lengths.len() - 1 && remaining > self.lengths[i] {
            remaining -= self.lengths[i];
            i += 1;
        }
        self.points[i] + self.tangents[i] * remaining.clamp(0.0, self.lengths[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l_path(cyclic: bool) -> Pathway {
        let points = [Vec2::ZERO, Vec2::new(4.0, 0.0), Vec2::new(4.0, 3.0)];
        Pathway::new(&points, 1.0, cyclic)
    }

    #[test]
    fn lengths_and_closing_segment() {
        assert_eq!(l_path(false).length(), 7.0);
        assert_eq!(l_path(true).length(), 12.0);
    }

    #[test]
    fn maps_a_point_to_the_nearest_segment() {
        let m = l_path(false).map_point(Vec2::new(2.0, -3.0));
        assert_eq!(m.on_path, Vec2::new(2.0, 0.0));
        assert_eq!(m.tangent, Vec2::new(1.0, 0.0));
        assert_eq!((m.outside, m.distance), (2.0, 2.0));
    }

    #[test]
    fn a_corner_tie_keeps_the_earlier_segment() {
        let m = l_path(false).map_point(Vec2::new(5.0, -1.0));
        assert_eq!((m.tangent, m.distance), (Vec2::new(1.0, 0.0), 4.0));
    }

    #[test]
    fn distance_to_point_clamps_and_wraps() {
        let open = l_path(false);
        assert_eq!(open.point_at(5.0), Vec2::new(4.0, 1.0));
        assert_eq!(open.point_at(99.0), Vec2::new(4.0, 3.0));
        let cyclic = l_path(true);
        assert_eq!(cyclic.point_at(13.0), Vec2::new(1.0, 0.0));
        assert_eq!(cyclic.point_at(-1.0), Vec2::ZERO);
    }
}
