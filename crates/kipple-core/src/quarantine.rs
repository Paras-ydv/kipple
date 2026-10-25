//! Quarantine (SPEC §8.4): `<data>/quarantine/<ulid>/{payload/, manifest.json}`.

use std::path::PathBuf;

use crate::dirs::Dirs;
use crate::model::QuarantineEntry;

pub struct Quarantine {
    pub dir: PathBuf,
    pub keep_days: u32,
}

impl Quarantine {
    pub fn new(dirs: &Dirs, keep_days: u32) -> Self {
        Self {
            dir: dirs.quarantine_dir(),
            keep_days,
        }
    }

    /// Entries with readable manifests, oldest first.
    pub fn entries(&self) -> Vec<QuarantineEntry> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<QuarantineEntry> = rd
            .flatten()
            .filter_map(|e| {
                let text = std::fs::read_to_string(e.path().join("manifest.json")).ok()?;
                serde_json::from_str(&text).ok()
            })
            .collect();
        out.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        out
    }

    pub fn total_bytes(&self) -> u64 {
        self.entries().iter().map(|e| e.size.on_disk).sum()
    }
}
