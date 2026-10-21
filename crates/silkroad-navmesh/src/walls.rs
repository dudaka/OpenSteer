//! Walls (added here, not in caravan): the navmesh's blocked edges as segments in one region's
//! frame, as sro-polyanya picks its wall constraints (`navmeshTriangulation.cpp`), simplified:
//! one-way edges (ledges) are walls both ways, an outline edge stitched to another object (0x08)
//! is open (the floor goes on in that object; caravan's overlay draws it as a stitch), and the
//! walls sro-polyanya builds on region borders from closed tiles are not built.

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::region_id;
    use crate::tests::data_dir;
    use std::collections::HashSet;

    const JANGAN: u16 = 0x61a8;

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
