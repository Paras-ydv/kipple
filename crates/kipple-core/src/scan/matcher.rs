//! Matches walked directories against artifact rules and recognises project
//! markers (SPEC §5.2, §5.4).

use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::integration::ArtifactRule;

/// Cache Directory Tagging Specification signature.
pub const CACHEDIR_SIGNATURE: &[u8] = b"Signature: 8a477f597d28d172789f06886806bc55";

/// Whether `dir/CACHEDIR.TAG` exists and starts with the signature.
pub fn has_valid_cachedir_tag(dir: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(dir.join("CACHEDIR.TAG")) else {
        return false;
    };
    let mut buf = [0u8; 43];
    file.read_exact(&mut buf).is_ok() && buf == CACHEDIR_SIGNATURE
}

fn is_glob(s: &str) -> bool {
    s.contains(['*', '?', '[', '{'])
}

fn glob(pattern: &str) -> Result<Glob, globset::Error> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .case_insensitive(cfg!(windows))
        .build()
}

/// Checks a name list where entries may be globs.
struct NameList {
    exact: Vec<String>,
    globs: Option<GlobSet>,
}

impl NameList {
    fn new(names: &[String]) -> Result<Self, globset::Error> {
        let mut exact = Vec::new();
        let mut builder = GlobSetBuilder::new();
        let mut any_glob = false;
        for n in names {
            if is_glob(n) {
                builder.add(glob(n)?);
                any_glob = true;
            } else {
                exact.push(n.clone());
            }
        }
        Ok(Self {
            exact,
            globs: if any_glob {
                Some(builder.build()?)
            } else {
                None
            },
        })
    }

    fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.globs.is_none()
    }

    /// Which entries exist in `dir`. Exact names are stat'ed; globs need a
    /// directory listing.
    fn present_in(&self, dir: &Path) -> Vec<bool> {
        let mut found: Vec<bool> = self
            .exact
            .iter()
            .map(|n| dir.join(n).symlink_metadata().is_ok())
            .collect();
        if let Some(globs) = &self.globs {
            let mut any = false;
            if let Ok(rd) = std::fs::read_dir(dir) {
                for entry in rd.flatten() {
                    if globs.is_match(entry.file_name()) {
                        any = true;
                        break;
                    }
                }
            }
            found.push(any);
        }
        found
    }

    fn any(&self, dir: &Path) -> bool {
        self.present_in(dir).into_iter().any(|b| b)
    }

    fn all(&self, dir: &Path) -> bool {
        self.present_in(dir).into_iter().all(|b| b)
    }
}

struct CompiledRule {
    rule: ArtifactRule,
    sibling_any: NameList,
    sibling_all: NameList,
    inside_any: NameList,
}

/// All artifact rules, compiled.
pub struct Matcher {
    rules: Vec<CompiledRule>,
    names: GlobSet,
    gitignore: GitignoreCache,
}

impl Matcher {
    pub fn new(mut rules: Vec<ArtifactRule>) -> Result<Self, globset::Error> {
        rules.sort_by_key(|r| r.priority);
        let mut builder = GlobSetBuilder::new();
        let mut compiled = Vec::with_capacity(rules.len());
        for rule in rules {
            builder.add(glob(&rule.dir)?);
            compiled.push(CompiledRule {
                sibling_any: NameList::new(&rule.requires_sibling_any)?,
                sibling_all: NameList::new(&rule.requires_sibling_all)?,
                inside_any: NameList::new(&rule.requires_inside_any)?,
                rule,
            });
        }
        Ok(Self {
            rules: compiled,
            names: builder.build()?,
            gitignore: GitignoreCache::default(),
        })
    }

    pub fn rule(&self, idx: usize) -> &ArtifactRule {
        &self.rules[idx].rule
    }

    /// The first rule matching directory `path`, by priority.
    pub fn match_dir(&self, path: &Path) -> Option<usize> {
        let name = path.file_name()?;
        // Insertion order is priority order.
        let mut candidates = self.names.matches(name);
        candidates.sort_unstable();
        candidates
            .into_iter()
            .find(|&idx| self.check(&self.rules[idx], path))
    }

    fn check(&self, c: &CompiledRule, path: &Path) -> bool {
        let parent = path.parent().unwrap_or(path);
        if !c.sibling_any.is_empty() && !c.sibling_any.any(parent) {
            return false;
        }
        if !c.sibling_all.is_empty() && !c.sibling_all.all(parent) {
            return false;
        }
        if !c.inside_any.is_empty() && !c.inside_any.any(path) {
            return false;
        }
        if c.rule.requires_cachedir_tag && !has_valid_cachedir_tag(path) {
            return false;
        }
        if c.rule.requires_gitignored && !self.gitignore.is_ignored(path) {
            return false;
        }
        true
    }
}

