//! The Silkroad world around Jangan: the navmesh of the Jangan region and its 8 neighbors, read
//! from the extracted game `Data/`, answering ground heights. Coordinates are game units in
//! Jangan's frame: (0, 0)..(1920, 1920) is Jangan, the neighbors reach from -1920 to 3840.

use std::collections::HashSet;
use std::path::PathBuf;

use silkroad_navmesh::heights;
use silkroad_navmesh::navmesh::Navmesh;
use silkroad_navmesh::region::{region_id, sectors};
use silkroad_navmesh::walls::{self, Wall};
use silkroad_navmesh::{DataDir, Error};

/// Jangan's region id.
pub const JANGAN: u16 = 0x61a8;
/// The side of a region in game units.
pub const REGION_SIZE: f32 = 1920.0;

/// The extracted game `Data/`: `CARAVAN_GAME_DATA`, default `~/Workspace/silkroad/Data`.
pub fn data_path() -> PathBuf {
    std::env::var_os("CARAVAN_GAME_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join("Workspace/silkroad/Data")
        })
}

/// Heights on a square grid, game units: `size` x `size` points `spacing` apart from `origin`
/// (x, z), row by row along z.
#[derive(Debug, Clone, PartialEq)]
pub struct HeightGrid {
    pub origin: [f32; 2],
    pub spacing: f32,
    pub size: usize,
    pub heights: Vec<f32>,
}

/// The navmesh of Jangan and its 8 neighbors, and its walls (straight runs merged).
#[derive(Debug)]
pub struct World {
    navmesh: Navmesh,
    walls: Vec<Wall>,
}

impl World {
    pub fn load(data: PathBuf) -> Result<Self, Error> {
        let (x, z) = sectors(JANGAN);
        let regions: HashSet<u16> = (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| region_id(x + dx, z + dz)))
            .collect();
        let navmesh = Navmesh::load(&DataDir::folder(data), &regions)?;
        let walls = walls::merge_straight_runs(&walls::walls(&navmesh, JANGAN));
        Ok(World { navmesh, walls })
    }

    /// Every wall in Jangan's frame, straight runs merged into one wall each.
    pub fn walls(&self) -> &[Wall] {
        &self.walls
    }

    /// The terrain's heights at every `step`-th height point (a point every `20 * step` game
    /// units; `step` divides 96) over the 3x3 regions, from (-1920, -1920), rows along z.
    /// Neighbors share their edge points, so each is taken once.
    pub fn terrain_grid(&self, step: usize) -> HeightGrid {
        let (jx, jz) = sectors(JANGAN);
        let size = 3 * 96 / step + 1;
        // The region (0..3) and its height point (0..=96) of a point of the whole grid.
        let split = |g: usize| ((g / 96).min(2), g - 96 * (g / 96).min(2));
        let mut heights = Vec::with_capacity(size * size);
        for row in 0..size {
            let (rz, vz) = split(row * step);
            for column in 0..size {
                let (rx, vx) = split(column * step);
                let id = region_id(jx - 1 + rx as i32, jz - 1 + rz as i32);
                heights.push(self.navmesh.regions[&id].tile_vertex_heights[vz][vx]);
            }
        }
        HeightGrid {
            origin: [-REGION_SIZE, -REGION_SIZE],
            spacing: 20.0 * step as f32,
            size,
            heights,
        }
    }

    /// Every walkable surface over (x, z): the terrain where its tile is walkable, then each
    /// object floor; empty outside the loaded regions.
    pub fn surface_heights(&self, x: f32, z: f32) -> Vec<f32> {
        heights::surface_heights(&self.navmesh, JANGAN, x, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The world, None (test skipped) without the game data.
    fn world() -> Option<World> {
        let path = data_path();
        if !path.join("navmesh").is_dir() {
            eprintln!("game Data/ not present at {}: skipped", path.display());
            return None;
        }
        Some(World::load(path).unwrap())
    }

    /// On the Jangan plaza two surfaces are stacked: the terrain (-54.09, as
    /// `silkroad-navmesh`'s own test reads it) and the plaza's floor (-32.6).
    #[test]
    fn the_jangan_plaza_floor_over_its_terrain() {
        let Some(world) = world() else { return };
        let h = world.surface_heights(857.893_25, 854.046_9);
        assert_eq!(h.len(), 2, "{h:?}");
        assert!((h[0] - -54.09).abs() < 0.01, "{h:?}");
        assert!((h[1] - -32.6).abs() < 0.05, "{h:?}");
    }

    /// A point west of Jangan is read from the western neighbor, in that region's own frame.
    #[test]
    fn neighbors_are_read_in_their_own_frame() {
        let Some(world) = world() else { return };
        let (jx, jz) = sectors(JANGAN);
        let west = &world.navmesh.regions[&region_id(jx - 1, jz)];
        let h = west.height_at(1820.0, 500.0).unwrap();
        // The terrain comes first; an object floor stands there too.
        assert_eq!(world.surface_heights(-100.0, 500.0)[0], h);
        assert!(world.surface_heights(-1921.0, 500.0).is_empty());
        assert!(world.surface_heights(3840.0, 500.0).is_empty());
    }

    /// The grid every 4th height point: 73 x 73 points 80 apart; its points are the terrain's
    /// height points (Jangan's corner at index (24, 24), a point inside Jangan, the far corner
    /// of the north-east neighbor).
    #[test]
    fn the_terrain_grid_takes_the_height_points() {
        let Some(world) = world() else { return };
        let grid = world.terrain_grid(4);
        assert_eq!(
            (grid.size, grid.spacing, grid.heights.len()),
            (73, 80.0, 73 * 73)
        );
        let at = |column: usize, row: usize| grid.heights[row * 73 + column];
        let jangan = &world.navmesh.regions[&JANGAN];
        assert_eq!(at(24, 24), jangan.height_at(0.0, 0.0).unwrap());
        assert_eq!(at(30, 40), jangan.height_at(480.0, 1280.0).unwrap());
        let (jx, jz) = sectors(JANGAN);
        let north_east = &world.navmesh.regions[&region_id(jx + 1, jz + 1)];
        assert_eq!(at(72, 72), north_east.tile_vertex_heights[96][96]);
    }
}
