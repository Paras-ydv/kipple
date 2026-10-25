//! Parallel directory walk (SPEC §5.2). `ignore::WalkParallel` behind a
//! trait so alternatives can be benchmarked (decision 0007).

use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
    Symlink,
    Other,
}

impl EntryKind {
    fn of(ft: &std::fs::FileType) -> Self {
        if ft.is_symlink() {
            EntryKind::Symlink
        } else if ft.is_dir() {
            EntryKind::Dir
        } else if ft.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        }
    }
}

pub struct WalkEntry<'a> {
    pub path: &'a Path,
    /// 1 for direct children of a root.
    pub depth: usize,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visit {
    Continue,
    /// Don't descend into this directory.
    Skip,
}

/// Called from many threads.
pub trait Visitor: Send + Sync {
    fn visit(&self, entry: &WalkEntry<'_>) -> Visit;
    fn error(&self, path: Option<&Path>, message: String);
    fn cancelled(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct WalkOptions {
    pub follow_symlinks: bool,
    pub threads: usize,
    /// Gitignore-syntax file honoured in every directory.
    pub ignore_file: Option<String>,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            follow_symlinks: false,
            threads: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
                .min(16),
            ignore_file: Some(".kippleignore".to_owned()),
        }
    }
}

pub trait Walker: Send + Sync {
    fn name(&self) -> &'static str;
    fn walk(&self, roots: &[PathBuf], opts: &WalkOptions, visitor: Arc<dyn Visitor>);
}

/// `ignore::WalkParallel`, with every built-in filter off except the custom
/// ignore file.
pub struct IgnoreWalker;

impl Walker for IgnoreWalker {
    fn name(&self) -> &'static str {
        "ignore"
    }

    fn walk(&self, roots: &[PathBuf], opts: &WalkOptions, visitor: Arc<dyn Visitor>) {
        let Some((first, rest)) = roots.split_first() else {
            return;
        };
        let mut builder = ignore::WalkBuilder::new(first);
        for root in rest {
            builder.add(root);
        }
        builder
            .standard_filters(false)
            .hidden(false)
            .parents(false)
            .ignore(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .follow_links(opts.follow_symlinks)
            .threads(opts.threads);
        if let Some(name) = &opts.ignore_file {
            builder.add_custom_ignore_filename(name);
        }
        let follow = opts.follow_symlinks;
        builder.build_parallel().run(|| {
            let visitor = Arc::clone(&visitor);
            Box::new(move |result| {
                use ignore::WalkState;
                if visitor.cancelled() {
                    return WalkState::Quit;
                }
                match result {
                    Ok(entry) => {
                        if entry.depth() == 0 {
                            return WalkState::Continue;
                        }
                        let kind = entry
                            .file_type()
                            .map(|ft| EntryKind::of(&ft))
                            .unwrap_or(EntryKind::Other);
                        let kind = if entry.path_is_symlink() && !follow {
                            EntryKind::Symlink
                        } else {
                            kind
                        };
                        let e = WalkEntry {
                            path: entry.path(),
                            depth: entry.depth(),
                            kind,
                        };
                        match visitor.visit(&e) {
                            Visit::Continue => WalkState::Continue,
                            Visit::Skip => WalkState::Skip,
                        }
                    }
                    Err(err) => {
                        visitor.error(None, err.to_string());
                        WalkState::Continue
                    }
                }
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Default)]
    struct Collect {
        seen: Mutex<Vec<PathBuf>>,
        skip_name: Option<&'static str>,
        cancel_after: Option<usize>,
        cancelled: AtomicBool,
    }

    impl Visitor for Collect {
        fn visit(&self, e: &WalkEntry<'_>) -> Visit {
            let mut seen = self.seen.lock().unwrap();
            seen.push(e.path.to_path_buf());
            if let Some(n) = self.cancel_after {
                if seen.len() >= n {
                    self.cancelled.store(true, Ordering::Relaxed);
                }
            }
            if self
                .skip_name
                .is_some_and(|n| e.path.file_name().unwrap() == n)
            {
                Visit::Skip
            } else {
                Visit::Continue
            }
        }
        fn error(&self, _: Option<&Path>, _: String) {}
        fn cancelled(&self) -> bool {
            self.cancelled.load(Ordering::Relaxed)
        }
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for p in [
            "a/b/c.txt",
            "a/skipme/inner/x.txt",
            "ign/y.txt",
            "keep/z.txt",
            ".hidden/h.txt",
        ] {
            let full = root.join(p);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, "x").unwrap();
        }
        std::fs::write(root.join(".kippleignore"), "ign/\n").unwrap();
        dir
    }

    fn run(skip: Option<&'static str>) -> Vec<String> {
        let dir = tree();
        let root = dir.path().canonicalize().unwrap();
        let v = Arc::new(Collect {
            skip_name: skip,
            ..Default::default()
        });
        IgnoreWalker.walk(
            std::slice::from_ref(&root),
            &WalkOptions::default(),
            v.clone(),
        );
        let mut seen: Vec<String> = v
            .seen
            .lock()
            .unwrap()
            .iter()
            .map(|p| {
                p.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        seen.sort();
        seen
    }

    #[test]
    fn walks_everything_but_skipped_and_ignored() {
        for skip in [None, Some("skipme")] {
            let a = run(skip);
            assert!(a.contains(&".hidden/h.txt".to_owned()));
            assert!(
                !a.iter().any(|p| p.starts_with("ign")),
                "kippleignore honoured"
            );
            if skip.is_some() {
                assert!(a.contains(&"a/skipme".to_owned()));
                assert!(!a.iter().any(|p| p.starts_with("a/skipme/")));
            }
        }
    }

    #[test]
    fn cancellation_stops_walk() {
        {
            let dir = tempfile::tempdir().unwrap();
            for i in 0..200 {
                std::fs::create_dir_all(dir.path().join(format!("d{i}/e"))).unwrap();
            }
            let v = Arc::new(Collect {
                cancel_after: Some(5),
                ..Default::default()
            });
            IgnoreWalker.walk(
                &[dir.path().to_path_buf()],
                &WalkOptions::default(),
                v.clone(),
            );
            assert!(v.seen.lock().unwrap().len() < 400, "kept walking");
        }
    }
}
