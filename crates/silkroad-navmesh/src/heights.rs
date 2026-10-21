//! Heights along a walk: sro-polyanya `path.cpp` `surfaceHeightsAtPoint` (every surface stacked
//! over a point) and `reconstructHeights` (one surface per point, the least squared climbing,
//! anchored at the start and goal heights), over a straight leg densified as its paths are.

use std::collections::BTreeSet;

use crate::navmesh::Navmesh;
use crate::region::{region_id, sectors};

/// Units between height samples along a walk (sro-polyanya `kMaxStep`; a terrain tile is 20).
const MAX_STEP: f64 = 10.0;
/// Heights within this are the same floor when bridging a hole (`kBridgeTol`).
const BRIDGE_TOL: f32 = 40.0;
/// Only holes up to this long are bridged, about 3 tiles (`kMaxBridgeDist`).
const MAX_BRIDGE_DIST: f64 = 60.0;

/// `surfaceHeightsAtPoint`: the terrain where its tile is walkable, then each object floor (an
/// area of an object instance) over (x, z), given in `frame`'s coordinates (may lie past its
/// edges). Empty when the region the point falls in is not loaded.
pub fn surface_heights(navmesh: &Navmesh, frame: u16, x: f32, z: f32) -> Vec<f32> {
    let (fx, fz) = sectors(frame);
    let (ox, oz) = (
        (f64::from(x) / 1920.0).floor() as i32,
        (f64::from(z) / 1920.0).floor() as i32,
    );
    let id = region_id(fx + ox, fz + oz);
    let (x, z) = (x - 1920.0 * ox as f32, z - 1920.0 * oz as f32);
    let Some(region) = navmesh.regions.get(&id) else {
        return Vec::new();
    };
    let mut heights = Vec::new();
    let col = ((x / 20.0) as i32).clamp(0, 95) as usize;
    let row = ((z / 20.0) as i32).clamp(0, 95) as usize;
    if region.enabled_tiles[row][col] {
        heights.extend(region.height_at(x, z));
    }
    for &instance in &region.object_instance_ids {
        let Some(object) = navmesh.transformed_object(instance, id) else {
            continue;
        };
        let areas: BTreeSet<u32> = object.cell_area_ids.iter().copied().collect();
        heights.extend(
            areas
                .into_iter()
                .filter_map(|area| object.height(x, z, area)),
        );
    }
    heights
}

/// The heights of a straight walk from `start` to `goal` (x, z in `frame`'s coordinates), as
/// (distance from the start, height) every [`MAX_STEP`] units or less, from `start_y` to the
/// surface nearest `goal_y` (None: no goal height, the walk's cheapest end).
pub fn walk_heights(
    navmesh: &Navmesh,
    frame: u16,
    start: (f32, f32),
    goal: (f32, f32),
    start_y: f32,
    goal_y: Option<f32>,
) -> Vec<(f32, f32)> {
    let (dx, dz) = (f64::from(goal.0 - start.0), f64::from(goal.1 - start.1));
    let length = (dx * dx + dz * dz).sqrt();
    let steps = ((length / MAX_STEP).ceil() as usize).max(1);
    let points: Vec<(f32, f32)> = (0..=steps)
        .map(|s| {
            let t = s as f64 / steps as f64;
            (
                (f64::from(start.0) + dx * t) as f32,
                (f64::from(start.1) + dz * t) as f32,
            )
        })
        .collect();
    let candidates = points
        .iter()
        .map(|&(x, z)| surface_heights(navmesh, frame, x, z))
        .collect();
    let heights = reconstruct_heights(&points, candidates, start_y, goal_y);
    (0..=steps)
        .map(|s| ((length * s as f64 / steps as f64) as f32, heights[s]))
        .collect()
}

