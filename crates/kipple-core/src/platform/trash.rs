//! OS trash wrapper (SPEC §16).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where trashed items go.
#[derive(Debug, Clone)]
pub enum TrashBin {
    /// The OS trash via the `trash` crate.
    System,
    /// A plain directory. Used inside test sandboxes so tests never touch the
    /// real trash.
    Directory(PathBuf),
    /// No trash on this system (e.g. headless Linux).
    Unavailable,
}

impl TrashBin {
    pub fn available(&self) -> bool {
        match self {
            TrashBin::System => true,
            TrashBin::Directory(_) => true,
            TrashBin::Unavailable => false,
        }
    }

    pub fn trash(&self, path: &Path) -> io::Result<()> {
        match self {
            TrashBin::System => trash::delete(path).map_err(io::Error::other),
            TrashBin::Directory(dir) => {
                fs::create_dir_all(dir)?;
                let name = path
                    .file_name()
                    .ok_or_else(|| io::Error::other("path has no file name"))?;
                let target = dir.join(format!(
                    "{}-{}",
                    ulid::Ulid::generate(),
                    name.to_string_lossy()
                ));
                fs::rename(path, target)
            }
            TrashBin::Unavailable => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "no trash is available on this system",
            )),
        }
    }
}
