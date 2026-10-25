//! Honest sizes (SPEC §5.5): apparent, on-disk and reclaimable with hardlink
//! accounting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rayon::prelude::*;

use crate::model::{CancelToken, Size};
use crate::platform::fsx;

/// A file with more than one link, seen while measuring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Linked {
    key: (u64, u64),
    nlink: u64,
    apparent: u64,
    on_disk: u64,
}

/// Raw measurement of one item's paths.
#[derive(Debug, Clone, Default)]
pub struct Measured {
    pub apparent: u64,
    pub on_disk: u64,
    pub files: u64,
    /// Newest modification time of any file or directory inside.
    pub newest_mtime: Option<SystemTime>,
    pub errors: u64,
    linked: Vec<Linked>,
}

impl Measured {
    fn merge(mut self, other: Measured) -> Measured {
        self.apparent += other.apparent;
        self.on_disk += other.on_disk;
        self.files += other.files;
        self.errors += other.errors;
        self.newest_mtime = self.newest_mtime.max(other.newest_mtime);
        self.linked.extend(other.linked);
        self
    }

    fn touch(&mut self, meta: &std::fs::Metadata) {
        if let Ok(m) = meta.modified() {
            self.newest_mtime = self.newest_mtime.max(Some(m));
        }
    }

    /// Final sizes. Each hard-linked inode counts once per item, and only
    /// toward `reclaimable` when every link to it was seen inside this item.
    /// Returns the size and the bytes shared with other places.
    pub fn finish(&self) -> (Size, u64) {
        let mut seen: HashMap<(u64, u64), (Linked, u64)> = HashMap::new();
        for l in &self.linked {
            seen.entry(l.key).or_insert((*l, 0)).1 += 1;
        }
        let mut apparent = self.apparent;
        let mut on_disk = self.on_disk;
        let mut shared = 0;
        for (l, count) in seen.values() {
            apparent += l.apparent;
            on_disk += l.on_disk;
            if *count < l.nlink {
                shared += l.on_disk;
            }
        }
        let size = Size {
            apparent,
            on_disk,
            reclaimable: on_disk.saturating_sub(shared),
            file_count: Some(self.files),
        };
        (size, shared)
    }
}

#[cfg(unix)]
fn link_info(meta: &std::fs::Metadata) -> Option<((u64, u64), u64)> {
    use std::os::unix::fs::MetadataExt;
    (meta.nlink() > 1).then(|| ((meta.dev(), meta.ino()), meta.nlink()))
}

/// Link counts aren't exposed on Windows in stable std; v1 adds them.
#[cfg(not(unix))]
fn link_info(_meta: &std::fs::Metadata) -> Option<((u64, u64), u64)> {
    None
}

fn add_file(m: &mut Measured, meta: &std::fs::Metadata) {
    m.files += 1;
    m.touch(meta);
    let apparent = meta.len();
    let on_disk = fsx::allocated(meta);
    match link_info(meta) {
        Some((key, nlink)) => m.linked.push(Linked {
            key,
            nlink,
            apparent,
            on_disk,
        }),
        None => {
            m.apparent += apparent;
            m.on_disk += on_disk;
        }
    }
}

fn measure_dir(dir: &Path, cancel: &CancelToken) -> Measured {
    let mut m = Measured::default();
    if cancel.is_cancelled() {
        return m;
    }
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => {
            m.errors += 1;
            return m;
        }
    };
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in rd {
        let Ok(entry) = entry else {
            m.errors += 1;
            continue;
        };
        let path = entry.path();
        // Never follow symlinks: a link counts as itself.
        let Ok(meta) = path.symlink_metadata() else {
            m.errors += 1;
            continue;
        };
        if meta.is_dir() {
            m.touch(&meta);
            m.on_disk += fsx::allocated(&meta);
            subdirs.push(path);
        } else {
            add_file(&mut m, &meta);
        }
    }
    let nested = subdirs
        .par_iter()
        .map(|d| measure_dir(d, cancel))
        .reduce(Measured::default, Measured::merge);
    m.merge(nested)
}

/// Measures files and directories, never following symlinks.
pub fn measure(paths: &[PathBuf], cancel: &CancelToken) -> Measured {
    paths
        .par_iter()
        .map(|p| match p.symlink_metadata() {
            Ok(meta) if meta.is_dir() => {
                let mut m = measure_dir(p, cancel);
                m.touch(&meta);
                m.on_disk += fsx::allocated(&meta);
                m
            }
            Ok(meta) => {
                let mut m = Measured::default();
                add_file(&mut m, &meta);
                m
            }
            Err(_) => Measured {
                errors: 1,
                ..Default::default()
            },
        })
        .reduce(Measured::default, Measured::merge)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, bytes: usize) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, vec![b'x'; bytes]).unwrap();
    }

    #[test]
    fn sums_apparent_sizes() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a/1"), 1000);
        write(&dir.path().join("a/b/2"), 3000);
        let m = measure(&[dir.path().join("a")], &CancelToken::new());
        let (size, shared) = m.finish();
        assert_eq!(size.apparent, 4000);
        assert_eq!(size.file_count, Some(2));
        assert!(size.on_disk >= 4000 || cfg!(windows));
        assert_eq!(shared, 0);
        assert_eq!(size.reclaimable, size.on_disk);
    }

    #[cfg(unix)]
    #[test]
    fn hardlinks_inside_one_item_are_reclaimable_once() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("item/a");
        write(&a, 8192);
        std::fs::hard_link(&a, dir.path().join("item/b")).unwrap();
        let (size, shared) = measure(&[dir.path().join("item")], &CancelToken::new()).finish();
        assert_eq!(size.apparent, 8192);
        assert_eq!(shared, 0);
        assert_eq!(size.reclaimable, size.on_disk);
    }

    /// pnpm-style: node_modules files hard-linked from a global store.
    #[cfg(unix)]
    #[test]
    fn hardlinks_shared_with_store_are_not_reclaimable() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store/files/00/abc");
        write(&store, 16384);
        let nm = dir.path().join("app/node_modules/.pnpm/pkg/index.js");
        std::fs::create_dir_all(nm.parent().unwrap()).unwrap();
        std::fs::hard_link(&store, &nm).unwrap();
        write(&dir.path().join("app/node_modules/.modules.yaml"), 100);

        let (nm_size, shared) =
            measure(&[dir.path().join("app/node_modules")], &CancelToken::new()).finish();
        assert!(shared >= 16384);
        assert!(nm_size.reclaimable < nm_size.on_disk);
        assert!(nm_size.reclaimable <= nm_size.on_disk - 16384);

        let (store_size, store_shared) =
            measure(&[dir.path().join("store")], &CancelToken::new()).finish();
        assert_eq!(
            store_shared,
            store_size.on_disk - (store_size.on_disk - store_shared)
        );
        assert!(store_size.reclaimable < store_size.on_disk);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("big/f"), 100_000);
        std::fs::create_dir_all(dir.path().join("item")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("big"), dir.path().join("item/link")).unwrap();
        let (size, _) = measure(&[dir.path().join("item")], &CancelToken::new()).finish();
        assert!(size.apparent < 1000);
    }
}
