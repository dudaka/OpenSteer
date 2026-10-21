//! One region's `navmesh\nv_<id>.nvm`: sro-polyanya `NavmeshParser::parseRegion` and its
//! `parseRegion*` steps, `navmesh::Region` with `getHeightAtPoint` and `sanityCheck`.

use crate::{DataDir, Error, Reader};

/// sro-polyanya `navmesh::GlobalEdgeLink`; `linked_obj_global_id` is 0 until matched.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalEdgeLink {
    pub linked_obj_id: i16,
    pub linked_obj_edge_id: i16,
    pub edge_id: i16,
    pub linked_obj_global_id: u32,
}

/// One placed object (sro-polyanya `navmesh::ObjectInstance`). `center` is in its owning
/// region's frame (`region_id`); `yaw` is clockwise.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInstance {
    pub object_id: u32,
    pub center: [f32; 3],
    pub kind: i16,
    pub yaw: f32,
    pub local_uid: u16,
    pub unk: i16,
    pub is_large: u8,
    pub is_structure: u8,
    pub region_id: u16,
    pub global_edge_links: Vec<GlobalEdgeLink>,
}

impl ObjectInstance {
    pub fn global_id(&self) -> u32 {
        (u32::from(self.region_id) << 16) | u32::from(self.local_uid)
    }
}

/// A terrain cell (sro-polyanya `navmesh::Cell`): its rectangle and the internal edges on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub x_min: f32,
    pub z_min: f32,
    pub x_max: f32,
    pub z_max: f32,
    pub edges: Vec<u16>,
}

/// sro-polyanya `navmesh::EdgeFlag` bits.
pub mod edge_flag {
    pub const BLOCK_DST2SRC: u8 = 1;
    pub const BLOCK_SRC2DST: u8 = 2;
    pub const BLOCKED: u8 = BLOCK_DST2SRC | BLOCK_SRC2DST;
    pub const INTERNAL: u8 = 4;
    pub const GLOBAL: u8 = 8;
    pub const BRIDGE: u8 = 16;
    pub const ENTRANCE: u8 = 32;
    pub const BIT6: u8 = 64;
    pub const SIEGE: u8 = 128;
}

/// A terrain edge (sro-polyanya `navmesh::Edge`, x / z only); `assoc_region` only for a global
/// edge (one on the region's border).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub flag: u8,
    pub assoc_direction: [i8; 2],
    pub assoc_cell: [i16; 2],
    pub assoc_region: Option<[i16; 2]>,
}

/// sro-polyanya `navmesh::SurfaceType`: 0 none, 1 water, 2 ice.
pub const SURFACE_ICE: u8 = 2;

/// One region's terrain navmesh (sro-polyanya `navmesh::Region`).
#[derive(Debug, Clone)]
pub struct Region {
    pub id: u16,
    pub object_instance_ids: Vec<u32>,
    pub cell_quads: Vec<Cell>,
    pub global_edges: Vec<Edge>,
    pub internal_edges: Vec<Edge>,
    /// `[row][col]`, z then x; true unless the tile's flag has bit 0.
    pub enabled_tiles: Box<[[bool; 96]; 96]>,
    pub tile_vertex_heights: Box<[[f32; 97]; 97]>,
    pub surface_types: [[u8; 6]; 6],
    pub surface_heights: [[f32; 6]; 6],
}

/// Sector (x, z) of a world region id (sro-polyanya `sectorsFromWorldRegionId`).
pub fn sectors(region_id: u16) -> (i32, i32) {
    (
        i32::from(region_id & 0xFF),
        i32::from((region_id >> 8) & 0xFF),
    )
}

/// The world region id of sectors (sro-polyanya `worldRegionIdFromSectors`).
pub fn region_id(x: i32, z: i32) -> u16 {
    ((x & 0xFF) as u16) | (((z & 0xFF) as u16) << 8)
}

/// The region's file and the object instances it places (with their edge links matched to
/// global ids), as `parseRegion` reads them before the object resources.
pub fn parse_region(data: &DataDir, id: u16) -> Result<(Region, Vec<ObjectInstance>), Error> {
    let name = format!("navmesh\\nv_{id:x}.nvm");
    let bytes = data.read(&name)?;
    let mut r = Reader::new(&name, &bytes);
    r.skip(12)?;
    let instances = parse_object_instances(&mut r, id)?;
    let mut cell_quads = parse_cell_quads(&mut r)?;
    let global_edges = parse_edges(&mut r, true)?;
    let internal_edges = parse_edges(&mut r, false)?;
    for (i, edge) in internal_edges.iter().enumerate() {
        for cell in edge.assoc_cell {
            if cell != -1 {
                let quad = (cell_quads.get_mut(cell as usize))
                    .ok_or_else(|| r.error("edge cell out of range"))?;
                quad.edges.push(i as u16);
            }
        }
    }
    let enabled_tiles = parse_tile_map(&mut r)?;
    let (tile_vertex_heights, surface_types, surface_heights) = parse_height_map(&mut r)?;
    let region = Region {
        id,
        object_instance_ids: instances.iter().map(ObjectInstance::global_id).collect(),
        cell_quads,
        global_edges,
        internal_edges,
        enabled_tiles,
        tile_vertex_heights,
        surface_types,
        surface_heights,
    };
    if !region.sanity_check()? {
        return Err(r.error(
            "Sanity Check Failure. Some assumptions about the region's navmesh didn't hold.",
        ));
    }
    Ok((region, instances))
}

