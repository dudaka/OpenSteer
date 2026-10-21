//! Walls (added here, not in caravan): the navmesh's blocked edges as segments in one region's
//! frame, as sro-polyanya picks its wall constraints (`navmeshTriangulation.cpp`), simplified:
//! one-way edges (ledges) are walls both ways, an outline edge stitched to another object (0x08)
//! is open (the floor goes on in that object; caravan's overlay draws it as a stitch), and the
//! walls sro-polyanya builds on region borders from closed tiles are not built.

use std::collections::HashMap;

use crate::navmesh::Navmesh;
use crate::region::{edge_flag, sectors};

/// A blocked edge from `a` to `b`: (x, height, z) in the frame's coordinates, the height being
/// the floor's at each end (an object's vertex height, or the terrain's).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wall {
    pub a: [f32; 3],
    pub b: [f32; 3],
}

/// Outline edge bits that make a wall: blocked, a bridge deck's rim (no walking off the deck; a
/// walker under the bridge is far below its height), a siege gate. An outline edge without them
/// (0x00) joins the object to the terrain.
const OUTLINE_WALL: u8 = edge_flag::BLOCKED | edge_flag::BRIDGE | edge_flag::SIEGE;

/// Every wall of the loaded regions and objects in `frame`'s coordinates: the terrain's, then
/// the objects'.
pub fn walls(navmesh: &Navmesh, frame: u16) -> Vec<Wall> {
    let mut walls = terrain_walls(navmesh, frame);
    walls.extend(object_walls(navmesh, frame));
    walls
}

/// The blocked edges inside each loaded region, at the terrain's height. Edges lying on the
/// region's border are skipped, as sro-polyanya does; an edge with an end on the far border
/// (x or z = 1920) has no terrain height there and is left out.
pub fn terrain_walls(navmesh: &Navmesh, frame: u16) -> Vec<Wall> {
    let mut walls = Vec::new();
    let (fx, fz) = sectors(frame);
    for region in navmesh.regions.values() {
        let (rx, rz) = sectors(region.id);
        let (ox, oz) = (((rx - fx) * 1920) as f32, ((rz - fz) * 1920) as f32);
        let end = |[x, z]: [f32; 2]| region.height_at(x, z).map(|h| [x + ox, h, z + oz]);
        for e in &region.internal_edges {
            let on_border =
                |i: usize| e.min[i] == e.max[i] && (e.min[i] == 0.0 || e.min[i] == 1920.0);
            if e.flag & edge_flag::BLOCKED == 0 || on_border(0) || on_border(1) {
                continue;
            }
            if let (Some(a), Some(b)) = (end(e.min), end(e.max)) {
                walls.push(Wall { a, b });
            }
        }
    }
    walls
}

/// Each loaded object's wall outline edges and blocked inline edges, at its vertices' heights.
pub fn object_walls(navmesh: &Navmesh, frame: u16) -> Vec<Wall> {
    let mut walls = Vec::new();
    for &id in navmesh.instances.keys() {
        let Some(object) = navmesh.transformed_object(id, frame) else {
            continue;
        };
        let outline = (object.outline_edges.iter()).filter(|e| e.flag & OUTLINE_WALL != 0);
        let inline = (object.inline_edges.iter()).filter(|e| e.flag & edge_flag::BLOCKED != 0);
        for e in outline.chain(inline) {
            walls.push(Wall {
                a: object.vertices[e.src_vertex as usize],
                b: object.vertices[e.dest_vertex as usize],
            });
        }
    }
    walls
}

/// Two walls whose directions are closer than this (cosine) are in line (5 degrees).
const IN_LINE: f32 = 0.996_195;

/// Joins walls that meet end to end (an exactly shared endpoint) and run in line (within 5
/// degrees, in 3D) into one longer wall, growing each chain from both ends. The navmesh splits a
/// straight wall into many short edges; each would look free at its ends to a rectangle obstacle.
pub fn merge_straight_runs(walls: &[Wall]) -> Vec<Wall> {
    let key = |p: [f32; 3]| p.map(f32::to_bits);
    let mut at: HashMap<[u32; 3], Vec<usize>> = HashMap::new();
    for (i, w) in walls.iter().enumerate() {
        at.entry(key(w.a)).or_default().push(i);
        at.entry(key(w.b)).or_default().push(i);
    }
    let mut used = vec![false; walls.len()];
    let mut merged = Vec::new();
    for start in 0..walls.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let (mut a, mut b) = (walls[start].a, walls[start].b);
        // Grow past `b`, then (swapped) past `a`.
        for _ in 0..2 {
            while let Some((i, far)) = next_in_line(walls, &at, &used, a, b) {
                used[i] = true;
                b = far;
            }
            (a, b) = (b, a);
        }
        merged.push(Wall { a, b });
    }
    merged
}

