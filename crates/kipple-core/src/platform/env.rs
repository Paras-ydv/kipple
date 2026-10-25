use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// A snapshot of environment variables.
///
/// The core never reads `std::env` directly outside [`Env::from_process`], so
/// tests can run in parallel with fully isolated environments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    vars: BTreeMap<OsString, OsString>,
}

impl Env {
    /// Captures the current process environment.
    pub fn from_process() -> Self {
        Self {
            vars: std::env::vars_os().collect(),
        }
    }

    /// An empty environment.
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_pairs<K, V, I>(pairs: I) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
        I: IntoIterator<Item = (K, V)>,
    {
        Self {
            vars: pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// Returns a non-empty value for `key`. Empty values count as unset.
    pub fn get(&self, key: impl AsRef<OsStr>) -> Option<&OsStr> {
        let key = key.as_ref();
        let value = self.vars.get(key).or_else(|| {
            // Windows variable names are case-insensitive.
            if cfg!(windows) {
                self.vars
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v)
            } else {
                None
            }
        })?;
        (!value.is_empty()).then_some(value.as_os_str())
    }

    pub fn get_str(&self, key: impl AsRef<OsStr>) -> Option<&str> {
        self.get(key).and_then(OsStr::to_str)
    }

    pub fn get_path(&self, key: impl AsRef<OsStr>) -> Option<PathBuf> {
        self.get(key).map(PathBuf::from)
    }

    pub fn set(&mut self, key: impl Into<OsString>, value: impl Into<OsString>) {
        self.vars.insert(key.into(), value.into());
    }

    pub fn remove(&mut self, key: impl AsRef<OsStr>) {
        self.vars.remove(key.as_ref());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.vars
            .iter()
            .map(|(k, v)| (k.as_os_str(), v.as_os_str()))
    }

    /// The user's home directory: `HOME`, then `USERPROFILE` on Windows.
    pub fn home(&self) -> Option<PathBuf> {
        let primary = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let fallback = if cfg!(windows) { "HOME" } else { "USERPROFILE" };
        self.get_path(primary).or_else(|| self.get_path(fallback))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_values_are_unset() {
        let env = Env::from_pairs([("A", ""), ("B", "x")]);
        assert_eq!(env.get("A"), None);
        assert_eq!(env.get_str("B"), Some("x"));
    }

    #[test]
    fn home_prefers_platform_variable() {
        let env = Env::from_pairs([("HOME", "/h"), ("USERPROFILE", "/u")]);
        let expected = if cfg!(windows) { "/u" } else { "/h" };
        assert_eq!(env.home(), Some(PathBuf::from(expected)));
    }
}
