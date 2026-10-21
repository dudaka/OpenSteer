//! A navmesh over a set of regions: sro-polyanya `NavmeshParser::parseNavmesh` with a region
//! allow-list (as its engine loads a zone), `parseMapInfo`, `parseRegionObjectResources`, and
//! `navmesh::Navmesh` (`addObjectInstance`, `sanityCheck`, `postProcess`, the object transforms).

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::object::{object_paths, parse_object_resource, set_cell_areas, ObjectResource};
use crate::region::{edge_flag, parse_region, region_id, sectors, ObjectInstance, Region};
use crate::{DataDir, Error, Reader};

/// `navmesh\mapinfo.mfo`: the world's size in regions and which regions are open.
pub struct MapInfo {
    pub width: u16,
    pub height: u16,
    region_data: Vec<u8>,
}

impl MapInfo {
    /// `parseMapInfo`.
    pub fn read(data: &DataDir) -> Result<Self, Error> {
        let name = "navmesh\\mapinfo.mfo";
        let bytes = data.read(name)?;
        let mut r = Reader::new(name, &bytes);
        r.skip(12)?;
        let (width, height) = (r.u16()?, r.u16()?);
        r.skip(8)?;
        let region_data = r.bytes(8192)?.to_vec();
        Ok(MapInfo {
            width,
            height,
            region_data,
        })
    }

    /// The region's bit in the map's bitmap.
    pub fn is_enabled(&self, id: u16) -> bool {
        self.region_data[(id >> 3) as usize] & (128u8 >> (id % 8)) != 0
    }
}

/// The regions loaded and their objects (sro-polyanya `navmesh::Navmesh`). Objects whose
/// resource fails to load are left out, as there (it prints and goes on).
#[derive(Debug, Default)]
pub struct Navmesh {
    pub regions: BTreeMap<u16, Region>,
    pub resources: HashMap<u32, ObjectResource>,
    pub instances: BTreeMap<u32, ObjectInstance>,
    /// Resources that failed to load, with why.
    pub failed: Vec<(u32, String)>,
}

impl Navmesh {
    /// `parseNavmesh` limited to `allow` (sro-polyanya `setRegionAllowList`): the open regions
    /// among them, row by row, then the sanity checks and the post-processing.
    pub fn load(data: &DataDir, allow: &HashSet<u16>) -> Result<Self, Error> {
        let paths = object_paths(data)?;
        let map = MapInfo::read(data)?;
        let mut navmesh = Navmesh::default();
        for row in 0..i32::from(map.height) {
            for col in 0..i32::from(map.width) {
                let id = region_id(col, row);
                if allow.contains(&id) && map.is_enabled(id) {
                    navmesh.add_region(data, &paths, id)?;
                }
            }
        }
        navmesh.sanity_check()?;
        navmesh.post_process();
        Ok(navmesh)
    }

    /// `parseRegion`'s tail: the instances, then any resource not loaded yet.
    fn add_region(
        &mut self,
        data: &DataDir,
        paths: &HashMap<u32, String>,
        id: u16,
    ) -> Result<(), Error> {
        let (region, instances) = parse_region(data, id)?;
        for instance in instances {
            self.add_object_instance(instance)?;
        }
        for global_id in &region.object_instance_ids {
            let object_id = self.instances[global_id].object_id;
            if self.resources.contains_key(&object_id)
                || self.failed.iter().any(|f| f.0 == object_id)
            {
                continue;
            }
            let loaded = (paths.get(&object_id))
                .ok_or_else(|| Error::Format {
                    path: "navmesh\\object.ifo".into(),
                    what: format!("no object {object_id}"),
                })
                .and_then(|path| parse_object_resource(data, path));
            match loaded {
                Ok(resource) => _ = self.resources.insert(object_id, resource),
                Err(e) => self.failed.push((object_id, e.to_string())),
            }
        }
        self.regions.insert(id, region);
        Ok(())
    }

    /// `addObjectInstance`: a second sighting (from a neighbour's file) only updates the links.
    fn add_object_instance(&mut self, instance: ObjectInstance) -> Result<(), Error> {
        let Some(existing) = self.instances.get_mut(&instance.global_id()) else {
            self.instances.insert(instance.global_id(), instance);
            return Ok(());
        };
        if instance.global_edge_links.len() != existing.global_edge_links.len() {
            return Err(navmesh_error(
                "Both object instances should have the same number of global edge links",
            ));
        }
        for (o, eo) in instance
            .global_edge_links
            .iter()
            .zip(&mut existing.global_edge_links)
        {
            if o.linked_obj_id != eo.linked_obj_id && o.linked_obj_id != -1 {
                eo.linked_obj_id = o.linked_obj_id;
                eo.linked_obj_global_id = o.linked_obj_global_id;
            }
            if o.linked_obj_edge_id != eo.linked_obj_edge_id && o.linked_obj_edge_id != -1 {
                eo.linked_obj_edge_id = o.linked_obj_edge_id;
            }
            if o.edge_id != eo.edge_id && o.edge_id != -1 {
                eo.edge_id = o.edge_id;
            }
        }
        Ok(())
    }

