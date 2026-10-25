//! Project roots found during the walk and attribution of items to them
//! (SPEC §5.4, §6).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::model::{ProjectId, ProjectRef};
use crate::platform::paths;

/// Collects marker hits from walker threads.
#[derive(Default)]
pub struct ProjectCollector {
    roots: Mutex<BTreeMap<PathBuf, BTreeSet<String>>>,
}

impl ProjectCollector {
    pub fn add(&self, root: PathBuf, marker: String) {
        if let Ok(mut roots) = self.roots.lock() {
            roots.entry(root).or_default().insert(marker);
        }
    }

    pub fn finish(self) -> ProjectSet {
        ProjectSet::new(self.roots.into_inner().unwrap_or_default())
    }
}

#[derive(Debug, Clone)]
pub struct ProjectRoot {
    pub root: PathBuf,
    pub markers: BTreeSet<String>,
    pub name: String,
}

impl ProjectRoot {
    pub fn has_git(&self) -> bool {
        self.markers.contains(".git")
    }

    pub fn reference(&self) -> ProjectRef {
        ProjectRef {
            id: ProjectId::for_root(&self.root),
            root: self.root.clone(),
            name: self.name.clone(),
        }
    }
}

/// Where an item belongs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribution {
    /// The project shown in the Projects view (outermost git root).
    pub project: ProjectRef,
    /// The nearest marker root, when it differs (monorepo package).
    pub subproject: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectSet {
    roots: BTreeMap<PathBuf, ProjectRoot>,
}

impl ProjectSet {
    pub fn new(found: BTreeMap<PathBuf, BTreeSet<String>>) -> Self {
        let roots = found
            .into_iter()
            .map(|(root, markers)| {
                let name = project_name(&root, &markers);
                (
                    root.clone(),
                    ProjectRoot {
                        root,
                        markers,
                        name,
                    },
                )
            })
            .collect();
        Self { roots }
    }

    pub fn get(&self, root: &Path) -> Option<&ProjectRoot> {
        self.roots.get(root)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ProjectRoot> {
        self.roots.values()
    }

    /// Adds a project known only from tool metadata (e.g. an AI session's
    /// `cwd`). Markers are read from disk if the folder exists.
    pub fn ensure(&mut self, root: &Path) -> &ProjectRoot {
        let root = paths::lexical_normalize(root);
        self.roots.entry(root.clone()).or_insert_with(|| {
            let markers = read_markers(&root);
            let name = project_name(&root, &markers);
            ProjectRoot {
                root,
                markers,
                name,
            }
        })
    }

    /// Nearest root at or above `path`.
    pub fn nearest(&self, path: &Path) -> Option<&ProjectRoot> {
        path.ancestors().find_map(|a| self.roots.get(a))
    }

    /// Nearest root strictly above `path` (for artifacts, which sit inside
    /// their project).
    pub fn enclosing(&self, path: &Path) -> Option<&ProjectRoot> {
        path.parent().and_then(|p| self.nearest(p))
    }

    /// Outermost git root at or above `root`, else `root` itself.
    fn rollup<'a>(&'a self, root: &'a ProjectRoot) -> &'a ProjectRoot {
        let mut best = None;
        for a in root.root.ancestors() {
            if let Some(r) = self.roots.get(a)
                && r.has_git()
            {
                best = Some(r);
            }
        }
        best.unwrap_or(root)
    }

    /// Attributes an item at `path`. `inside` = the path is inside the
    /// project (artifacts) rather than being the project folder itself.
    pub fn attribute(&self, path: &Path, inside: bool) -> Option<Attribution> {
        let nearest = if inside {
            self.enclosing(path)
        } else {
            self.nearest(path)
        }?;
        let top = self.rollup(nearest);
        Some(Attribution {
            project: top.reference(),
            subproject: (top.root != nearest.root).then(|| nearest.root.clone()),
        })
    }
}

fn read_markers(root: &Path) -> BTreeSet<String> {
    let markers = super::matcher::Markers::default();
    let mut found = BTreeSet::new();
    if let Ok(rd) = std::fs::read_dir(root) {
        for entry in rd.flatten() {
            if let Some(hit) = markers.hit(&entry.path()) {
                found.insert(hit.marker);
            }
        }
    }
    found
}

/// Package name if known, else the folder name.
pub fn project_name(root: &Path, markers: &BTreeSet<String>) -> String {
    let from_json = || -> Option<String> {
        let text = std::fs::read_to_string(root.join("package.json")).ok()?;
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        v.get("name")?.as_str().map(str::to_owned)
    };
    let from_toml = |file: &str, table: &str| -> Option<String> {
        let text = std::fs::read_to_string(root.join(file)).ok()?;
        let v: toml::Table = text.parse().ok()?;
        v.get(table)?.get("name")?.as_str().map(str::to_owned)
    };
    let name = if markers.contains("package.json") {
        from_json()
    } else if markers.contains("Cargo.toml") {
        from_toml("Cargo.toml", "package")
    } else if markers.contains("pyproject.toml") {
        from_toml("pyproject.toml", "project")
    } else {
        None
    };
    name.filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| folder_name(root))
}

pub fn folder_name(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(entries: &[(&str, &[&str])]) -> ProjectSet {
        let c = ProjectCollector::default();
        for (root, markers) in entries {
            for m in *markers {
                c.add(PathBuf::from(root), (*m).to_owned());
            }
        }
        c.finish()
    }

    #[test]
    fn rolls_up_to_outermost_git_root() {
        let s = set(&[
            ("/w/mono", &[".git", "package.json"]),
            ("/w/mono/packages/a", &["package.json"]),
            ("/w/solo", &["Cargo.toml"]),
        ]);
        let a = s
            .attribute(Path::new("/w/mono/packages/a/node_modules"), true)
            .unwrap();
        assert_eq!(a.project.root, PathBuf::from("/w/mono"));
        assert_eq!(a.subproject, Some(PathBuf::from("/w/mono/packages/a")));
        let b = s.attribute(Path::new("/w/solo/target"), true).unwrap();
        assert_eq!(b.project.root, PathBuf::from("/w/solo"));
        assert_eq!(b.subproject, None);
        assert!(s.attribute(Path::new("/elsewhere/x"), true).is_none());
    }

    #[test]
    fn artifact_at_root_level_is_not_its_own_project() {
        // A venv that itself contains a marker file must still belong to the
        // project around it.
        let s = set(&[
            ("/w/app", &["pyproject.toml"]),
            ("/w/app/.venv", &["setup.py"]),
        ]);
        let a = s.attribute(Path::new("/w/app/.venv"), true).unwrap();
        assert_eq!(a.project.root, PathBuf::from("/w/app"));
    }

    #[test]
    fn reads_package_names() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"@acme/web"}"#).unwrap();
        let markers = BTreeSet::from(["package.json".to_owned()]);
        assert_eq!(project_name(dir.path(), &markers), "@acme/web");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"tool\"\n",
        )
        .unwrap();
        let markers = BTreeSet::from(["Cargo.toml".to_owned()]);
        assert_eq!(project_name(dir.path(), &markers), "tool");
    }
}
