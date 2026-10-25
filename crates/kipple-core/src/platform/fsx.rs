//! Filesystem helpers: identity, volumes, sandbox guard.

use std::fs::Metadata;
use std::io;
use std::path::Path;

use crate::model::Identity;
use crate::platform::{Env, paths};

/// Identity of a path, without following a symlink at `path`.
pub fn identity(path: &Path) -> io::Result<Identity> {
    let meta = path.symlink_metadata()?;
    Ok(identity_of(&meta))
}

#[cfg(unix)]
pub fn identity_of(meta: &Metadata) -> Identity {
    use std::os::unix::fs::MetadataExt;
    Identity::Inode {
        dev: meta.dev(),
        ino: meta.ino(),
    }
}

#[cfg(not(unix))]
pub fn identity_of(meta: &Metadata) -> Identity {
    let nanos = meta
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i128)
        .unwrap_or(0);
    Identity::Created { nanos }
}

/// Device id of the nearest existing ancestor of `path`.
#[cfg(unix)]
pub fn device_of(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    path.ancestors()
        .find_map(|p| p.symlink_metadata().ok())
        .map(|m| m.dev())
}

/// Whether `a` and `b` live on the same volume (so `rename` works).
#[cfg(unix)]
pub fn same_volume(a: &Path, b: &Path) -> bool {
    matches!((device_of(a), device_of(b)), (Some(x), Some(y)) if x == y)
}

#[cfg(windows)]
pub fn same_volume(a: &Path, b: &Path) -> bool {
    use std::path::Component;
    let prefix = |p: &Path| match p.components().next() {
        Some(Component::Prefix(pre)) => Some(pre.as_os_str().to_ascii_lowercase()),
        _ => None,
    };
    matches!((prefix(a), prefix(b)), (Some(x), Some(y)) if x == y)
}

#[cfg(not(any(unix, windows)))]
pub fn same_volume(_a: &Path, _b: &Path) -> bool {
    false
}

#[derive(Debug, thiserror::Error)]
#[error("refusing to change {0} outside the test sandbox")]
pub struct SandboxViolation(pub String);

/// Opt-in that lets a development build change real data.
pub const ALLOW_REAL_DATA: &str = "KIPPLE_ALLOW_REAL_DATA";

/// Whether this process may change anything outside a test sandbox.
///
/// Development builds (`cargo test`, `cargo run`, `tauri dev`) never change
/// real data unless `KIPPLE_ALLOW_REAL_DATA=1`; they only act inside a
/// `KIPPLE_TEST_SANDBOX`. Release builds may (after the user confirms a plan).
pub fn may_change_real_data(env: &Env) -> bool {
    !cfg!(debug_assertions) || env.get_str(ALLOW_REAL_DATA) == Some("1")
}

/// Refuses any change at all when there is no sandbox and real data is off
/// limits. Called before every action, including ones that don't touch the
/// filesystem directly (native commands, Docker removals).
pub fn change_guard(env: &Env) -> Result<(), SandboxViolation> {
    if env.get("KIPPLE_TEST_SANDBOX").is_some() || may_change_real_data(env) {
        Ok(())
    } else {
        Err(SandboxViolation(
            "real data: development builds of kipple only change files inside a test sandbox"
                .into(),
        ))
    }
}

/// Refuses filesystem changes outside `KIPPLE_TEST_SANDBOX` when it is set,
/// and any change to real data from a development build. Every destructive
/// operation calls this first (SPEC §18).
pub fn sandbox_guard(env: &Env, path: &Path) -> Result<(), SandboxViolation> {
    let Some(root) = env.get_path("KIPPLE_TEST_SANDBOX") else {
        return change_guard(env);
    };
    let resolved = paths::canonicalize_parent(path);
    if paths::is_within(&resolved, &root) || paths::is_within(path, &root) {
        Ok(())
    } else {
        Err(SandboxViolation(path.display().to_string()))
    }
}

/// Seconds since `meta` was last modified, if the clock allows.
pub fn modified_age_secs(meta: &Metadata) -> Option<u64> {
    let modified = meta.modified().ok()?;
    std::time::SystemTime::now()
        .duration_since(modified)
        .ok()
        .map(|d| d.as_secs())
}

/// Allocated bytes for a file (`st_blocks * 512` on Unix).
#[cfg(unix)]
pub fn allocated(meta: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

/// Windows on-disk sizes come in v1 (`GetCompressedFileSizeW`); until then
/// the length rounded up to 4 KiB clusters.
#[cfg(not(unix))]
pub fn allocated(meta: &Metadata) -> u64 {
    meta.len().div_ceil(4096) * 4096
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TestHome;

    #[test]
    fn development_builds_never_touch_real_data() {
        // No sandbox: a debug build (this test) refuses every change.
        let env = Env::from_pairs([("HOME", "/h")]);
        let p = std::env::temp_dir().join("kipple-real");
        if cfg!(debug_assertions) {
            assert!(change_guard(&env).is_err());
            assert!(sandbox_guard(&env, &p).is_err());
            let allowed = Env::from_pairs([("HOME", "/h"), (ALLOW_REAL_DATA, "1")]);
            assert!(change_guard(&allowed).is_ok());
            let wrong = Env::from_pairs([("HOME", "/h"), (ALLOW_REAL_DATA, "yes")]);
            assert!(change_guard(&wrong).is_err(), "only exactly 1 opts in");
        }
    }

    #[test]
    fn sandbox_guard_rejects_outside_paths() {
        let home = TestHome::new().unwrap();
        assert!(sandbox_guard(home.env(), &home.join("x/y")).is_ok());
        let outside = std::env::temp_dir().join("kipple-not-sandbox");
        assert!(sandbox_guard(home.env(), &outside).is_err());
        assert!(sandbox_guard(home.env(), &home.join("../escape")).is_err());
    }
}
