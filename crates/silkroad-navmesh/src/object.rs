//! Object navmeshes: sro-polyanya `NavmeshParser::buildObjectFileInfoMap`,
//! `parseObjectResource` (`.bsr`), `parseCompoundResource` (`.cpd`), `parseObjectBms`, and
//! `Navmesh::postProcess` step 1 (each object's cells split into connected areas).

use std::collections::HashMap;

use crate::{DataDir, Error, Reader};

/// sro-polyanya `navmesh::PrimMeshNavCell`: a triangle of the object's navmesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavCell {
    pub vertex: [u16; 3],
    pub event_zone_data: Option<u8>,
}

/// sro-polyanya `navmesh::PrimMeshNavEdge`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavEdge {
    pub src_vertex: i16,
    pub dest_vertex: i16,
    pub src_cell: i16,
    pub dest_cell: i16,
    pub flag: u8,
    pub event_zone_data: Option<u8>,
}

/// An object's navmesh in its own frame (sro-polyanya `navmesh::ObjectResource`): vertices
/// (x, height, z), cells, outline edges (its border) and inline edges (between its cells).
/// `cell_area_ids` numbers each cell's connected area (`postProcess`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectResource {
    pub name: String,
    pub vertices: Vec<[f32; 3]>,
    pub cells: Vec<NavCell>,
    pub cell_area_ids: Vec<u32>,
    pub outline_edges: Vec<NavEdge>,
    pub inline_edges: Vec<NavEdge>,
}

impl ObjectResource {
    /// `getHeight`: the height at (x, z) on a cell of `area`, None when the point is not on it
    /// (sro-polyanya throws).
    pub fn height(&self, x: f32, z: f32, area: u32) -> Option<f32> {
        (self.cells.iter().zip(&self.cell_area_ids))
            .filter(|(_, &a)| a == area)
            .find_map(|(cell, _)| {
                let [v0, v1, v2] = cell.vertex.map(|i| self.vertices[i as usize]);
                height_in_triangle((x, z), v0, v1, v2)
            })
    }
}

/// `heightInTriangle`: barycentric weights over x / z, None outside.
fn height_in_triangle((x, z): (f32, f32), v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> Option<f32> {
    let denom = (v1[2] - v2[2]) * (v0[0] - v2[0]) + (v2[0] - v1[0]) * (v0[2] - v2[2]);
    let w0 = ((v1[2] - v2[2]) * (x - v2[0]) + (v2[0] - v1[0]) * (z - v2[2])) / denom;
    let w1 = ((v2[2] - v0[2]) * (x - v2[0]) + (v0[0] - v2[0]) * (z - v2[2])) / denom;
    let w2 = 1.0 - w0 - w1;
    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
        return None;
    }
    Some(w0 * v0[1] + w1 * v1[1] + w2 * v2[1])
}

