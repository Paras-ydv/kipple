//! Attributor (SPEC §6): when a project was last used.

use std::path::Path;
use std::time::SystemTime;

use crate::model::{LastUsed, LastUsedSource, Timestamp};

/// Most source files sampled per project.
const SAMPLE_LIMIT: usize = 2_000;

/// Folders never sampled for source mtimes.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "build",
    "dist",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".gradle",
    "Pods",
    "DerivedData",
    ".build",
    "out",
    "coverage",
];

fn to_ts(t: SystemTime) -> Option<Timestamp> {
    Timestamp::try_from(t).ok()
}

/// Time of the HEAD commit, read with `gix` (read-only).
pub fn git_head_time(root: &Path) -> Option<Timestamp> {
    let repo = gix::open(root).ok()?;
    let commit = repo.head_commit().ok()?;
    let time = commit.time().ok()?;
    Timestamp::from_second(time.seconds).ok()
}

fn git_index_time(root: &Path) -> Option<Timestamp> {
    root.join(".git/index")
        .metadata()
        .ok()?
        .modified()
        .ok()
        .and_then(to_ts)
}

/// Newest mtime among up to [`SAMPLE_LIMIT`] files, breadth-first from the
/// root, skipping artifact folders.
fn source_mtime(root: &Path) -> Option<Timestamp> {
    let mut newest: Option<SystemTime> = None;
    let mut seen = 0usize;
    let mut queue = std::collections::VecDeque::from([root.to_path_buf()]);
    while let Some(dir) = queue.pop_front() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            let name = entry.file_name();
            if ft.is_dir() {
                let n = name.to_string_lossy();
                if !SKIP_DIRS.contains(&n.as_ref()) && !n.starts_with('.') {
                    queue.push_back(entry.path());
                }
            } else if ft.is_file() {
                if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                    newest = newest.max(Some(m));
                }
                seen += 1;
                if seen >= SAMPLE_LIMIT {
                    return newest.and_then(to_ts);
                }
            }
        }
    }
    newest.and_then(to_ts)
}

/// Max of the HEAD commit time, `.git/index` mtime and sampled source mtimes;
/// records which one won.
pub fn project_last_used(root: &Path, has_git: bool) -> Option<LastUsed> {
    let mut best: Option<LastUsed> = None;
    let mut consider = |at: Option<Timestamp>, source| {
        if let Some(at) = at {
            best = LastUsed::newest(best, Some(LastUsed { at, source }));
        }
    };
    if has_git {
        consider(git_head_time(root), LastUsedSource::GitCommit);
        consider(git_index_time(root), LastUsedSource::GitIndex);
    }
    consider(source_mtime(root), LastUsedSource::SourceMtime);
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn set_mtime(path: &Path, secs_ago: u64) {
        let t = SystemTime::now() - Duration::from_secs(secs_ago);
        let f = std::fs::File::options().write(true).open(path).unwrap();
        f.set_modified(t).unwrap();
    }

    #[test]
    fn newest_source_file_wins_and_artifacts_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
        std::fs::write(root.join("src/a.js"), "a").unwrap();
        std::fs::write(root.join("node_modules/x/b.js"), "b").unwrap();
        set_mtime(&root.join("src/a.js"), 40 * 86_400);
        // node_modules file is fresh but must be ignored.
        let lu = project_last_used(root, false).unwrap();
        assert_eq!(lu.source, LastUsedSource::SourceMtime);
        let age = Timestamp::now().as_second() - lu.at.as_second();
        assert!(age >= 39 * 86_400, "age {age}");
    }

    #[test]
    fn no_git_no_files_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(project_last_used(dir.path(), false).is_none());
        assert!(git_head_time(dir.path()).is_none());
    }
}
