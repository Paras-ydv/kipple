//! Helpers shared by adapters.

use std::path::{Path, PathBuf};

use kipple_core::model::{
    Category, IntegrationId, IntegrationInfo, IntegrationKind, IntegrationStatus, LastUsed,
    LastUsedSource, Os, Timestamp,
};
use kipple_core::platform::Env;

/// Builds an adapter's static info.
pub fn info(
    id: &str,
    name: &str,
    category: Category,
    description: &str,
    status: IntegrationStatus,
    os: &[Os],
    in_use: &[&str],
) -> IntegrationInfo {
    IntegrationInfo {
        id: IntegrationId::from(id),
        name: name.to_owned(),
        kind: IntegrationKind::Adapter,
        category,
        description: description.to_owned(),
        docs_url: None,
        status,
        os: os.to_vec(),
        in_use_processes: in_use.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// First set env var (as a path), else `default` joined to home.
pub fn env_or_home(env: &Env, vars: &[&str], default: &str) -> Option<PathBuf> {
    vars.iter()
        .find_map(|v| env.get_path(v))
        .or_else(|| env.home().map(|h| h.join(default)))
}

/// Modification time as a last-used value.
pub fn mtime_used(path: &Path) -> Option<LastUsed> {
    let t = path.symlink_metadata().ok()?.modified().ok()?;
    Some(LastUsed {
        at: Timestamp::try_from(t).ok()?,
        source: LastUsedSource::ItemMtime,
    })
}

/// Sorted children of a directory (empty if unreadable).
pub fn children(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    out.sort();
    out
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `1 file`, `3 files`.
pub fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}
