//! Platform layer (SPEC §16): environment, OS detection, paths, commands,
//! processes and trash.

pub mod command;
mod env;
pub mod fsx;
pub mod os;
pub mod paths;
pub mod process;
mod trash;

use std::sync::Arc;

pub use command::{CommandError, CommandOutput, CommandRunner};
pub use env::Env;
pub use process::{FixedProcesses, ProcessInfo, ProcessSnapshot, ProcessSource, SystemProcesses};
pub use trash::TrashBin;

use crate::model::{Os, PlatformInfo};

/// Everything platform-specific the core and integrations use.
#[derive(Debug, Clone)]
pub struct Platform {
    pub env: Env,
    pub os: Os,
    pub wsl: bool,
    pub runner: CommandRunner,
    pub processes: Arc<dyn ProcessSource>,
    pub trash: TrashBin,
}

impl Platform {
    /// Builds the platform from an environment snapshot. Inside a test
    /// sandbox (`KIPPLE_TEST_SANDBOX`), the trash is a directory in the
    /// sandbox and no real processes are listed.
    pub fn new(env: Env) -> Self {
        let sandbox = env.get_path("KIPPLE_TEST_SANDBOX");
        let trash = match &sandbox {
            Some(root) => TrashBin::Directory(root.join(".kipple-test-trash")),
            // The freedesktop trash lives under the home directory.
            None if cfg!(target_os = "linux") && env.home().is_none() => TrashBin::Unavailable,
            None => TrashBin::System,
        };
        let processes: Arc<dyn ProcessSource> = match sandbox {
            Some(_) => Arc::new(FixedProcesses::default()),
            None => Arc::new(SystemProcesses),
        };
        Self {
            os: os::current_os(),
            wsl: os::is_wsl(&env),
            runner: CommandRunner::new(env.clone()),
            env,
            processes,
            trash,
        }
    }

    pub fn with_processes(mut self, processes: Arc<dyn ProcessSource>) -> Self {
        self.processes = processes;
        self
    }

    pub fn with_trash(mut self, trash: TrashBin) -> Self {
        self.trash = trash;
        self
    }

    pub fn home(&self) -> Option<std::path::PathBuf> {
        self.env.home()
    }

    pub fn info(&self) -> PlatformInfo {
        PlatformInfo {
            os: self.os,
            arch: std::env::consts::ARCH.to_owned(),
            wsl: self.wsl,
            trash_available: self.trash.available(),
        }
    }
}