    /// `sanityCheck`: every global edge has its twin in a loaded neighbour; every linked object
    /// edge is unblocked.
    fn sanity_check(&self) -> Result<(), Error> {
        for (&id, region) in &self.regions {
            for edge in &region.global_edges {
                let other = edge.assoc_region.map_or(-1, |r| r[1]) as u16;
                let Some(neighbour) = self.regions.get(&other) else {
                    continue;
                };
                let twin = (neighbour.global_edges.iter())
                    .filter(|t| t.assoc_region.map(|r| r[1] as u16) == Some(id))
                    .any(|t| edges_are_equal(edge, id, t, other));
                if !twin {
                    return Err(navmesh_error(
                        "Navmesh data does not match our expectations. Global edges do not match in neighboring regions",
                    ));
                }
            }
        }
        for region in self.regions.values() {
            for global_id in &region.object_instance_ids {
                let instance = &self.instances[global_id];
                let Some(resource) = self.resources.get(&instance.object_id) else {
                    continue; // failed to load: sro-polyanya throws here; it has no such object
                };
                for link in &instance.global_edge_links {
                    if link.edge_id == -1 {
                        continue;
                    }
                    let edge = (resource.outline_edges.get(link.edge_id as usize))
                        .ok_or_else(|| navmesh_error("edge link out of range"))?;
                    if edge.flag & edge_flag::BLOCKED != 0 {
                        return Err(navmesh_error("Edge flag is blocked!"));
                    }
                }
            }
        }
        Ok(())
    }

    /// `postProcess`: each resource's areas, then every instance referenced by each loaded region
    /// its bounding box (in its owner's frame) overlaps.
    fn post_process(&mut self) {
        for resource in self.resources.values_mut() {
            set_cell_areas(resource);
        }
        let ids: Vec<(u32, u16)> = self
            .instances
            .iter()
            .map(|(&id, i)| (id, i.region_id))
            .collect();
        for (global_id, owner) in ids {
            let Some(object) = self.transformed_object(global_id, owner) else {
                continue;
            };
            let (mut min_x, mut min_z) = (f32::MAX, f32::MAX);
            let (mut max_x, mut max_z) = (f32::MIN, f32::MIN);
            for v in &object.vertices {
                (min_x, min_z) = (min_x.min(v[0]), min_z.min(v[2]));
                (max_x, max_z) = (max_x.max(v[0]), max_z.max(v[2]));
            }
            let (rx, rz) = sectors(owner);
            let tile = |v: f32| (f64::from(v) / 1920.0).floor() as i32;
            for x in rx + tile(min_x)..=rx + tile(max_x) {
                for z in rz + tile(min_z)..=rz + tile(max_z) {
                    if let Some(region) = self.regions.get_mut(&region_id(x, z)) {
                        if !region.object_instance_ids.contains(&global_id) {
                            region.object_instance_ids.push(global_id);
                        }
                    }
                }
            }
        }
    }

    /// `getTransformationFromObjectInstanceToWorld`: rotate by the yaw, then move to the center,
    /// shifted into `region`'s frame when another region owns the object.
    pub fn transformation(&self, global_id: u32, region: u16) -> Option<Matrix4x4> {
        let instance = self.instances.get(&global_id)?;
        let mut rotation = Matrix4x4::identity();
        rotation.set_rotation_y(-instance.yaw);
        let mut center = instance.center;
        if instance.region_id != region {
            let (our_x, our_z) = sectors(region);
            let (owner_x, owner_z) = sectors(instance.region_id);
            center[0] -= ((our_x - owner_x) * 1920) as f32;
            center[2] -= ((our_z - owner_z) * 1920) as f32;
        }
        let mut translation = Matrix4x4::identity();
        translation.set_translation(center);
        Some(translation.mul(&rotation))
    }

    /// `getTransformedObjectResourceForRegion`: the object's navmesh in `region`'s frame. None for
    /// an unknown instance or a resource that failed to load (sro-polyanya throws).
    pub fn transformed_object(&self, global_id: u32, region: u16) -> Option<ObjectResource> {
        let instance = self.instances.get(&global_id)?;
        let mut object = self.resources.get(&instance.object_id)?.clone();
        let m = self.transformation(global_id, region)?;
        for v in &mut object.vertices {
            *v = m.mul_point(*v);
        }
        Some(object)
    }
}

