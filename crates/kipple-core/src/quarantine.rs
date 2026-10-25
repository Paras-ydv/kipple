//! Quarantine (SPEC §8.4): `<data>/quarantine/<ulid>/{payload/, manifest.json}`.
//!
//! Moving in and out is always an atomic rename on the same volume; never
//! copy + delete.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::dirs::Dirs;
use crate::model::{
    Item, PlanId, QuarantineEntry, QuarantineId, RestoreReport, RestoredEntry, SkippedRestore,
    Timestamp,
};
use crate::platform::{Env, fsx};

const MANIFEST: &str = "manifest.json";
const PAYLOAD: &str = "payload";

#[derive(Debug, thiserror::Error)]
pub enum QuarantineError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Sandbox(#[from] fsx::SandboxViolation),
    #[error("{0}")]
    Refused(String),
}

pub struct Quarantine {
    pub dir: PathBuf,
    pub keep_days: u32,
}

/// Payload location of the n-th original path.
fn payload_path(entry_dir: &Path, index: usize, original: &Path) -> PathBuf {
    let name = original
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| "item".into());
    entry_dir.join(PAYLOAD).join(index.to_string()).join(name)
}

/// Makes everything under `path` writable so it can be deleted (Go's module
/// cache is read-only on purpose).
fn make_writable(path: &Path) {
    let Ok(meta) = path.symlink_metadata() else {
        return;
    };
    if meta.file_type().is_symlink() {
        return;
    }
    let mut perms = meta.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(perms.mode() | 0o700);
    }
    #[cfg(not(unix))]
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = fs::set_permissions(path, perms);
    if meta.is_dir() {
        if let Ok(rd) = fs::read_dir(path) {
            for e in rd.flatten() {
                make_writable(&e.path());
            }
        }
    }
}

/// Deletes a file or directory tree, fixing permissions if needed.
pub fn remove_tree(path: &Path) -> io::Result<()> {
    let meta = path.symlink_metadata()?;
    let result = if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    match result {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            make_writable(path);
            if meta.is_dir() {
                fs::remove_dir_all(path)
            } else {
                fs::remove_file(path)
            }
        }
        Err(e) => Err(e),
    }
}

impl Quarantine {
    pub fn new(dirs: &Dirs, keep_days: u32) -> Self {
        Self {
            dir: dirs.quarantine_dir(),
            keep_days,
        }
    }

    /// Whether `path` can be renamed into the quarantine.
    pub fn same_volume(&self, path: &Path) -> bool {
        fsx::same_volume(path, &self.dir)
    }

    /// Entries with readable manifests, oldest first.
    pub fn entries(&self) -> Vec<QuarantineEntry> {
        let Ok(rd) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<QuarantineEntry> = rd
            .flatten()
            .filter_map(|e| {
                let text = fs::read_to_string(e.path().join(MANIFEST)).ok()?;
                serde_json::from_str(&text).ok()
            })
            .collect();
        out.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
        out
    }

