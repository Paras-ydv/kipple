//! Spawns integration-defined commands: explicit argv, never a shell, with a
//! timeout and captured output (SPEC §16).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::platform::Env;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("empty command")]
    Empty,
    #[error("`{0}` is not installed or not on PATH")]
    NotFound(String),
    #[error("could not start `{program}`: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("`{program}` did not finish within {secs} seconds")]
    Timeout { program: String, secs: u64 },
    #[error("i/o error while running `{program}`: {source}")]
    Io {
        program: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Exit code; `None` when killed by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// Finds `program` on the `PATH` from `env`. Absolute and relative paths with
/// separators are returned as-is when they exist.
pub fn which(program: &str, env: &Env) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        return candidate.is_file().then(|| candidate.to_path_buf());
    }
    let path = env.get("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        env.get_str("PATHEXT")
            .unwrap_or(".COM;.EXE;.BAT;.CMD")
            .split(';')
            .map(|s| s.to_ascii_lowercase())
            .chain(std::iter::once(String::new()))
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in std::env::split_paths(path) {
        for ext in &exts {
            let full = dir.join(format!("{program}{ext}"));
            if is_executable(&full) {
                return Some(full);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Runs commands with the environment snapshot it was built from.
#[derive(Debug, Clone)]
pub struct CommandRunner {
    env: Env,
}

impl CommandRunner {
    pub fn new(env: Env) -> Self {
        Self { env }
    }

    pub fn which(&self, program: &str) -> Option<PathBuf> {
        which(program, &self.env)
    }

    /// Runs `argv` in `cwd` (default: home) and waits up to `timeout`.
    pub fn run(
        &self,
        argv: &[impl AsRef<str>],
        cwd: Option<&Path>,
        timeout: Duration,
    ) -> Result<CommandOutput, CommandError> {
        let (program, args) = argv.split_first().ok_or(CommandError::Empty)?;
        let program = program.as_ref();
        let resolved = self
            .which(program)
            .ok_or_else(|| CommandError::NotFound(program.to_owned()))?;
        let mut cmd = Command::new(&resolved);
        cmd.args(args.iter().map(AsRef::as_ref));
        cmd.env_clear();
        for (k, v) in self.env.iter() {
            cmd.env(k, v);
        }
        cmd.env("LC_ALL", "C").env("LANG", "C");
        if let Some(dir) = cwd.map(Path::to_path_buf).or_else(|| self.env.home()) {
            cmd.current_dir(dir);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        tracing::debug!(command = %display_argv(argv), "running command");
        let mut child = cmd.spawn().map_err(|source| CommandError::Spawn {
            program: program.to_owned(),
            source,
        })?;

        // Drain pipes on threads so a chatty child can't block on a full pipe.
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        let out_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(s) = stdout.as_mut() {
                let _ = s.read_to_end(&mut buf);
            }
            buf
        });
        let err_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(s) = stderr.as_mut() {
                let _ = s.read_to_end(&mut buf);
            }
            buf
        });

        let status = child
            .wait_timeout(timeout)
            .map_err(|source| CommandError::Io {
                program: program.to_owned(),
                source,
            })?;
        let status = match status {
            Some(status) => status,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CommandError::Timeout {
                    program: program.to_owned(),
                    secs: timeout.as_secs(),
                });
            }
        };
        let stdout = out_thread.join().unwrap_or_default();
        let stderr = err_thread.join().unwrap_or_default();
        Ok(CommandOutput {
            code: status.code(),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }

    /// Runs a command and returns trimmed stdout as a path if it succeeds.
    /// Used for `npm config get cache`-style resolution.
    pub fn output_path(&self, argv: &[impl AsRef<str>], timeout: Duration) -> Option<PathBuf> {
        let out = self.run(argv, None, timeout).ok()?;
        if !out.success() {
            return None;
        }
        let line = out.stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
        let path = PathBuf::from(line);
        path.is_absolute().then_some(path)
    }
}

/// Converts args for display, quoting those with spaces.
pub fn display_argv(argv: &[impl AsRef<str>]) -> String {
    argv.iter()
        .map(|a| {
            let a = a.as_ref();
            if a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::testing::TestHome;

    #[test]
    fn runs_mock_command_from_sandbox_path() {
        let home = TestHome::new().unwrap();
        home.mock_command("hello", "echo \"hi $1\"").unwrap();
        let runner = CommandRunner::new(home.env().clone());
        let out = runner
            .run(&["hello", "there"], None, Duration::from_secs(5))
            .unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "hi there");
    }

    #[test]
    fn missing_program_is_not_found() {
        let home = TestHome::new().unwrap();
        let runner = CommandRunner::new(home.env().clone());
        let err = runner
            .run(&["npm"], None, Duration::from_secs(5))
            .unwrap_err();
        assert!(matches!(err, CommandError::NotFound(_)));
    }

    #[test]
    fn times_out() {
        let home = TestHome::new().unwrap();
        home.mock_command("slow", "sleep 5").unwrap();
        let runner = CommandRunner::new(home.env().clone());
        let err = runner
            .run(&["slow"], None, Duration::from_millis(200))
            .unwrap_err();
        assert!(matches!(err, CommandError::Timeout { .. }));
    }

    #[test]
    fn output_path_reads_first_line() {
        let home = TestHome::new().unwrap();
        home.mock_command("tool", "echo /some/cache").unwrap();
        let runner = CommandRunner::new(home.env().clone());
        assert_eq!(
            runner.output_path(&["tool"], Duration::from_secs(5)),
            Some(PathBuf::from("/some/cache"))
        );
    }
}
