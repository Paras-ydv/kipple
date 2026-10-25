use std::path::Path;

use crate::model::Os;
use crate::platform::Env;

/// The OS kipple was built for.
pub fn current_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::Macos
    } else if cfg!(windows) {
        Os::Windows
    } else {
        Os::Linux
    }
}

/// Linux running under WSL (SPEC §16).
pub fn is_wsl(env: &Env) -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    env.get("WSL_DISTRO_NAME").is_some()
        || (env.get("KIPPLE_TEST_SANDBOX").is_none()
            && Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists())
}

/// Whether an integration restricted to `oses` applies here.
pub fn os_matches(oses: &[Os], current: Os, wsl: bool) -> bool {
    oses.is_empty() || oses.contains(&current) || (wsl && oses.contains(&Os::Wsl))
}

pub fn os_key(os: Os) -> &'static str {
    match os {
        Os::Macos => "macos",
        Os::Linux => "linux",
        Os::Windows => "windows",
        Os::Wsl => "wsl",
    }
}
