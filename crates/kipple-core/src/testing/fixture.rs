//! Synthetic fixture trees described by `fixtures/<id>/fixture.toml`
//! (SPEC §18). Generated content only — never real user data.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use serde::Deserialize;

use super::TestHome;

/// A fixture manifest.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    /// Age applied to every created file and folder unless overridden.
    #[serde(default = "default_days")]
    pub days_ago: u64,
    #[serde(default, rename = "file")]
    pub files: Vec<FileSpec>,
    #[serde(default, rename = "dir")]
    pub dirs: Vec<DirSpec>,
    #[serde(default, rename = "hardlink")]
    pub hardlinks: Vec<LinkSpec>,
    #[serde(default, rename = "symlink")]
    pub symlinks: Vec<LinkSpec>,
    #[serde(default, rename = "git")]
    pub git: Vec<GitSpec>,
    #[serde(default, rename = "cachedir_tag")]
    pub cachedir_tags: Vec<DirSpec>,
    /// Environment variables to set, as paths relative to the test home.
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
}

fn default_days() -> u64 {
    60
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpec {
    /// Relative to the test home.
    pub path: String,
    #[serde(default)]
    pub content: Option<String>,
    /// Filler bytes when `content` is not given.
    #[serde(default)]
    pub size: Option<u64>,
    /// `{i}` in the path is replaced by 0..count.
    #[serde(default)]
    pub count: Option<u64>,
    #[serde(default)]
    pub days_ago: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirSpec {
    pub path: String,
    #[serde(default)]
    pub days_ago: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkSpec {
    pub path: String,
    pub target: String,
}

/// A git repository with one commit made `days_ago` days ago.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitSpec {
    pub path: String,
    pub days_ago: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("bad fixture: {0}")]
    Parse(String),
    #[error("git failed: {0}")]
    Git(String),
}

fn ago(days: u64) -> SystemTime {
    SystemTime::now() - Duration::from_secs(days * 86_400)
}

fn set_age(path: &Path, days: u64) -> std::io::Result<()> {
    let t = filetime::FileTime::from_system_time(ago(days));
    filetime::set_symlink_file_times(path, t, t)
}

fn ensure_parent(p: &Path) -> std::io::Result<()> {
    match p.parent() {
        Some(parent) => std::fs::create_dir_all(parent),
        None => Ok(()),
    }
}

impl Fixture {
    pub fn parse(text: &str) -> Result<Self, FixtureError> {
        toml::from_str(text).map_err(|e| FixtureError::Parse(e.to_string()))
    }

    pub fn load(path: &Path) -> Result<Self, FixtureError> {
        Self::parse(&std::fs::read_to_string(path)?)
    }

    /// Loads `fixtures/<id>/fixture.toml` from the workspace.
    pub fn named(id: &str) -> Result<Self, FixtureError> {
        Self::load(&fixtures_dir().join(id).join("fixture.toml"))
    }

    /// Creates the tree inside `home` and applies `env`.
    pub fn generate(&self, home: &mut TestHome) -> Result<(), FixtureError> {
        let root = home.path().to_path_buf();
        for (k, v) in &self.env {
            let value = root.join(v).into_os_string();
            home.env_mut().set(k.clone(), value);
        }
        let root = root.as_path();
        let mut aged: Vec<(PathBuf, u64)> = Vec::new();
        for d in &self.dirs {
            let p = root.join(&d.path);
            std::fs::create_dir_all(&p)?;
            aged.push((p, d.days_ago.unwrap_or(self.days_ago)));
        }
        for f in &self.files {
            for i in 0..f.count.unwrap_or(1) {
                let p = root.join(f.path.replace("{i}", &i.to_string()));
                ensure_parent(&p)?;
                match (&f.content, f.size) {
                    (Some(c), _) => std::fs::write(&p, c)?,
                    (None, Some(n)) => std::fs::write(&p, vec![b'k'; n as usize])?,
                    (None, None) => std::fs::write(&p, b"")?,
                }
                aged.push((p, f.days_ago.unwrap_or(self.days_ago)));
            }
        }
        for t in &self.cachedir_tags {
            let p = root.join(&t.path).join("CACHEDIR.TAG");
            ensure_parent(&p)?;
            std::fs::write(
                &p,
                "Signature: 8a477f597d28d172789f06886806bc55\n# kipple test fixture\n",
            )?;
            aged.push((p, t.days_ago.unwrap_or(self.days_ago)));
        }
        for l in &self.hardlinks {
            let p = root.join(&l.path);
            ensure_parent(&p)?;
            std::fs::hard_link(root.join(&l.target), &p)?;
        }
        for l in &self.symlinks {
            let p = root.join(&l.path);
            ensure_parent(&p)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(root.join(&l.target), &p)?;
            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(root.join(&l.target), &p)?;
        }
        for g in &self.git {
            make_repo(&root.join(&g.path), g.days_ago, home)?;
            aged.push((root.join(&g.path).join(".git"), g.days_ago));
        }
        // Age everything deepest-first so creating children doesn't bump a
        // parent afterwards.
        let mut all: Vec<(PathBuf, u64)> = Vec::new();
        collect(root, root, &aged, self.days_ago, &mut all);
        all.sort_by_key(|(p, _)| std::cmp::Reverse(p.components().count()));
        for (p, days) in all {
            let _ = set_age(&p, days);
        }
        Ok(())
    }
}

/// Every path under `dir` with the age of its nearest configured ancestor.
fn collect(
    root: &Path,
    dir: &Path,
    aged: &[(PathBuf, u64)],
    default: u64,
    out: &mut Vec<(PathBuf, u64)>,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if dir == root && (e.file_name() == "bin" || e.file_name() == "tmp") {
            continue;
        }
        let days = aged
            .iter()
            .filter(|(a, _)| p.starts_with(a))
            .max_by_key(|(a, _)| a.components().count())
            .map(|(_, d)| *d)
            .unwrap_or(default);
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            collect(root, &p, aged, default, out);
        }
        out.push((p, days));
    }
}

