//! Append-only journal of executed actions (SPEC §8.5).

use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::model::JournalEntry;

pub struct Journal {
    path: PathBuf,
}

impl Journal {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one line and syncs it to disk before returning.
    pub fn append(&self, entry: &JournalEntry) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let mut line = serde_json::to_vec(entry).map_err(io::Error::other)?;
        line.push(b'\n');
        f.write_all(&line)?;
        f.sync_data()
    }

    /// Every readable entry, oldest first. Damaged lines are skipped.
    pub fn read(&self) -> Vec<JournalEntry> {
        let Ok(f) = std::fs::File::open(&self.path) else {
            return Vec::new();
        };
        io::BufReader::new(f)
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| serde_json::from_str(&l).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    #[test]
    fn appends_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let j = Journal::new(dir.path().join("sub/journal.jsonl"));
        let e = |n: u64| JournalEntry {
            ts: Timestamp::now(),
            plan_id: PlanId::from("p"),
            item_id: ItemId::from(format!("i{n}")),
            integration: IntegrationId::from("x"),
            title: "t".into(),
            action: ActionKind::Quarantine,
            outcome: Outcome::Done { freed: n },
            freed: n,
            original_path: None,
            quarantine_id: None,
        };
        j.append(&e(1)).unwrap();
        j.append(&e(2)).unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(j.path())
            .unwrap()
            .write_all(b"garbage\n")
            .unwrap();
        let read = j.read();
        assert_eq!(read.len(), 2);
        assert_eq!(read[1].freed, 2);
    }
}