/// `navmesh\object.ifo`: each object id's resource path (`buildObjectFileInfoMap`; the flag is
/// read and not used).
pub fn object_paths(data: &DataDir) -> Result<HashMap<u32, String>, Error> {
    let name = "navmesh\\object.ifo";
    let text = String::from_utf8_lossy(&data.read(name)?).into_owned();
    let mut lines = text.lines();
    let bad = |what: &str| Error::Format {
        path: name.into(),
        what: what.into(),
    };
    lines.next().ok_or_else(|| bad("Unable to parse header"))?;
    lines.next().ok_or_else(|| bad("no entry count"))?;
    let mut paths = HashMap::new();
    for line in lines {
        // `00000 0x00000001 "res\bldg\...\x.bsr"`
        let mut parts = line.splitn(3, ' ');
        let (Some(index), Some(_flag), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let index = index.trim().parse().map_err(|_| bad("bad index"))?;
        paths.insert(index, path.trim_end().trim_matches('"').to_string());
    }
    Ok(paths)
}

/// `parseObjectResource`: a `.cpd` points at its collision resource, a `.bsr` at its root mesh,
/// whose `.bms` holds the navmesh. The name is the `.bsr`'s own.
pub fn parse_object_resource(data: &DataDir, path: &str) -> Result<ObjectResource, Error> {
    if path.ends_with(".cpd") {
        return parse_compound_resource(data, path);
    }
    if !path.ends_with(".bsr") {
        return Err(Error::Format {
            path: path.into(),
            what: "Unable to determine filetype".into(),
        });
    }
    let bytes = data.read(path)?;
    let mut r = Reader::new(path, &bytes);
    r.skip(12)?;
    r.skip(8 * 4)?; // material, mesh, skeleton, animation, mesh group, animation group, sound, bounding box
    r.skip(5 * 4)?;
    let _type = r.u32()?;
    let name = r.string()?;
    r.skip(48)?;
    let root_mesh = r.string()?;
    let mut object = parse_object_bms(data, &root_mesh)?;
    object.name = name;
    Ok(object)
}

/// `parseCompoundResource`.
fn parse_compound_resource(data: &DataDir, path: &str) -> Result<ObjectResource, Error> {
    let bytes = data.read(path)?;
    let mut r = Reader::new(path, &bytes);
    r.skip(12)?;
    let collision = r.u32()?;
    r.seek(collision as usize)?;
    let collision_path = r.string()?;
    parse_object_resource(data, &collision_path)
}

/// `parseObjectBms`: the navmesh section. A nonzero cell flag fails the resource, as there.
fn parse_object_bms(data: &DataDir, path: &str) -> Result<ObjectResource, Error> {
    let bytes = data.read(path)?;
    let mut r = Reader::new(path, &bytes);
    r.skip(12)?;
    let mut offsets = [0u32; 12];
    for o in &mut offsets {
        *o = r.u32()?;
    }
    let (nav_mesh_offset, nav_flag) = (offsets[7], offsets[11]);
    if nav_mesh_offset == 0 {
        return Err(r.error("Wait, no navmesh?"));
    }
    r.seek(nav_mesh_offset as usize)?;
    let mut object = ObjectResource::default();
    for _ in 0..r.u32()? {
        object.vertices.push([r.f32()?, r.f32()?, r.f32()?]);
        let _angle_index = r.u8()?;
    }
    for _ in 0..r.u32()? {
        let vertex = [r.u16()?, r.u16()?, r.u16()?];
        if r.u16()? != 0 {
            return Err(r.error("Whoa! Nonzero flag"));
        }
        let event_zone_data = if nav_flag & 2 != 0 {
            r.u8().map(nonzero)?
        } else {
            None
        };
        object.cells.push(NavCell {
            vertex,
            event_zone_data,
        });
    }
    object.outline_edges = parse_edges(&mut r, nav_flag)?;
    object.inline_edges = parse_edges(&mut r, nav_flag)?;
    Ok(object)
}

fn nonzero(v: u8) -> Option<u8> {
    (v != 0).then_some(v)
}

fn parse_edges(r: &mut Reader, nav_flag: u32) -> Result<Vec<NavEdge>, Error> {
    let count = r.u32()?;
    let mut edges = Vec::with_capacity(count as usize);
    for _ in 0..count {
        edges.push(NavEdge {
            src_vertex: r.i16()?,
            dest_vertex: r.i16()?,
            src_cell: r.i16()?,
            dest_cell: r.i16()?,
            flag: r.u8()?,
            event_zone_data: if nav_flag & 1 != 0 {
                r.u8().map(nonzero)?
            } else {
                None
            },
        });
    }
    Ok(edges)
}

/// `postProcess` step 1: number the connected areas of an object's cells, walking inline edges
/// from each unvisited cell in index order.
pub fn set_cell_areas(object: &mut ObjectResource) {
    let n = object.cells.len();
    object.cell_area_ids = vec![0; n];
    let mut visited = vec![false; n];
    let mut area = 0;
    for start in 0..n {
        if visited[start] {
            continue;
        }
        let mut stack = vec![start];
        while let Some(cell) = stack.pop() {
            visited[cell] = true;
            object.cell_area_ids[cell] = area;
            for edge in &object.inline_edges {
                let neighbour = if edge.src_cell as usize == cell {
                    edge.dest_cell
                } else if edge.dest_cell as usize == cell {
                    edge.src_cell
                } else {
                    continue;
                };
                if neighbour != -1 && !visited[neighbour as usize] {
                    stack.push(neighbour as usize);
                }
            }
        }
        area += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::parse_region;
    use crate::tests::data_dir;

    /// Jangan's own objects, as sro-polyanya's `/geometry` sends them (instance ids
    /// 0x61a8xxxx): 57 objects, 674 cells, 570 outline edges, 75 internal blocking edges
    /// (internal edges with a block bit), 66 areas; the first, 0x61a82001, 38 / 24 / 12 in one area.
    #[test]
    fn jangan_objects_match_sro_polyanya() {
        let Some(data) = data_dir() else { return };
        let paths = object_paths(&data).unwrap();
        let (_, instances) = parse_region(&data, 0x61a8).unwrap();
        let own: Vec<_> = instances.iter().filter(|i| i.region_id == 0x61a8).collect();
        assert_eq!(own.len(), 57);
        let mut totals = [0usize; 4];
        for instance in &own {
            let mut o = parse_object_resource(&data, &paths[&instance.object_id]).unwrap();
            set_cell_areas(&mut o);
            let blocking = (o.inline_edges.iter())
                .filter(|e| e.flag & 0x04 != 0 && e.flag & 0x03 != 0)
                .count();
            let areas = o.cell_area_ids.iter().max().map_or(0, |m| m + 1) as usize;
            if instance.global_id() == 0x61a8_2001 {
                assert_eq!(
                    (o.cells.len(), o.outline_edges.len(), blocking, areas),
                    (38, 24, 12, 1)
                );
            }
            for (t, v) in
                totals
                    .iter_mut()
                    .zip([o.cells.len(), o.outline_edges.len(), blocking, areas])
            {
                *t += v;
            }
        }
        assert_eq!(totals, [674, 570, 75, 66]);
    }

    /// Every object in object.ifo: how many load (sro-polyanya prints and skips a failure) and
    /// whether an inline edge ever lacks a cell, which `postProcess` would follow as cell 65535
    /// (its neighbour index is a uint16_t, so its `!= -1` test never fails). Reads all of them:
    /// `cargo test -p silkroad-navmesh -- --ignored`.
    #[test]
    #[ignore]
    fn every_object_resource() {
        let Some(data) = data_dir() else { return };
        let paths = object_paths(&data).unwrap();
        let (mut ok, mut failed, mut open_inline) = (0, Vec::new(), 0);
        for (id, path) in &paths {
            match parse_object_resource(&data, path) {
                Ok(o) => {
                    ok += 1;
                    open_inline += (o.inline_edges.iter())
                        .filter(|e| e.src_cell == -1 || e.dest_cell == -1)
                        .count();
                }
                Err(e) => failed.push(format!("{id}: {e}")),
            }
        }
        failed.sort();
        eprintln!(
            "{ok} loaded, {} failed, {open_inline} inline edges without a cell",
            failed.len()
        );
        for f in failed.iter().take(15) {
            eprintln!("  {f}");
        }
    }
}