/// Answers "does git ignore this directory?" from `.gitignore` files and
/// `.git/info/exclude`, without running git.
#[derive(Default)]
pub struct GitignoreCache {
    dirs: Mutex<HashMap<PathBuf, Option<Arc<Gitignore>>>>,
    git_roots: Mutex<HashMap<PathBuf, Option<PathBuf>>>,
}

impl GitignoreCache {
    fn git_root(&self, dir: &Path) -> Option<PathBuf> {
        if let Some(hit) = self.git_roots.lock().ok()?.get(dir) {
            return hit.clone();
        }
        let found = if dir.join(".git").symlink_metadata().is_ok() {
            Some(dir.to_path_buf())
        } else {
            dir.parent().and_then(|p| self.git_root(p))
        };
        if let Ok(mut map) = self.git_roots.lock() {
            map.insert(dir.to_path_buf(), found.clone());
        }
        found
    }

    fn matcher_for(&self, dir: &Path, git_root: &Path) -> Option<Arc<Gitignore>> {
        if let Some(hit) = self.dirs.lock().ok()?.get(dir) {
            return hit.clone();
        }
        let mut builder = GitignoreBuilder::new(dir);
        let mut any = false;
        let gi = dir.join(".gitignore");
        if gi.is_file() {
            any = true;
            let _ = builder.add(gi);
        }
        if dir == git_root {
            let exclude = dir.join(".git/info/exclude");
            if exclude.is_file() {
                any = true;
                let _ = builder.add(exclude);
            }
        }
        let built = if any {
            builder.build().ok().map(Arc::new)
        } else {
            None
        };
        if let Ok(mut map) = self.dirs.lock() {
            map.insert(dir.to_path_buf(), built.clone());
        }
        built
    }

    /// `path` is a directory. Not inside a git repo → not ignored.
    pub fn is_ignored(&self, path: &Path) -> bool {
        let Some(parent) = path.parent() else {
            return false;
        };
        let Some(root) = self.git_root(parent) else {
            return false;
        };
        // Matchers from the deepest directory up to the git root; the first
        // decisive match wins, as in git.
        for dir in parent.ancestors() {
            if let Some(gi) = self.matcher_for(dir, &root) {
                let m = gi.matched_path_or_any_parents(path, true);
                if m.is_ignore() {
                    return true;
                }
                if m.is_whitelist() {
                    return false;
                }
            }
            if dir == root {
                break;
            }
        }
        false
    }
}

/// Project marker files and directories (SPEC §5.4).
pub struct Markers {
    exact: Vec<&'static str>,
    globs: GlobSet,
    glob_names: Vec<&'static str>,
}

const MARKER_FILES: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "pyproject.toml",
    "setup.py",
    "requirements.txt",
    "Pipfile",
    "go.mod",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "pom.xml",
    "Package.swift",
    "pubspec.yaml",
    "CMakeLists.txt",
    "mix.exs",
    "stack.yaml",
    "build.sbt",
    "Gemfile",
    "composer.json",
    "deno.json",
    "deno.jsonc",
    "main.tf",
    "MODULE.bazel",
    "WORKSPACE",
    "build.zig",
    ".git",
];

const MARKER_GLOBS: &[&str] = &[
    "*.xcodeproj",
    "*.xcworkspace",
    "*.csproj",
    "*.sln",
    "*.cabal",
    "*.uproject",
];

/// What a walked entry says about projects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerHit {
    /// The project root the marker belongs to.
    pub root: PathBuf,
    pub marker: String,
}

impl Default for Markers {
    fn default() -> Self {
        let mut b = GlobSetBuilder::new();
        for g in MARKER_GLOBS {
            if let Ok(g) = glob(g) {
                b.add(g);
            }
        }
        Self {
            exact: MARKER_FILES.to_vec(),
            globs: b.build().unwrap_or_else(|_| GlobSet::empty()),
            glob_names: MARKER_GLOBS.to_vec(),
        }
    }
}