/// `parseRegionObjectInstances`: an instance owned by another region gets its center shifted
/// into that region's frame; edge links get the global id of the instance they name.
fn parse_object_instances(r: &mut Reader, region: u16) -> Result<Vec<ObjectInstance>, Error> {
    let count = r.u16()?;
    let mut instances = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let mut object = ObjectInstance {
            object_id: r.u32()?,
            center: [r.f32()?, r.f32()?, r.f32()?],
            kind: r.i16()?,
            yaw: r.f32()?,
            local_uid: r.u16()?,
            unk: r.i16()?,
            is_large: r.u8()?,
            is_structure: r.u8()?,
            region_id: r.u16()?,
            global_edge_links: Vec::new(),
        };
        if object.region_id != region {
            let (our_x, our_z) = sectors(region);
            let (owner_x, owner_z) = sectors(object.region_id);
            object.center[0] += ((our_x - owner_x) * 1920) as f32;
            object.center[2] += ((our_z - owner_z) * 1920) as f32;
        }
        let links = r.u16()?;
        for _ in 0..links {
            object.global_edge_links.push(GlobalEdgeLink {
                linked_obj_id: r.i16()?,
                linked_obj_edge_id: r.i16()?,
                edge_id: r.i16()?,
                linked_obj_global_id: 0,
            });
        }
        instances.push(object);
    }
    let ids: Vec<u32> = instances.iter().map(ObjectInstance::global_id).collect();
    for object in &mut instances {
        for link in &mut object.global_edge_links {
            if link.linked_obj_id != -1 {
                let id = ids
                    .get(link.linked_obj_id as usize)
                    .ok_or_else(|| r.error("edge link out of range"))?;
                link.linked_obj_global_id = *id;
            }
        }
    }
    Ok(instances)
}

/// `parseRegionCellQuads`: the extra count and each cell's object indices are read and dropped.
fn parse_cell_quads(r: &mut Reader) -> Result<Vec<Cell>, Error> {
    let count = r.u32()?;
    let _extra = r.u32()?;
    let mut cells = Vec::with_capacity(count as usize);
    for _ in 0..count {
        cells.push(Cell {
            x_min: r.f32()?,
            z_min: r.f32()?,
            x_max: r.f32()?,
            z_max: r.f32()?,
            edges: Vec::new(),
        });
        let objects = r.u8()?;
        r.skip(2 * objects as usize)?;
    }
    Ok(cells)
}

/// `parseRegionGlobalEdges` / `parseRegionInternalEdges`.
fn parse_edges(r: &mut Reader, global: bool) -> Result<Vec<Edge>, Error> {
    let count = r.u32()?;
    let mut edges = Vec::with_capacity(count as usize);
    for _ in 0..count {
        edges.push(Edge {
            min: [r.f32()?, r.f32()?],
            max: [r.f32()?, r.f32()?],
            flag: r.u8()?,
            assoc_direction: [r.i8()?, r.i8()?],
            assoc_cell: [r.i16()?, r.i16()?],
            assoc_region: if global {
                Some([r.i16()?, r.i16()?])
            } else {
                None
            },
        });
    }
    Ok(edges)
}

/// `parseRegionTileMap`: cell id, flag and texture id per tile; enabled when flag bit 0 is clear.
fn parse_tile_map(r: &mut Reader) -> Result<Box<[[bool; 96]; 96]>, Error> {
    let mut tiles = Box::new([[false; 96]; 96]);
    for row in tiles.iter_mut() {
        for tile in row.iter_mut() {
            let _cell_id = r.i32()?;
            let flag = r.u16()?;
            let _texture_id = r.u16()?;
            *tile = flag & 1 == 0;
        }
    }
    Ok(tiles)
}

type HeightMaps = (Box<[[f32; 97]; 97]>, [[u8; 6]; 6], [[f32; 6]; 6]);

/// `parseRegionHeightMap`: tile vertex heights, then surface types and heights.
fn parse_height_map(r: &mut Reader) -> Result<HeightMaps, Error> {
    let mut heights = Box::new([[0f32; 97]; 97]);
    for row in heights.iter_mut() {
        for h in row.iter_mut() {
            *h = r.f32()?;
        }
    }
    let mut types = [[0u8; 6]; 6];
    for row in types.iter_mut() {
        for t in row.iter_mut() {
            *t = r.u8()?;
        }
    }
    let mut surface = [[0f32; 6]; 6];
    for row in surface.iter_mut() {
        for h in row.iter_mut() {
            *h = r.f32()?;
        }
    }
    Ok((heights, types, surface))
}

