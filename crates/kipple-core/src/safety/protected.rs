//! Protected locations (SPEC §7.4) and the last-line `is_actionable` guard
//! every destructive operation calls (SPEC §8.3).

use std::path::{Component, Path, PathBuf};

use crate::dirs::Dirs;
use crate::model::Os;
use crate::platform::{Env, paths};

/// How a location is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionKind {
    /// The location and everything inside it (`~/.ssh`, kipple's data dir).
    Subtree,
    /// Personal folders (`~/Documents`, `~/Desktop`, …). The folder itself and
    /// anything not positively identified are protected; project artifacts
    /// and caches found inside are judged as usual (decision 0005).
    Folder,
    /// Only the path itself (scan roots).
    Exact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedLocation {
    pub path: PathBuf,
    pub kind: ProtectionKind,
    /// Short user-facing name, e.g. "your SSH keys".
    pub label: String,
}

/// Result of checking a path against the protected set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Protection {
    None,
    /// The path is, or is inside, a fully protected location.
    Protected {
        label: String,
    },
    /// The path contains a protected location (removing it would remove that).
    ContainsProtected {
        label: String,
    },
    /// Inside a personal folder; only identified dev artifacts may be acted on.
    InsideFolder {
        label: String,
    },
}

impl Protection {
    /// Blocks every item regardless of integration.
    pub fn is_hard(&self) -> bool {
        matches!(
            self,
            Protection::Protected { .. } | Protection::ContainsProtected { .. }
        )
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            Protection::None => None,
            Protection::Protected { label }
            | Protection::ContainsProtected { label }
            | Protection::InsideFolder { label } => Some(label),
        }
    }
}

/// Why `is_actionable` refused a path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("{path} is protected ({label})")]
    Protected { path: String, label: String },
    #[error("{path} contains {label}")]
    ContainsProtected { path: String, label: String },
    #[error("{path} is a git directory or holds one")]
    Git { path: String },
    #[error("{path} is too close to the filesystem root")]
    TooShallow { path: String },
    #[error("{path} is outside the scanned locations")]
    OutsideRoots { path: String },
    #[error("{path} is not an absolute path")]
    Relative { path: String },
}

#[derive(Debug, Clone, Default)]
pub struct ProtectedSet {
    locations: Vec<ProtectedLocation>,
}

const HOME_SUBTREES: &[(&str, &str)] = &[
    (".ssh", "your SSH keys"),
    (".gnupg", "your GnuPG keys"),
    (".aws/credentials", "your AWS credentials"),
    (".aws/config", "your AWS config"),
    (".config/gcloud", "your Google Cloud credentials"),
    (".kube/config", "your Kubernetes config"),
    (".docker/config.json", "your Docker credentials"),
    (".netrc", "your .netrc credentials"),
    (".git-credentials", "your git credentials"),
    (".password-store", "your password store"),
];

const HOME_FOLDERS: &[(&str, &str)] = &[
    ("Documents", "your Documents folder"),
    ("Desktop", "your Desktop folder"),
    ("Pictures", "your Pictures folder"),
    ("Movies", "your Movies folder"),
    ("Music", "your Music folder"),
    ("Videos", "your Videos folder"),
    ("Downloads", "your Downloads folder"),
];

#[cfg(unix)]
const SYSTEM_SUBTREES: &[&str] = &[
    "/System",
    "/bin",
    "/sbin",
    "/etc",
    "/boot",
    "/proc",
    "/sys",
    "/dev",
    "/Library/Keychains",
];
#[cfg(windows)]
const SYSTEM_SUBTREES: &[&str] = &[
    r"C:\Windows",
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\ProgramData",
];
#[cfg(not(any(unix, windows)))]
const SYSTEM_SUBTREES: &[&str] = &[];