impl Markers {
    /// Checks one walked entry (file or directory).
    pub fn hit(&self, path: &Path) -> Option<MarkerHit> {
        let name = path.file_name()?;
        let parent = path.parent()?;
        if let Some(m) = self.exact.iter().find(|m| OsStr::new(m) == name) {
            return Some(MarkerHit {
                root: parent.to_path_buf(),
                marker: (*m).to_owned(),
            });
        }
        if let Some(idx) = self.globs.matches(name).first() {
            return Some(MarkerHit {
                root: parent.to_path_buf(),
                marker: self.glob_names[*idx].to_owned(),
            });
        }
        // Unity: ProjectSettings/ProjectVersion.txt
        if name == "ProjectVersion.txt" && parent.file_name() == Some(OsStr::new("ProjectSettings"))
        {
            return Some(MarkerHit {
                root: parent.parent()?.to_path_buf(),
                marker: "ProjectSettings/ProjectVersion.txt".to_owned(),
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::ItemDraft;
    use crate::model::{IntegrationId, Locator};

    fn rule(dir: &str) -> ArtifactRule {
        ArtifactRule {
            integration: IntegrationId::from("t"),
            dir: dir.into(),
            requires_sibling_any: vec![],
            requires_sibling_all: vec![],
            requires_inside_any: vec![],
            requires_gitignored: false,
            requires_cachedir_tag: false,
            template: ItemDraft::new("k", "t", Locator::path("/")),
            priority: 0,
        }
    }

    fn mk(root: &Path, files: &[&str]) {
        for f in files {
            let p = root.join(f);
            if f.ends_with('/') {
                std::fs::create_dir_all(&p).unwrap();
            } else {
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(p, "x").unwrap();
            }
        }
    }

    #[test]
    fn sibling_and_inside_rules() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        mk(
            root,
            &[
                "a/package.json",
                "a/node_modules/",
                "b/node_modules/",
                "v/.venv/pyvenv.cfg",
                "w/env/",
            ],
        );
        let mut nm = rule("node_modules");
        nm.requires_sibling_any = vec!["package.json".into()];
        let mut venv = rule("*");
        venv.requires_inside_any = vec!["pyvenv.cfg".into()];
        venv.priority = 100;
        let m = Matcher::new(vec![venv, nm]).unwrap();
        assert_eq!(
            m.match_dir(&root.join("a/node_modules"))
                .map(|i| m.rule(i).dir.clone()),
            Some("node_modules".into())
        );
        assert_eq!(m.match_dir(&root.join("b/node_modules")), None);
        assert_eq!(
            m.match_dir(&root.join("v/.venv"))
                .map(|i| m.rule(i).dir.clone()),
            Some("*".into())
        );
        assert_eq!(m.match_dir(&root.join("w/env")), None);
    }

    #[test]
    fn sibling_globs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        mk(root, &["a/App.csproj", "a/bin/", "b/bin/"]);
        let mut r = rule("bin");
        r.requires_sibling_any = vec!["*.csproj".into()];
        let m = Matcher::new(vec![r]).unwrap();
        assert!(m.match_dir(&root.join("a/bin")).is_some());
        assert!(m.match_dir(&root.join("b/bin")).is_none());
    }

    #[test]
    fn cachedir_tag_requires_signature() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("good")).unwrap();
        std::fs::create_dir_all(root.join("bad")).unwrap();
        std::fs::write(
            root.join("good/CACHEDIR.TAG"),
            "Signature: 8a477f597d28d172789f06886806bc55\n# comment",
        )
        .unwrap();
        std::fs::write(root.join("bad/CACHEDIR.TAG"), "nope").unwrap();
        assert!(has_valid_cachedir_tag(&root.join("good")));
        assert!(!has_valid_cachedir_tag(&root.join("bad")));
        assert!(!has_valid_cachedir_tag(&root.join("missing")));
    }

    #[test]
    fn gitignored_rule() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        mk(
            root,
            &[
                "repo/.git/",
                "repo/.gitignore",
                "repo/web/package.json",
                "repo/web/dist/",
                "repo/web/build/",
                "repo/sub/.gitignore",
                "repo/sub/package.json",
                "repo/sub/out/",
                "nogit/package.json",
                "nogit/dist/",
            ],
        );
        std::fs::write(root.join("repo/.gitignore"), "dist/\n").unwrap();
        std::fs::write(root.join("repo/sub/.gitignore"), "/out\n").unwrap();
        let cache = GitignoreCache::default();
        assert!(cache.is_ignored(&root.join("repo/web/dist")));
        assert!(!cache.is_ignored(&root.join("repo/web/build")));
        assert!(cache.is_ignored(&root.join("repo/sub/out")));
        assert!(!cache.is_ignored(&root.join("nogit/dist")));
    }

    #[test]
    fn markers() {
        let m = Markers::default();
        let hit = m.hit(Path::new("/p/app/package.json")).unwrap();
        assert_eq!(hit.root, PathBuf::from("/p/app"));
        assert_eq!(
            m.hit(Path::new("/p/ios/App.xcodeproj")).unwrap().marker,
            "*.xcodeproj"
        );
        assert_eq!(
            m.hit(Path::new("/p/game/ProjectSettings/ProjectVersion.txt"))
                .unwrap()
                .root,
            PathBuf::from("/p/game")
        );
        assert!(m.hit(Path::new("/p/readme.md")).is_none());
    }
}
