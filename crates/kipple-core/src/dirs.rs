//! kipple's own directories (SPEC §10.1). Resolved from the injected
//! environment rather than the `directories` crate so tests can sandbox them;
//! the layout matches what `directories` would return for
//! `ProjectDirs::from("dev", "kipple", "kipple")`.

use std::path::{Path, PathBuf};

use crate::model::Os;
use crate::platform::Env;

#[derive(Debug, thiserror::Error)]
#[error("cannot find the home directory; set HOME (or USERPROFILE on Windows)")]
pub struct NoHomeDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
}

impl Dirs {
    pub fn resolve(env: &Env, os: Os) -> Result<Self, NoHomeDir> {
        let home = env.home().ok_or(NoHomeDir)?;
        let config_file = match env.get_path("KIPPLE_CONFIG") {
            Some(p) => p,
            None => default_config_file(env, os, &home),
        };
        let data_dir = match env.get_path("KIPPLE_DATA_DIR") {
            Some(p) => p,
            None => default_data_dir(env, os, &home),
        };
        Ok(Self {
            config_file,
            data_dir,
        })
    }

    pub fn index_db(&self) -> PathBuf {
        self.data_dir.join("index.sqlite")
    }

    pub fn journal(&self) -> PathBuf {
        self.data_dir.join("journal.jsonl")
    }

    pub fn plans_dir(&self) -> PathBuf {
        self.data_dir.join("plans")
    }

    pub fn quarantine_dir(&self) -> PathBuf {
        self.data_dir.join("quarantine")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    pub fn config_dir(&self) -> Option<&Path> {
        self.config_file.parent()
    }
}

fn xdg(env: &Env, var: &str, home: &Path, default: &str) -> PathBuf {
    env.get_path(var)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(default))
}

fn default_config_file(env: &Env, os: Os, home: &Path) -> PathBuf {
    match os {
        Os::Macos => {
            let xdg_file = xdg(env, "XDG_CONFIG_HOME", home, ".config")
                .join("kipple")
                .join("config.toml");
            if xdg_file.exists() {
                xdg_file
            } else {
                mac_support(home).join("config.toml")
            }
        }
        Os::Windows => windows_dir(env, "APPDATA", home, "AppData/Roaming")
            .join("kipple")
            .join("config")
            .join("config.toml"),
        Os::Linux | Os::Wsl => xdg(env, "XDG_CONFIG_HOME", home, ".config")
            .join("kipple")
            .join("config.toml"),
    }
}

fn default_data_dir(env: &Env, os: Os, home: &Path) -> PathBuf {
    match os {
        Os::Macos => mac_support(home),
        Os::Windows => windows_dir(env, "LOCALAPPDATA", home, "AppData/Local")
            .join("kipple")
            .join("data"),
        Os::Linux | Os::Wsl => xdg(env, "XDG_DATA_HOME", home, ".local/share").join("kipple"),
    }
}

fn mac_support(home: &Path) -> PathBuf {
    home.join("Library")
        .join("Application Support")
        .join("dev.kipple.kipple")
}

fn windows_dir(env: &Env, var: &str, home: &Path, default: &str) -> PathBuf {
    env.get_path(var).unwrap_or_else(|| home.join(default))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_uses_xdg() {
        let env = Env::from_pairs([("HOME", "/h"), ("XDG_DATA_HOME", "/d")]);
        let dirs = Dirs::resolve(&env, Os::Linux).unwrap();
        assert_eq!(
            dirs.config_file,
            PathBuf::from("/h/.config/kipple/config.toml")
        );
        assert_eq!(dirs.data_dir, PathBuf::from("/d/kipple"));
    }

    #[test]
    fn env_overrides_win() {
        let env = Env::from_pairs([
            ("HOME", "/h"),
            ("USERPROFILE", "/h"),
            ("KIPPLE_CONFIG", "/c.toml"),
            ("KIPPLE_DATA_DIR", "/data"),
        ]);
        for os in [Os::Linux, Os::Macos, Os::Windows] {
            let dirs = Dirs::resolve(&env, os).unwrap();
            assert_eq!(dirs.config_file, PathBuf::from("/c.toml"));
            assert_eq!(dirs.data_dir, PathBuf::from("/data"));
        }
    }

    #[test]
    fn macos_defaults_to_application_support() {
        let env = Env::from_pairs([("HOME", "/nonexistent-home")]);
        let dirs = Dirs::resolve(&env, Os::Macos).unwrap();
        assert_eq!(
            dirs.data_dir,
            PathBuf::from("/nonexistent-home/Library/Application Support/dev.kipple.kipple")
        );
        assert!(dirs.config_file.ends_with("dev.kipple.kipple/config.toml"));
    }
}