/// An unused wall touching `b` that continues `a`-`b` in line, and its far end.
fn next_in_line(
    walls: &[Wall],
    at: &HashMap<[u32; 3], Vec<usize>>,
    used: &[bool],
    a: [f32; 3],
    b: [f32; 3],
) -> Option<(usize, [f32; 3])> {
    let direction = |from: [f32; 3], to: [f32; 3]| {
        let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        d.map(|c| c / length)
    };
    let along = direction(a, b);
    at.get(&b.map(f32::to_bits))?.iter().find_map(|&i| {
        let w = walls[i];
        let far = if w.a == b { w.b } else { w.a };
        let next = direction(b, far);
        let cos = along[0] * next[0] + along[1] * next[1] + along[2] * next[2];
        (!used[i] && cos > IN_LINE).then_some((i, far))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::region_id;
    use crate::tests::data_dir;
    use std::collections::HashSet;

    const JANGAN: u16 = 0x61a8;

    fn wall(a: [f32; 3], b: [f32; 3]) -> Wall {
        Wall { a, b }
    }

    /// Three pieces of a straight wall, given in any order and either way round, become one; a
    /// corner and a piece that does not touch stay apart.
    #[test]
    fn straight_runs_merge_and_corners_do_not() {
        let walls = [
            wall([2.0, 0.0, 0.0], [3.0, 0.0, 0.0]),
            wall([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
            wall([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
            wall([3.0, 0.0, 0.0], [3.0, 0.0, 1.0]),
            wall([5.0, 0.0, 0.0], [6.0, 0.0, 0.0]),
        ];
        let merged = merge_straight_runs(&walls);
        assert_eq!(
            merged,
            [
                wall([0.0, 0.0, 0.0], [3.0, 0.0, 0.0]),
                wall([3.0, 0.0, 0.0], [3.0, 0.0, 1.0]),
                wall([5.0, 0.0, 0.0], [6.0, 0.0, 0.0]),
            ]
        );
    }

    /// A bend of 4 degrees merges, one of 6 does not; a climb counts as a bend.
    #[test]
    fn in_line_means_within_5_degrees_in_3d() {
        let bend = |degrees: f32, up: bool| {
            let (s, c) = degrees.to_radians().sin_cos();
            let far = if up {
                [1.0 + c, s, 0.0]
            } else {
                [1.0 + c, 0.0, s]
            };
            merge_straight_runs(&[wall([0.0; 3], [1.0, 0.0, 0.0]), wall([1.0, 0.0, 0.0], far)])
                .len()
        };
        assert_eq!((bend(4.0, false), bend(6.0, false)), (1, 2));
        assert_eq!((bend(4.0, true), bend(6.0, true)), (1, 2));
    }

    /// Jangan alone: of its 23 blocked terrain edges, 15 lie on the region's border (skipped) and
    /// 8 fence its two closed tiles, at (580, 540) and (1400, 980); 762 object walls of its 62
    /// objects: 664 outline 0x03, 21 outline 0x10 (bridge rims), 77 inline 0x07.
    #[test]
    fn jangan_has_8_terrain_walls_and_762_object_walls() {
        let Some(data) = data_dir() else { return };
        let navmesh = Navmesh::load(&data, &HashSet::from([JANGAN])).unwrap();
        assert_eq!(navmesh.instances.len(), 62);
        let terrain = terrain_walls(&navmesh, JANGAN);
        assert_eq!(terrain.len(), 8);
        for w in &terrain {
            let corner = |p: [f32; 3]| [p[0], p[2]];
            let tile = |[x, z]: [f32; 2]| [(x - 580.0) / 20.0, (z - 540.0) / 20.0];
            let near = |[x, z]: [f32; 2]| (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&z);
            let other = |[x, z]: [f32; 2]| near([x - 41.0, z - 22.0]);
            let (a, b) = (tile(corner(w.a)), tile(corner(w.b)));
            assert!((near(a) && near(b)) || (other(a) && other(b)), "{w:?}");
        }
        assert_eq!(object_walls(&navmesh, JANGAN).len(), 762);
    }

    /// The hole in the plaza floor along z = 854 (x 880..1070, found by walking it) is fenced:
    /// exactly two walls cross that line between x 840 and 1120, at x 878.9 and 1072.3, both at
    /// the plaza's height (-32.6). Loaded with the neighbors, in Jangan's frame.
    #[test]
    fn the_plaza_hole_is_fenced_at_the_plaza_height() {
        let Some(data) = data_dir() else { return };
        let (x, z) = sectors(JANGAN);
        let around: HashSet<u16> = (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| region_id(x + dx, z + dz)))
            .collect();
        let navmesh = Navmesh::load(&data, &around).unwrap();
        let mut crossings: Vec<(f32, f32)> = walls(&navmesh, JANGAN)
            .iter()
            .filter(|w| (w.a[2] - 854.0) * (w.b[2] - 854.0) < 0.0)
            .map(|w| {
                let t = (854.0 - w.a[2]) / (w.b[2] - w.a[2]);
                (
                    w.a[0] + t * (w.b[0] - w.a[0]),
                    w.a[1] + t * (w.b[1] - w.a[1]),
                )
            })
            .filter(|&(x, _)| x > 840.0 && x < 1120.0)
            .collect();
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(crossings.len(), 2, "{crossings:?}");
        for ((x, y), want) in crossings.iter().zip([878.9, 1072.3]) {
            assert!(
                (x - want).abs() < 0.05 && (y - -32.6).abs() < 0.05,
                "{crossings:?}"
            );
        }
    }
}