    pub fn get(&self, id: &QuarantineId) -> Option<QuarantineEntry> {
        let text = fs::read_to_string(self.dir.join(id.as_str()).join(MANIFEST)).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn total_bytes(&self) -> u64 {
        self.entries().iter().map(|e| e.size.on_disk).sum()
    }

    /// Moves every path of `item` into a new entry. Either all paths move or
    /// none do.
    pub fn put(
        &self,
        env: &Env,
        plan_id: &PlanId,
        item: &Item,
    ) -> Result<QuarantineEntry, QuarantineError> {
        let originals: Vec<PathBuf> = item.locator.paths().into_iter().cloned().collect();
        if originals.is_empty() {
            return Err(QuarantineError::Refused("item has no paths".into()));
        }
        for p in &originals {
            fsx::sandbox_guard(env, p)?;
        }
        let id = QuarantineId::generate();
        let entry_dir = self.dir.join(id.as_str());
        fsx::sandbox_guard(env, &entry_dir)?;
        fs::create_dir_all(entry_dir.join(PAYLOAD))?;
        let now = Timestamp::now();
        let entry = QuarantineEntry {
            id: id.clone(),
            plan_id: plan_id.clone(),
            created_at: now,
            expires_at: Timestamp::from_second(
                now.as_second() + i64::from(self.keep_days) * 86_400,
            )
            .unwrap_or(now),
            integration: item.integration.clone(),
            original_paths: originals.clone(),
            item: item.clone(),
            size: item.size,
        };
        // Manifest first: a crash mid-move still leaves a restorable record.
        write_atomic(
            &entry_dir.join(MANIFEST),
            &serde_json::to_vec_pretty(&entry)?,
        )?;
        let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
        for (i, original) in originals.iter().enumerate() {
            let target = payload_path(&entry_dir, i, original);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(e) = fs::rename(original, &target) {
                // Roll back what already moved.
                for (from, to) in moved.iter().rev() {
                    let _ = fs::rename(to, from);
                }
                let _ = fs::remove_dir_all(&entry_dir);
                return Err(e.into());
            }
            moved.push((original.clone(), target));
        }
        Ok(entry)
    }

    /// Moves an entry back. Refuses when an original path exists again,
    /// unless `to` gives another folder to restore into.
    pub fn restore(
        &self,
        env: &Env,
        id: &QuarantineId,
        to: Option<&Path>,
    ) -> Result<RestoredEntry, QuarantineError> {
        let entry = self
            .get(id)
            .ok_or_else(|| QuarantineError::Refused(format!("no quarantine entry `{id}`")))?;
        let entry_dir = self.dir.join(id.as_str());
        let mut targets = Vec::new();
        for (i, original) in entry.original_paths.iter().enumerate() {
            let target = match to {
                Some(dir) => dir.join(original.file_name().unwrap_or_default()),
                None => original.clone(),
            };
            if target.symlink_metadata().is_ok() {
                return Err(QuarantineError::Refused(format!(
                    "{} exists again; restore somewhere else with --to <folder>",
                    target.display()
                )));
            }
            fsx::sandbox_guard(env, &target)?;
            targets.push((payload_path(&entry_dir, i, original), target));
        }
        for (from, to) in &targets {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(from, to)?;
        }
        fs::remove_dir_all(&entry_dir)?;
        Ok(RestoredEntry {
            id: id.clone(),
            title: entry.item.title.clone(),
            paths: targets.into_iter().map(|(_, t)| t).collect(),
        })
    }

    /// Restores several entries, collecting refusals instead of stopping.
    pub fn restore_many(&self, env: &Env, ids: &[QuarantineId]) -> RestoreReport {
        let mut report = RestoreReport::default();
        for id in ids {
            match self.restore(env, id, None) {
                Ok(r) => report.restored.push(r),
                Err(e) => report.skipped.push(SkippedRestore {
                    id: id.clone(),
                    reason: e.to_string(),
                }),
            }
        }
        report
    }

    /// Deletes an entry for good. Returns the bytes freed.
    pub fn purge_one(&self, env: &Env, id: &QuarantineId) -> Result<u64, QuarantineError> {
        let entry_dir = self.dir.join(id.as_str());
        fsx::sandbox_guard(env, &entry_dir)?;
        let size = self.get(id).map(|e| e.size.on_disk).unwrap_or(0);
        remove_tree(&entry_dir)?;
        Ok(size)
    }

    /// Entries older than `older_than_secs` (default: expired).
    pub fn expired(&self, older_than_secs: Option<u64>) -> Vec<QuarantineEntry> {
        let now = Timestamp::now().as_second();
        self.entries()
            .into_iter()
            .filter(|e| match older_than_secs {
                Some(s) => now - e.created_at.as_second() >= s as i64,
                None => e.expires_at.as_second() <= now,
            })
            .collect()
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::testing::TestHome;

    fn item(paths: Vec<PathBuf>) -> Item {
        let integration = IntegrationId::from("t");
        let locator = if paths.len() == 1 {
            Locator::Path {
                path: paths[0].clone(),
            }
        } else {
            Locator::Paths { paths }
        };
        Item {
            id: ItemId::new(&integration, &locator),
            integration,
            category: Category::ProjectArtifact,
            kind: "k".into(),
            title: "thing".into(),
            locator,
            size: Size::simple(10, 4096),
            project: None,
            last_used: None,
            safety: Safety::Safe,
            reasons: vec![],
            actions: vec![ActionKind::Quarantine],
            details: serde_json::Value::Null,
            notes: vec![],
            restore_hint: None,
            identity: None,
        }
    }

    fn setup() -> (TestHome, Quarantine) {
        let home = TestHome::new().unwrap();
        let dirs = Dirs::resolve(home.env(), crate::platform::os::current_os()).unwrap();
        (home, Quarantine::new(&dirs, 7))
    }

    #[test]
    fn put_and_restore_round_trip() {
        let (home, q) = setup();
        home.write("a/node_modules/x/index.js", "hello").unwrap();
        let p = home.join("a/node_modules");
        let e = q
            .put(home.env(), &PlanId::from("p1"), &item(vec![p.clone()]))
            .unwrap();
        assert!(!p.exists());
        assert_eq!(q.entries().len(), 1);
        q.restore(home.env(), &e.id, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(p.join("x/index.js")).unwrap(),
            "hello"
        );
        assert!(q.entries().is_empty());
    }

    #[test]
    fn multi_path_items_move_together() {
        let (home, q) = setup();
        home.write("s/a/__pycache__/x.pyc", "1").unwrap();
        home.write("s/b/__pycache__/y.pyc", "2").unwrap();
        let paths = vec![home.join("s/a/__pycache__"), home.join("s/b/__pycache__")];
        let e = q
            .put(home.env(), &PlanId::from("p"), &item(paths.clone()))
            .unwrap();
        assert!(paths.iter().all(|p| !p.exists()));
        q.restore(home.env(), &e.id, None).unwrap();
        assert!(paths.iter().all(|p| p.exists()));
    }

    #[test]
    fn failed_move_rolls_back() {
        let (home, q) = setup();
        home.write("r/a/f", "1").unwrap();
        let paths = vec![home.join("r/a"), home.join("r/missing")];
        assert!(q.put(home.env(), &PlanId::from("p"), &item(paths)).is_err());
        assert!(home.join("r/a/f").exists());
        assert!(q.entries().is_empty());
    }

    #[test]
    fn restore_refuses_when_original_exists_again() {
        let (home, q) = setup();
        home.write("b/target/x", "1").unwrap();
        let p = home.join("b/target");
        let e = q
            .put(home.env(), &PlanId::from("p"), &item(vec![p.clone()]))
            .unwrap();
        home.write("b/target/new", "2").unwrap();
        let err = q.restore(home.env(), &e.id, None).unwrap_err();
        assert!(err.to_string().contains("exists again"));
        let to = home.mkdir("elsewhere").unwrap();
        let r = q.restore(home.env(), &e.id, Some(&to)).unwrap();
        assert_eq!(r.paths, vec![to.join("target")]);
        assert!(to.join("target/x").exists());
    }

    #[cfg(unix)]
    #[test]
    fn purge_handles_read_only_trees() {
        use std::os::unix::fs::PermissionsExt;
        let (home, q) = setup();
        home.write("go/pkg/mod/m@v1/f.go", "x").unwrap();
        let m = home.join("go/pkg/mod/m@v1");
        std::fs::set_permissions(m.join("f.go"), std::fs::Permissions::from_mode(0o444)).unwrap();
        std::fs::set_permissions(&m, std::fs::Permissions::from_mode(0o555)).unwrap();
        let e = q
            .put(
                home.env(),
                &PlanId::from("p"),
                &item(vec![home.join("go/pkg/mod")]),
            )
            .unwrap();
        q.purge_one(home.env(), &e.id).unwrap();
        assert!(q.entries().is_empty());
    }

    #[test]
    fn refuses_outside_sandbox() {
        let (home, q) = setup();
        let outside = std::env::temp_dir().join("kipple-not-yours");
        let err = q
            .put(home.env(), &PlanId::from("p"), &item(vec![outside]))
            .unwrap_err();
        assert!(matches!(err, QuarantineError::Sandbox(_)));
    }
}