fn make_repo(dir: &Path, days_ago: u64, home: &TestHome) -> Result<(), FixtureError> {
    std::fs::create_dir_all(dir)?;
    let date = format!(
        "@{} +0000",
        ago(days_ago)
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let run = |args: &[&str]| -> Result<(), FixtureError> {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("HOME", home.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", home.path().join(".gitconfig-fixture"))
            .env("GIT_AUTHOR_NAME", "kipple fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@kipple.invalid")
            .env("GIT_COMMITTER_NAME", "kipple fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@kipple.invalid")
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .map_err(|e| FixtureError::Git(e.to_string()))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(FixtureError::Git(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ))
        }
    };
    run(&["init", "-q", "-b", "main"])?;
    run(&["add", "-A"])?;
    run(&["commit", "-q", "--allow-empty", "-m", "fixture"])?;
    Ok(())
}

/// `<workspace>/fixtures`.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_files_dirs_and_ages() {
        let mut home = TestHome::new().unwrap();
        let f = Fixture::parse(
            r#"
days_ago = 30
[[file]]
path = "app/package.json"
content = "{}"
[[file]]
path = "app/node_modules/p{i}/index.js"
size = 100
count = 3
[[file]]
path = "app/src/new.js"
days_ago = 1
[[cachedir_tag]]
path = "app/.cache"
"#,
        )
        .unwrap();
        f.generate(&mut home).unwrap();
        assert!(home.join("app/node_modules/p2/index.js").is_file());
        assert!(home.join("app/.cache/CACHEDIR.TAG").is_file());
        let age = |p: &str| {
            SystemTime::now()
                .duration_since(home.join(p).metadata().unwrap().modified().unwrap())
                .unwrap()
                .as_secs()
                / 86_400
        };
        assert_eq!(age("app/package.json"), 30);
        assert_eq!(age("app/node_modules"), 30);
        assert_eq!(age("app/src/new.js"), 1);
    }

    #[test]
    fn makes_git_repos_with_dated_commits() {
        let mut home = TestHome::new().unwrap();
        let f = Fixture::parse(
            "[[file]]\npath = \"repo/a.txt\"\ncontent = \"x\"\n[[git]]\npath = \"repo\"\ndays_ago = 50\n",
        )
        .unwrap();
        f.generate(&mut home).unwrap();
        let t = crate::attribute::git_head_time(&home.join("repo")).unwrap();
        let days = (crate::model::Timestamp::now().as_second() - t.as_second()) / 86_400;
        assert_eq!(days, 50);
    }
}