/// `reconstructHeights` over each point's candidate surfaces: a point with none (its region not
/// loaded) takes the goal height at the end, else the start height; short holes in a floor are
/// bridged; then the choice with the least sum of squared steps, anchored at both heights.
pub fn reconstruct_heights(
    points: &[(f32, f32)],
    mut candidates: Vec<Vec<f32>>,
    start_y: f32,
    goal_y: Option<f32>,
) -> Vec<f32> {
    let step_cost = |a: f32, b: f32| (a - b) * (a - b);
    let n = points.len();
    for (i, c) in candidates.iter_mut().enumerate() {
        if c.is_empty() {
            let is_goal = i + 1 == n;
            c.push(if is_goal {
                goal_y.unwrap_or(start_y)
            } else {
                start_y
            });
        }
    }

    let mut cum = vec![0.0f64; n];
    for i in 1..n {
        let dx = f64::from(points[i].0 - points[i - 1].0);
        let dz = f64::from(points[i].1 - points[i - 1].1);
        cum[i] = cum[i - 1] + (dx * dx + dz * dz).sqrt();
    }
    let orig = candidates.clone();
    for a in 0..n.saturating_sub(1) {
        for &h in &orig[a] {
            let mut b = a + 1;
            while b < n && cum[b] - cum[a] <= MAX_BRIDGE_DIST {
                if let Some(&hb) = orig[b].iter().find(|&&c| (c - h).abs() <= BRIDGE_TOL) {
                    for m in a + 1..b {
                        let t = (cum[m] - cum[a]) / (cum[b] - cum[a]);
                        candidates[m].push((f64::from(h) * (1.0 - t) + f64::from(hb) * t) as f32);
                    }
                    break;
                }
                b += 1;
            }
        }
    }

    let mut cost: Vec<Vec<f32>> = candidates.iter().map(|c| vec![f32::MAX; c.len()]).collect();
    let mut back: Vec<Vec<usize>> = candidates.iter().map(|c| vec![0; c.len()]).collect();
    for (j, &c) in candidates[0].iter().enumerate() {
        cost[0][j] = step_cost(c, start_y);
    }
    for i in 1..n {
        for j in 0..candidates[i].len() {
            for k in 0..candidates[i - 1].len() {
                let c = cost[i - 1][k] + step_cost(candidates[i][j], candidates[i - 1][k]);
                if c < cost[i][j] {
                    cost[i][j] = c;
                    back[i][j] = k;
                }
            }
        }
    }
    let (mut chosen, mut best) = (0, f32::MAX);
    for (j, &c) in candidates[n - 1].iter().enumerate() {
        let total = cost[n - 1][j] + goal_y.map_or(0.0, |g| step_cost(c, g));
        if total < best {
            (best, chosen) = (total, j);
        }
    }
    let mut heights = vec![0.0; n];
    for i in (0..n).rev() {
        heights[i] = candidates[i][chosen];
        if i > 0 {
            chosen = back[i][chosen];
        }
    }
    heights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::data_dir;
    use std::collections::HashSet;

    fn line(n: usize) -> Vec<(f32, f32)> {
        (0..n).map(|i| (i as f32 * 10.0, 0.0)).collect()
    }

    /// Terrain at 0 everywhere and a ramp up to the goal's 30: the walk takes the ramp's small
    /// steps (3 x 10^2) over one jump at the end (30^2). A single jump costs the same anywhere,
    /// and a tie keeps the first candidate, as `<` does there.
    #[test]
    fn the_walk_spreads_a_climb_over_small_steps() {
        let candidates = vec![vec![0.0], vec![0.0, 10.0], vec![0.0, 20.0], vec![0.0, 30.0]];
        let h = reconstruct_heights(&line(4), candidates, 0.0, Some(30.0));
        assert_eq!(h, [0.0, 10.0, 20.0, 30.0]);
    }

    /// A one-sample hole in a deck at 50 over terrain at -100 is bridged: the walk stays on it.
    #[test]
    fn a_short_hole_in_a_floor_is_bridged() {
        let candidates = vec![
            vec![-100.0, 50.0],
            vec![-100.0, 50.0],
            vec![-100.0],
            vec![-100.0, 50.0],
            vec![-100.0, 50.0],
        ];
        let h = reconstruct_heights(&line(5), candidates, 50.0, Some(50.0));
        assert_eq!(h, [50.0; 5]);
    }

    /// Points with no surface (their region not loaded) take the start height, the last the goal's.
    #[test]
    fn points_outside_the_loaded_regions_take_the_anchors() {
        let h = reconstruct_heights(&line(3), vec![vec![], vec![], vec![]], 5.0, Some(9.0));
        assert_eq!(h, [5.0, 5.0, 9.0]);
    }

    /// A walk across the Jangan plaza between two recorded positions stays on its floor (-32.6),
    /// over terrain from -42.5 to -33.2; the floor covers the whole line.
    #[test]
    fn a_walk_across_the_jangan_plaza_stays_on_its_floor() {
        let Some(data) = data_dir() else { return };
        let (x, z) = sectors(0x61a8);
        let around: HashSet<u16> = (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| region_id(x + dx, z + dz)))
            .collect();
        let navmesh = Navmesh::load(&data, &around).unwrap();
        let walk = walk_heights(
            &navmesh,
            0x61a8,
            (970.0, 1040.0),
            (1161.0, 1175.0),
            -32.6,
            Some(-32.6),
        );
        assert!(walk.len() > 20);
        assert!(
            walk.iter().all(|&(_, h)| (h - -32.6).abs() < 0.05),
            "{walk:?}"
        );
    }
}