impl ProtectedSet {
    /// The built-in set plus `extra` (config `safety.protect`, already
    /// expanded) and every scan root (exact).
    pub fn new(env: &Env, os: Os, dirs: &Dirs, extra: &[PathBuf], roots: &[PathBuf]) -> Self {
        let mut set = Self::default();
        if let Some(home) = env.home() {
            for (rel, label) in HOME_SUBTREES {
                set.add(home.join(rel), ProtectionKind::Subtree, label);
            }
            for (rel, label) in HOME_FOLDERS {
                set.add(home.join(rel), ProtectionKind::Folder, label);
            }
            if os == Os::Macos {
                set.add(
                    home.join("Library/Keychains"),
                    ProtectionKind::Subtree,
                    "your keychains",
                );
                set.add(
                    home.join("Library/Mobile Documents"),
                    ProtectionKind::Subtree,
                    "your iCloud Drive",
                );
            }
            set.add(home, ProtectionKind::Exact, "your home folder");
        }
        for sys in SYSTEM_SUBTREES {
            set.add(
                PathBuf::from(sys),
                ProtectionKind::Subtree,
                "a system folder",
            );
        }
        set.add(
            dirs.data_dir.clone(),
            ProtectionKind::Subtree,
            "kipple's own data",
        );
        if let Some(config_dir) = dirs.config_dir() {
            set.add(
                config_dir.to_path_buf(),
                ProtectionKind::Subtree,
                "kipple's config",
            );
        }
        set.add(
            dirs.config_file.clone(),
            ProtectionKind::Subtree,
            "kipple's config",
        );
        for path in extra {
            set.add(
                path.clone(),
                ProtectionKind::Subtree,
                "a path you protected",
            );
        }
        for root in roots {
            set.add(root.clone(), ProtectionKind::Exact, "a scan root");
        }
        set
    }

    fn add(&mut self, path: PathBuf, kind: ProtectionKind, label: &str) {
        let path = paths::lexical_normalize(&path);
        // Also protect the resolved form (e.g. macOS `/var` → `/private/var`).
        let resolved = paths::canonicalize_parent(&path);
        if resolved != path {
            self.locations.push(ProtectedLocation {
                path: resolved,
                kind,
                label: label.to_owned(),
            });
        }
        self.locations.push(ProtectedLocation {
            path,
            kind,
            label: label.to_owned(),
        });
    }

    pub fn locations(&self) -> &[ProtectedLocation] {
        &self.locations
    }

    /// Checks one concrete path form.
    fn check_form(&self, path: &Path) -> Protection {
        let mut folder: Option<&ProtectedLocation> = None;
        for loc in &self.locations {
            let inside = paths::is_within(path, &loc.path);
            let equal = inside && paths::is_within(&loc.path, path);
            match loc.kind {
                ProtectionKind::Subtree if inside => {
                    return Protection::Protected {
                        label: loc.label.clone(),
                    };
                }
                ProtectionKind::Exact | ProtectionKind::Folder if equal => {
                    return Protection::Protected {
                        label: loc.label.clone(),
                    };
                }
                ProtectionKind::Folder if inside => folder = Some(loc),
                _ => {}
            }
            if !inside && paths::is_within(&loc.path, path) {
                return Protection::ContainsProtected {
                    label: loc.label.clone(),
                };
            }
        }
        match folder {
            Some(loc) => Protection::InsideFolder {
                label: loc.label.clone(),
            },
            None => Protection::None,
        }
    }

    /// Checks `path` both lexically and with symlinked parents resolved; the
    /// stricter result wins.
    pub fn check(&self, path: &Path) -> Protection {
        let lexical = paths::lexical_normalize(path);
        let resolved = paths::canonicalize_parent(path);
        let a = self.check_form(&lexical);
        if a.is_hard() {
            return a;
        }
        let b = self.check_form(&resolved);
        if b.is_hard() {
            return b;
        }
        if matches!(a, Protection::InsideFolder { .. }) {
            a
        } else {
            b
        }
    }
}

fn normal_components(path: &Path) -> usize {
    path.components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count()
}

fn has_git_component(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c, Component::Normal(n) if n.eq_ignore_ascii_case(".git")))
}

