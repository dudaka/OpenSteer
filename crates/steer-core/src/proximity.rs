//! Neighbor queries, the role of OpenSteer's `Proximity.h`: brute force, and a uniform grid of
//! square bins (the idea of `lq.c`'s locality query database). Both return the indices of the
//! points strictly closer than the radius, in increasing order.

use std::collections::HashMap;

use crate::Vec2;

/// Every point closer than `radius` to `center` (`BruteForceProximityDatabase`).
pub fn brute_force(points: &[Vec2], center: Vec2, radius: f32) -> Vec<usize> {
    points
        .iter()
        .enumerate()
        .filter(|(_, p)| (**p - center).length_squared() < radius * radius)
        .map(|(i, _)| i)
        .collect()
}

/// Points binned into square cells of side `cell_size`.
#[derive(Debug, Clone)]
pub struct Grid {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl Grid {
    pub fn new(points: &[Vec2], cell_size: f32) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, &p) in points.iter().enumerate() {
            cells.entry(cell_of(p, cell_size)).or_default().push(i);
        }
        Self { cell_size, cells }
    }

    /// The same answer as `brute_force`, looking only at the cells the query circle touches.
    pub fn neighbors(&self, points: &[Vec2], center: Vec2, radius: f32) -> Vec<usize> {
        let r = Vec2::new(radius, radius);
        let (low, high) = (
            cell_of(center - r, self.cell_size),
            cell_of(center + r, self.cell_size),
        );
        let mut found: Vec<usize> = (low.0..=high.0)
            .flat_map(|x| (low.1..=high.1).map(move |y| (x, y)))
            .filter_map(|cell| self.cells.get(&cell))
            .flatten()
            .copied()
            .filter(|&i| (points[i] - center).length_squared() < radius * radius)
            .collect();
        found.sort_unstable();
        found
    }
}

fn cell_of(p: Vec2, cell_size: f32) -> (i32, i32) {
    (
        (p.x / cell_size).floor() as i32,
        (p.y / cell_size).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed pseudo-random point in `-50..50` (an LCG, no dependency).
    fn scatter(seed: &mut u32) -> Vec2 {
        let mut next = || {
            *seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            (*seed >> 8) as f32 / (1u32 << 24) as f32 * 100.0 - 50.0
        };
        Vec2::new(next(), next())
    }

    #[test]
    fn grid_equals_brute_force() {
        let mut seed = 7;
        let points: Vec<Vec2> = (0..400).map(|_| scatter(&mut seed)).collect();
        let grid = Grid::new(&points, 9.0);
        for radius in [0.5, 3.0, 9.0, 20.0] {
            for _ in 0..50 {
                let center = scatter(&mut seed);
                assert_eq!(
                    grid.neighbors(&points, center, radius),
                    brute_force(&points, center, radius),
                    "center {center:?} radius {radius}"
                );
            }
        }
    }

    #[test]
    fn the_boundary_is_excluded() {
        let points = [Vec2::new(3.0, 0.0), Vec2::new(2.9, 0.0)];
        assert_eq!(brute_force(&points, Vec2::ZERO, 3.0), vec![1]);
        assert_eq!(
            Grid::new(&points, 1.0).neighbors(&points, Vec2::ZERO, 3.0),
            vec![1]
        );
    }
}
