//! The game's navmesh around a character (D31): a port of sro-polyanya's reading of it
//! (`silkroad_lib` `pk2/navmeshParser.cpp`, `navmesh/navmesh.cpp`), from the client's `Data`
//! files: the extracted `Data/` folder in development, `Data.pk2` itself on a player's PC (D39).
//! Route planning stays in sro-polyanya (D5); this answers local questions.

pub mod heights;
pub mod navmesh;
pub mod object;
mod reader;
pub mod region;

use std::path::PathBuf;
use std::sync::Arc;

pub use navmesh::Navmesh;
pub use reader::Reader;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {what}")]
    Format { path: String, what: String },
    #[error("{entry}: {what}")]
    Read { entry: String, what: String },
}

/// The client's `Data` files, read by entry name (`navmesh\nv_61a8.nvm`).
#[derive(Clone)]
pub struct DataDir(Arc<ReadEntry>);

/// Reads one entry's bytes.
type ReadEntry = dyn Fn(&str) -> Result<Vec<u8>, Error> + Send + Sync;

impl DataDir {
    /// The extracted `Data/` folder, read as sro-polyanya's loose mode does: `\` turned into
    /// `/` (`Pk2ReaderModern::resolveLoosePath`).
    pub fn folder(root: PathBuf) -> Self {
        DataDir(Arc::new(move |entry: &str| {
            let path = root.join(entry.replace('\\', "/"));
            std::fs::read(&path).map_err(|source| Error::Io { path, source })
        }))
    }

    /// Any other source of the files, such as `Data.pk2`.
    pub fn new(read: impl Fn(&str) -> Result<Vec<u8>, Error> + Send + Sync + 'static) -> Self {
        DataDir(Arc::new(read))
    }

    pub fn read(&self, entry: &str) -> Result<Vec<u8>, Error> {
        (self.0)(entry)
    }
}

impl std::fmt::Debug for DataDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DataDir")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::DataDir;

    /// The extracted client's `Data/` (`CARAVAN_GAME_DATA`, default `~/Workspace/silkroad/Data`),
    /// None to skip: game files are not in git.
    pub(crate) fn data_dir() -> Option<DataDir> {
        data_path().map(DataDir::folder)
    }

    /// The extracted `Data/` folder itself, None to skip.
    pub(crate) fn data_path() -> Option<std::path::PathBuf> {
        let path = std::env::var_os("CARAVAN_GAME_DATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var_os("HOME").unwrap_or_default();
                std::path::PathBuf::from(home).join("Workspace/silkroad/Data")
            });
        if !path.join("navmesh").is_dir() {
            eprintln!("game Data/ not present at {}: skipped", path.display());
            return None;
        }
        Some(path)
    }
}