/// The final guard before anything is moved or deleted. Returns `Ok` only if
/// `path` is absolute, not protected (folder-protected paths are allowed: the
/// safety engine already restricted them to identified artifacts), not a git
/// directory and inside one of `allowed_roots` (scan roots plus
/// integration-declared locations).
pub fn is_actionable(
    set: &ProtectedSet,
    path: &Path,
    allowed_roots: &[PathBuf],
) -> Result<(), Refusal> {
    let shown = path.display().to_string();
    if !path.is_absolute() {
        return Err(Refusal::Relative { path: shown });
    }
    let lexical = paths::lexical_normalize(path);
    let resolved = paths::canonicalize_parent(path);
    for form in [&lexical, &resolved] {
        if normal_components(form) < 2 {
            return Err(Refusal::TooShallow { path: shown });
        }
        if has_git_component(form) {
            return Err(Refusal::Git { path: shown });
        }
    }
    match set.check(path) {
        Protection::Protected { label } => return Err(Refusal::Protected { path: shown, label }),
        Protection::ContainsProtected { label } => {
            return Err(Refusal::ContainsProtected { path: shown, label });
        }
        Protection::InsideFolder { .. } | Protection::None => {}
    }
    // A directory holding a `.git` is a repository, never an artifact.
    if path.join(".git").symlink_metadata().is_ok() {
        return Err(Refusal::Git { path: shown });
    }
    let inside_roots = allowed_roots.iter().any(|root| {
        let root_resolved = paths::canonicalize_parent(root);
        (paths::is_within(&lexical, root) || paths::is_within(&resolved, &root_resolved))
            && !paths::is_within(root, &lexical)
    });
    if !inside_roots {
        return Err(Refusal::OutsideRoots { path: shown });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestHome;
    use proptest::prelude::*;

    fn setup() -> (TestHome, ProtectedSet, Vec<PathBuf>) {
        let home = TestHome::new().unwrap();
        let dirs = Dirs::resolve(home.env(), crate::platform::os::current_os()).unwrap();
        let extra = vec![home.join("work/client-x")];
        let roots = vec![home.path().to_path_buf()];
        let set = ProtectedSet::new(home.env(), Os::Macos, &dirs, &extra, &roots);
        (home, set, roots)
    }

    #[test]
    fn protects_builtin_locations() {
        let (home, set, roots) = setup();
        for rel in [
            ".ssh",
            ".ssh/id_ed25519",
            ".gnupg/private-keys-v1.d",
            ".aws/credentials",
            ".kube/config",
            "Library/Keychains/login.keychain-db",
            "work/client-x/node_modules",
            "Documents",
            "Desktop",
        ] {
            let p = home.join(rel);
            assert!(set.check(&p).is_hard(), "{rel} should be protected");
            assert!(is_actionable(&set, &p, &roots).is_err(), "{rel}");
        }
    }

    #[test]
    fn protects_kipple_dirs() {
        let (home, set, roots) = setup();
        let dirs = Dirs::resolve(home.env(), crate::platform::os::current_os()).unwrap();
        assert!(is_actionable(&set, &dirs.data_dir.join("quarantine/x"), &roots).is_err());
        assert!(is_actionable(&set, &dirs.config_file, &roots).is_err());
    }

    #[test]
    fn refuses_ancestors_of_protected_locations() {
        let (home, set, roots) = setup();
        for rel in [".aws", ".config", ".local", "work", "Library"] {
            let p = home.join(rel);
            assert!(
                matches!(set.check(&p), Protection::ContainsProtected { .. }),
                "{rel}"
            );
            assert!(is_actionable(&set, &p, &roots).is_err());
        }
        assert!(is_actionable(&set, home.path(), &roots).is_err());
        assert!(is_actionable(&set, Path::new("/"), &roots).is_err());
    }

    #[test]
    fn allows_artifacts_inside_personal_folders() {
        let (home, set, roots) = setup();
        let nm = home.mkdir("Desktop/Developer/app/node_modules").unwrap();
        assert!(matches!(set.check(&nm), Protection::InsideFolder { .. }));
        assert_eq!(is_actionable(&set, &nm, &roots), Ok(()));
    }

    #[test]
    fn allows_ordinary_artifacts() {
        let (home, set, roots) = setup();
        let p = home.mkdir("code/app/node_modules").unwrap();
        assert_eq!(set.check(&p), Protection::None);
        assert_eq!(is_actionable(&set, &p, &roots), Ok(()));
    }

    #[test]
    fn refuses_git_and_repos() {
        let (home, set, roots) = setup();
        home.mkdir("code/app/.git/objects").unwrap();
        assert!(matches!(
            is_actionable(&set, &home.join("code/app/.git"), &roots),
            Err(Refusal::Git { .. })
        ));
        assert!(matches!(
            is_actionable(&set, &home.join("code/app/.git/objects"), &roots),
            Err(Refusal::Git { .. })
        ));
        assert!(matches!(
            is_actionable(&set, &home.join("code/app"), &roots),
            Err(Refusal::Git { .. })
        ));
    }

    #[test]
    fn refuses_outside_roots() {
        let (_home, set, roots) = setup();
        let other = std::env::temp_dir().join("kipple-elsewhere/x");
        assert!(matches!(
            is_actionable(&set, &other, &roots),
            Err(Refusal::OutsideRoots { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_parent_into_protected_location_is_refused() {
        let (home, set, roots) = setup();
        home.mkdir(".ssh").unwrap();
        home.mkdir("code").unwrap();
        std::os::unix::fs::symlink(home.join(".ssh"), home.join("code/link")).unwrap();
        let sneaky = home.join("code/link/id_rsa");
        assert!(is_actionable(&set, &sneaky, &roots).is_err());
    }

    #[test]
    fn dotdot_cannot_escape() {
        let (home, set, roots) = setup();
        let p = home.join("code/app/../../.ssh");
        assert!(is_actionable(&set, &p, &roots).is_err());
    }

    fn segment() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-zA-Z0-9_]{1,8}",
            Just("..".to_owned()),
            Just(".".to_owned()),
            Just(".git".to_owned()),
            Just("node_modules".to_owned()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        /// Nothing inside a fully protected location is ever actionable.
        #[test]
        fn nothing_inside_subtrees_is_actionable(
            idx in 0usize..HOME_SUBTREES.len(),
            tail in proptest::collection::vec("[a-zA-Z0-9_.]{1,8}", 0..5),
        ) {
            let (home, set, roots) = setup();
            let mut p = home.join(HOME_SUBTREES[idx].0);
            for seg in &tail {
                if seg != ".." && seg != "." {
                    p.push(seg);
                }
            }
            prop_assert!(is_actionable(&set, &p, &roots).is_err());
        }

        /// No ancestor of a protected location is ever actionable.
        #[test]
        fn no_ancestor_of_protected_is_actionable(idx in 0..HOME_SUBTREES.len()) {
            let (home, set, roots) = setup();
            let p = home.join(HOME_SUBTREES[idx].0);
            for ancestor in p.ancestors() {
                prop_assert!(is_actionable(&set, ancestor, &roots).is_err(), "{}", ancestor.display());
            }
        }

        /// Random relative paths: if actionable, then the normalized path is
        /// inside the root, not protected and has no `.git` component.
        #[test]
        fn random_paths_respect_invariants(segs in proptest::collection::vec(segment(), 0..8)) {
            let (home, set, roots) = setup();
            let mut p = home.path().to_path_buf();
            for s in &segs {
                p.push(s);
            }
            if is_actionable(&set, &p, &roots).is_ok() {
                let norm = paths::lexical_normalize(&p);
                prop_assert!(paths::is_within(&norm, home.path()));
                prop_assert!(norm != home.path());
                prop_assert!(!set.check(&norm).is_hard());
                prop_assert!(!has_git_component(&norm));
            }
        }
    }
}