fn navmesh_error(what: &str) -> Error {
    Error::Format {
        path: "navmesh".into(),
        what: what.into(),
    }
}

/// `sanityCheck`'s `edgesAreEqual`: the twin, moved onto our border, matches either way round.
fn edges_are_equal(
    edge: &crate::region::Edge,
    src: u16,
    potential: &crate::region::Edge,
    dest: u16,
) -> bool {
    let (sx, sz) = sectors(src);
    let (dx, dz) = sectors(dest);
    let mut p = *potential;
    if dx > sx {
        (p.min[0], p.max[0]) = (1920.0, 1920.0);
    } else if dx < sx {
        (p.min[0], p.max[0]) = (0.0, 0.0);
    }
    if dz > sz {
        (p.min[1], p.max[1]) = (1920.0, 1920.0);
    } else if dz < sz {
        (p.min[1], p.max[1]) = (0.0, 0.0);
    }
    (edge.min == p.min && edge.max == p.max) || (edge.min == p.max && edge.max == p.min)
}

/// sro-polyanya `math::Matrix4x4`, row-major, the operations the transforms use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4x4(pub [[f32; 4]; 4]);

impl Matrix4x4 {
    pub fn identity() -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Matrix4x4(m)
    }

    /// `setTranslation` (it adds).
    pub fn set_translation(&mut self, v: [f32; 3]) {
        for (row, value) in self.0.iter_mut().zip(v) {
            row[3] += value;
        }
    }

    /// `setRotation(angle, {0, 1, 0})`: counter-clockwise about y.
    pub fn set_rotation_y(&mut self, angle: f32) {
        let (c, s) = (angle.cos(), angle.sin());
        let t = 1.0 - c;
        let m = &mut self.0;
        (m[0][0], m[0][1], m[0][2]) = (c, 0.0, s);
        (m[1][0], m[1][1], m[1][2]) = (0.0, t + c, 0.0);
        (m[2][0], m[2][1], m[2][2]) = (-s, 0.0, c);
    }

    pub fn mul(&self, other: &Matrix4x4) -> Matrix4x4 {
        let (a, b) = (&self.0, &other.0);
        let mut out = [[0.0; 4]; 4];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell =
                    a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j] + a[i][3] * b[3][j];
            }
        }
        Matrix4x4(out)
    }

    pub fn mul_point(&self, v: [f32; 3]) -> [f32; 3] {
        let m = &self.0;
        let row = |r: &[f32; 4]| r[0] * v[0] + r[1] * v[1] + r[2] * v[2] + r[3];
        [row(&m[0]), row(&m[1]), row(&m[2])]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::data_dir;

    const JANGAN: u16 = 0x61a8;

    fn around(center: u16) -> HashSet<u16> {
        let (x, z) = sectors(center);
        (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| region_id(x + dx, z + dz)))
            .collect()
    }

    /// Floor heights of `region`'s objects at (x, z) in its frame, any area.
    fn floors(navmesh: &Navmesh, region: u16, x: f32, z: f32) -> Vec<f32> {
        let mut out = Vec::new();
        for id in &navmesh.regions[&region].object_instance_ids {
            let Some(o) = navmesh.transformed_object(*id, region) else {
                continue;
            };
            let areas = o.cell_area_ids.iter().max().map_or(0, |m| m + 1);
            out.extend((0..areas).filter_map(|a| o.height(x, z, a)));
        }
        out
    }

    /// The server positions of vtc-sro-bot's Jangan captures (0x3013 and two spawn groups) all
    /// stand at -32.6, on the plaza object: its floor there, terrain 1-22 lower.
    #[test]
    fn server_heights_stand_on_the_jangan_plaza() {
        let Some(data) = data_dir() else { return };
        let navmesh = Navmesh::load(&data, &around(JANGAN)).unwrap();
        assert_eq!(navmesh.regions.len(), 9);
        assert_eq!(navmesh.regions[&JANGAN].object_instance_ids.len(), 62);
        for (x, y) in [
            (857.9, 854.0),
            (1153.0, 1057.0),
            (974.0, 1004.2),
            (980.3, 989.2),
            (1010.0, 1135.0),
            (1039.0, 1102.0),
            (1060.0, 960.1),
            (1098.0, 1028.0),
            (1161.0, 1175.0),
            (968.0, 1060.0),
            (970.0, 1040.0),
        ] {
            let heights = floors(&navmesh, JANGAN, x, y);
            assert!(
                heights.iter().any(|h| (h - -32.6).abs() < 0.05),
                "({x}, {y}): {heights:?}"
            );
        }
    }

    /// Every region of a sro-polyanya `/geometry?zone=` dump against the same zone loaded here:
    /// object lists, cells, areas and edges exactly, vertices within the dump's 6 significant
    /// digits. `CARAVAN_NAVMESH_ORACLE=<dump.json> CARAVAN_NAVMESH_ZONE=<x.submap>
    /// cargo test -p silkroad-navmesh --release -- --ignored oracle`.
    #[test]
    #[ignore]
    fn oracle_zone_matches_sro_polyanya() {
        let (Some(data), Some(dump), Some(zone)) = (
            data_dir(),
            std::env::var_os("CARAVAN_NAVMESH_ORACLE"),
            std::env::var_os("CARAVAN_NAVMESH_ZONE"),
        ) else {
            eprintln!("CARAVAN_NAVMESH_ORACLE / CARAVAN_NAVMESH_ZONE not set: skipped");
            return;
        };
        let allow: HashSet<u16> = (std::fs::read_to_string(zone).unwrap().split(','))
            .map(|h| u16::from_str_radix(h.trim().trim_start_matches("0x"), 16).unwrap())
            .collect();
        let navmesh = Navmesh::load(&data, &allow).unwrap();
        assert!(navmesh.failed.is_empty(), "{:?}", navmesh.failed);
        let geo: serde_json::Value = serde_json::from_slice(&std::fs::read(dump).unwrap()).unwrap();
        let (mut regions, mut objects, mut worst) = (0, 0, 0f32);
        for r in geo["regions"].as_array().unwrap() {
            let id = r["id"].as_u64().unwrap() as u16;
            let t = r["terrain"]["positions"].as_array().unwrap();
            let (ox, oz) = (t[0].as_f64().unwrap() as f32, t[2].as_f64().unwrap() as f32);
            let ours: HashSet<u32> = navmesh.regions[&id]
                .object_instance_ids
                .iter()
                .copied()
                .collect();
            let theirs: HashSet<u32> = (r["objects"].as_array().unwrap().iter())
                .map(|o| o["instanceId"].as_u64().unwrap() as u32)
                .collect();
            // sro-polyanya leaves out objects with no cells; it never sends a failed one.
            let ours: HashSet<u32> = (ours.into_iter())
                .filter(|g| {
                    navmesh
                        .transformed_object(*g, id)
                        .is_some_and(|o| !o.cells.is_empty())
                })
                .collect();
            assert_eq!(ours, theirs, "region {id:x}");
            for o in r["objects"].as_array().unwrap() {
                let g = o["instanceId"].as_u64().unwrap() as u32;
                let mine = navmesh.transformed_object(g, id).unwrap();
                let ints = |k: &str| {
                    (o[k].as_array().unwrap().iter())
                        .map(|v| v.as_u64().unwrap() as u32)
                        .collect::<Vec<_>>()
                };
                let cells: Vec<u32> = mine
                    .cells
                    .iter()
                    .flat_map(|c| c.vertex.map(u32::from))
                    .collect();
                assert_eq!(cells, ints("indices"), "{g:x} cells");
                assert_eq!(mine.cell_area_ids, ints("areaIds"), "{g:x} areas");
                let outline: Vec<u32> = (mine.outline_edges.iter())
                    .flat_map(|e| [e.src_vertex as u32, e.dest_vertex as u32, e.flag.into()])
                    .collect();
                assert_eq!(outline, ints("outlineEdges"), "{g:x} outline");
                let internal: Vec<u32> = (mine.inline_edges.iter())
                    .filter(|e| e.flag & 0x04 != 0 && e.flag & 0x03 != 0)
                    .flat_map(|e| [e.src_vertex as u32, e.dest_vertex as u32, e.flag.into()])
                    .collect();
                assert_eq!(internal, ints("internalEdges"), "{g:x} internal");
                let p = o["positions"].as_array().unwrap();
                for (v, chunk) in mine.vertices.iter().zip(p.chunks(3)) {
                    let abs = [
                        chunk[0].as_f64().unwrap() as f32,
                        chunk[1].as_f64().unwrap() as f32,
                        chunk[2].as_f64().unwrap() as f32,
                    ];
                    let local = [abs[0] - ox, abs[1], abs[2] - oz];
                    for k in 0..3 {
                        let tolerance = 0.05f32.max(abs[k].abs() * 6e-6);
                        let off = (v[k] - local[k]).abs();
                        assert!(off <= tolerance, "{g:x} vertex {v:?} != {local:?}");
                        worst = worst.max(off);
                    }
                }
                objects += 1;
            }
            regions += 1;
        }
        eprintln!("{regions} regions, {objects} objects match; worst vertex difference {worst}");
    }
}