impl Region {
    /// `getHeightAtPoint`: the terrain's bilinear height in a tile, or an ice surface above it.
    /// None out of the region (sro-polyanya throws).
    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        if !(0.0..1920.0).contains(&x) || !(0.0..1920.0).contains(&z) {
            return None;
        }
        let h = &self.tile_vertex_heights;
        let tile_x = (x / 20.0) as usize;
        let tile_z = (z / 20.0) as usize;
        let x_within = x - 20.0 * tile_x as f32;
        let z_within = z - 20.0 * tile_z as f32;
        let x_percent = x_within / 20.0;
        let height_z1 = h[tile_z][tile_x] + x_percent * (h[tile_z][tile_x + 1] - h[tile_z][tile_x]);
        let height_z2 =
            h[tile_z + 1][tile_x] + x_percent * (h[tile_z + 1][tile_x + 1] - h[tile_z + 1][tile_x]);
        let z_percent = z_within / 20.0;
        let mut height = height_z1 + z_percent * (height_z2 - height_z1);
        let (sx, sz) = ((x / 320.0) as usize, (z / 320.0) as usize);
        if self.surface_types[sz][sx] & SURFACE_ICE != 0 {
            height = height.max(self.surface_heights[sz][sx]);
        }
        Some(height)
    }

    /// `sanityCheck`: cells land on tile boundaries and are all blocked or all open; every
    /// global edge names this region first (an error, as sro-polyanya throws).
    pub fn sanity_check(&self) -> Result<bool, Error> {
        for cell in &self.cell_quads {
            if [cell.x_min, cell.z_min, cell.x_max, cell.z_max]
                .iter()
                .any(|v| v % 20.0 != 0.0)
            {
                return Ok(false);
            }
        }
        for cell in &self.cell_quads {
            let mut first: Option<bool> = None;
            let rows = (cell.z_min / 20.0).round() as i32..=(cell.z_max / 20.0).round() as i32 - 1;
            for row in rows {
                let cols =
                    (cell.x_min / 20.0).round() as i32..=(cell.x_max / 20.0).round() as i32 - 1;
                for col in cols {
                    let enabled = self.enabled_tiles[row as usize][col as usize];
                    match first {
                        Some(e) if e != enabled => return Ok(false),
                        _ => first = Some(enabled),
                    }
                }
            }
        }
        if self
            .global_edges
            .iter()
            .any(|e| e.assoc_region.is_some_and(|r| r[0] != self.id as i16))
        {
            return Err(Error::Format {
                path: format!("region {:x}", self.id),
                what: "Found a global edge which does not reference our region in region0!".into(),
            });
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::data_dir;

    const JANGAN: u16 = 0x61a8;

    #[test]
    fn region_ids_and_sectors_round_trip() {
        assert_eq!(sectors(JANGAN), (0xa8, 0x61));
        assert_eq!(region_id(0xa8, 0x61), JANGAN);
    }

    /// Heights as sro-polyanya's `/geometry` sends Jangan's terrain (6 significant digits).
    #[test]
    fn jangan_heights_match_sro_polyanya() {
        let Some(data) = data_dir() else { return };
        let (region, instances) = parse_region(&data, JANGAN).unwrap();
        let h = &region.tile_vertex_heights;
        for (row, col, want) in [
            (0, 0, -2.19405),
            (48, 48, -54.3942),
            (96, 96, 1.37246),
            (42, 42, -51.5249),
            (10, 77, 0.0),
        ] {
            assert!(
                (h[row][col] - want).abs() < 1e-3,
                "[{row}][{col}] {} != {want}",
                h[row][col]
            );
        }
        assert_eq!(instances.len(), 62);
        assert_eq!(region.object_instance_ids.len(), 62);
    }

    /// The terrain under the Jangan plaza, where the server puts characters 21.5 higher (on the
    /// plaza object); same value as the packed map's terrain.
    #[test]
    fn terrain_height_under_the_plaza() {
        let Some(data) = data_dir() else { return };
        let (region, _) = parse_region(&data, JANGAN).unwrap();
        let h = region.height_at(857.893_25, 854.046_9).unwrap();
        assert!((h - -54.09).abs() < 0.01, "{h}");
        assert_eq!(region.height_at(1920.0, 0.0), None);
    }

    /// Every world region file parses and passes the sanity check, as sro-polyanya requires.
    /// Reads all of them: `cargo test -p silkroad-navmesh -- --ignored`.
    #[test]
    #[ignore]
    fn every_region_file_parses() {
        let (Some(data), Some(path)) = (data_dir(), crate::tests::data_path()) else {
            return;
        };
        let mut parsed = 0;
        for entry in std::fs::read_dir(path.join("navmesh")).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            let Some(hex) = name
                .strip_prefix("nv_")
                .and_then(|n| n.strip_suffix(".nvm"))
            else {
                continue;
            };
            parse_region(&data, u16::from_str_radix(hex, 16).unwrap()).unwrap();
            parsed += 1;
        }
        eprintln!("{parsed} region files parsed");
        assert!(parsed > 4000);
    }
}
