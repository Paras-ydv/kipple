//! Test helpers. Enabled for this crate's tests and, for other crates, via the
//! `test-support` feature.
//!
//! [`TestHome`] creates an isolated home directory and an [`Env`] pointing
//! every variable kipple or an integration reads into it, so no test can reach
//! the real home directory or real tool data (SPEC §18).

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::platform::Env;

mod fixture;

pub use fixture::{Fixture, FixtureError, fixtures_dir};

/// Environment variables that [`TestHome`] points into its temp directory.
/// Each maps to a path relative to the temp home.
const SANDBOXED_VARS: &[(&str, &str)] = &[
    ("HOME", ""),
    ("USERPROFILE", ""),
    ("XDG_CONFIG_HOME", ".config"),
    ("XDG_DATA_HOME", ".local/share"),
    ("XDG_CACHE_HOME", ".cache"),
    ("XDG_STATE_HOME", ".local/state"),
    ("APPDATA", "AppData/Roaming"),
    ("LOCALAPPDATA", "AppData/Local"),
    ("TMPDIR", "tmp"),
    ("TEMP", "tmp"),
    ("TMP", "tmp"),
    ("KIPPLE_CONFIG", ".config/kipple/config.toml"),
    ("KIPPLE_DATA_DIR", ".local/share/kipple"),
    ("CODEX_HOME", ".codex"),
    ("DOCKER_CONFIG", ".docker"),
    ("CLAUDE_CONFIG_DIR", ".claude"),
    ("ANDROID_HOME", "Android/Sdk"),
    ("ANDROID_SDK_ROOT", "Android/Sdk"),
    ("ANDROID_USER_HOME", ".android"),
    ("ANDROID_AVD_HOME", ".android/avd"),
    ("CARGO_HOME", ".cargo"),
    ("GRADLE_USER_HOME", ".gradle"),
    ("OLLAMA_MODELS", ".ollama/models"),
    ("HF_HOME", ".cache/huggingface"),
    ("PLAYWRIGHT_BROWSERS_PATH", ".cache/ms-playwright"),
    ("PUPPETEER_CACHE_DIR", ".cache/puppeteer"),
];

/// Variables that are explicitly unset so a test never follows them to real
/// data on the developer's machine.
const CLEARED_VARS: &[&str] = &[
    "HF_HUB_CACHE",
    "GOPATH",
    "GOCACHE",
    "GOMODCACHE",
    "npm_config_cache",
    "PIP_CACHE_DIR",
    "UV_CACHE_DIR",
    "BUN_INSTALL_CACHE_DIR",
    "WSL_DISTRO_NAME",
];

/// An isolated home directory for tests. Deleted on drop.
pub struct TestHome {
    dir: tempfile::TempDir,
    root: PathBuf,
    env: Env,
}

impl TestHome {
    /// Creates a temp home with an empty `bin/` that is the only `PATH`
    /// entry, so real tools (`npm`, `docker`, `codex`, …) are never run.
    pub fn new() -> io::Result<Self> {
        let dir = tempfile::Builder::new().prefix("kipple-test-").tempdir()?;
        // Canonical root so paths compare equal to what the scanner reports
        // (macOS `/var` → `/private/var`).
        let root = dir.path().canonicalize()?;
        let mut env = Env::empty();
        for (key, rel) in SANDBOXED_VARS {
            let path = if rel.is_empty() {
                root.clone()
            } else {
                root.join(rel)
            };
            env.set(*key, path.into_os_string());
        }
        for key in CLEARED_VARS {
            env.remove(key);
        }
        let bin = root.join("bin");
        fs::create_dir_all(&bin)?;
        fs::create_dir_all(root.join("tmp"))?;
        env.set("PATH", bin.clone().into_os_string());
        // A socket that doesn't exist: tests never reach a real Docker daemon.
        env.set(
            "DOCKER_HOST",
            format!("unix://{}", root.join("docker.sock").display()),
        );
        // Marks the sandbox; debug builds refuse filesystem changes outside it.
        env.set("KIPPLE_TEST_SANDBOX", root.clone().into_os_string());
        env.set("NO_COLOR", "1");
        Ok(Self { dir, root, env })
    }

    /// The temp home directory (canonical).
    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn join(&self, rel: impl AsRef<Path>) -> PathBuf {
        self.root.join(rel)
    }

    /// Directory on the sandboxed `PATH` where tests place mock binaries.
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn env_mut(&mut self) -> &mut Env {
        &mut self.env
    }

    /// Writes `contents` to `rel`, creating parent directories.
    pub fn write(&self, rel: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> io::Result<PathBuf> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        Ok(path)
    }

    pub fn mkdir(&self, rel: impl AsRef<Path>) -> io::Result<PathBuf> {
        let path = self.root.join(rel);
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Installs an executable mock script named `name` on the sandboxed PATH.
    /// `script` is a POSIX shell body on Unix and a batch body on Windows.
    pub fn mock_command(&self, name: &str, script: &str) -> io::Result<PathBuf> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = self.bin_dir().join(name);
            // System dirs are appended inside the script only, so mocks can use
            // `sleep`, `cat`, … while kipple itself still sees just `bin/`.
            fs::write(
                &path,
                format!("#!/bin/sh\nPATH=\"$PATH:/usr/bin:/bin\"\n{script}\n"),
            )?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
            Ok(path)
        }
        #[cfg(windows)]
        {
            let path = self.bin_dir().join(format!("{name}.cmd"));
            fs::write(&path, format!("@echo off\r\n{script}\r\n"))?;
            Ok(path)
        }
    }

    /// Applies the sandboxed environment to a child process, clearing
    /// everything else.
    pub fn apply(&self, cmd: &mut Command) {
        cmd.env_clear();
        for (k, v) in self.env.iter() {
            cmd.env(k, v);
        }
        // Windows needs these to start most processes.
        #[cfg(windows)]
        for key in ["SystemRoot", "SYSTEMROOT", "ComSpec", "PATHEXT"] {
            if let Some(v) = std::env::var_os(key) {
                cmd.env(key, v);
            }
        }
    }

    /// Variables as owned pairs, e.g. for `assert_cmd`.
    pub fn env_pairs(&self) -> Vec<(OsString, OsString)> {
        self.env
            .iter()
            .map(|(k, v)| (k.to_os_string(), v.to_os_string()))
            .collect()
    }

    /// Keeps the directory after the test (for debugging).
    pub fn keep(self) -> PathBuf {
        self.dir.keep()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sandboxed_var_points_inside_the_temp_home() {
        let home = TestHome::new().unwrap();
        for (key, _) in SANDBOXED_VARS {
            let value = home.env().get_path(key).unwrap();
            assert!(value.starts_with(home.path()), "{key} escapes the sandbox");
        }
        assert_eq!(home.env().home().as_deref(), Some(home.path()));
        assert_eq!(home.env().get_path("PATH"), Some(home.bin_dir()));
    }

    #[test]
    fn cleared_vars_are_unset() {
        let home = TestHome::new().unwrap();
        for key in CLEARED_VARS {
            assert!(home.env().get(key).is_none(), "{key} leaked");
        }
    }

    #[test]
    fn temp_home_is_removed_on_drop() {
        let home = TestHome::new().unwrap();
        let path = home.path().to_path_buf();
        home.write("a/b.txt", "x").unwrap();
        drop(home);
        assert!(!path.exists());
    }
}
