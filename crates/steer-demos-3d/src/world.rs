//! The Silkroad world around Jangan: the navmesh of the Jangan region and its 8 neighbors, read
//! from the extracted game `Data/`, answering ground heights. Coordinates are game units in
//! Jangan's frame: (0, 0)..(1920, 1920) is Jangan, the neighbors reach from -1920 to 3840.

use std::collections::HashSet;
use std::path::PathBuf;

use silkroad_navmesh::heights;
use silkroad_navmesh::navmesh::Navmesh;
use silkroad_navmesh::region::{region_id, sectors};
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

/// The navmesh of Jangan and its 8 neighbors.
#[derive(Debug)]
pub struct World {
    navmesh: Navmesh,
}

impl World {
    pub fn load(data: PathBuf) -> Result<Self, Error> {
        let (x, z) = sectors(JANGAN);
        let regions: HashSet<u16> = (-1..=1)
            .flat_map(|dz| (-1..=1).map(move |dx| region_id(x + dx, z + dz)))
            .collect();
        let navmesh = Navmesh::load(&DataDir::folder(data), &regions)?;
        Ok(World { navmesh })
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
}
