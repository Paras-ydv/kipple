//! Process list for in-use checks (SPEC §7.2).

use std::sync::Arc;

use globset::{GlobBuilder, GlobMatcher};

/// One running process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub args: Vec<String>,
}

/// Matches processes by name glob and an optional argument substring.
///
/// Syntax: `name-glob` or `name-glob|arg-substring`, e.g. `qemu-system-*` or
/// `java|GradleDaemon`. Names compare case-insensitively and ignore a `.exe`
/// suffix.
#[derive(Debug, Clone)]
pub struct ProcessPattern {
    raw: String,
    name: GlobMatcher,
    arg: Option<String>,
}

impl ProcessPattern {
    pub fn parse(raw: &str) -> Result<Self, globset::Error> {
        let (name, arg) = match raw.split_once('|') {
            Some((n, a)) => (n, Some(a.to_owned())),
            None => (raw, None),
        };
        let name = GlobBuilder::new(name)
            .case_insensitive(true)
            .literal_separator(false)
            .build()?
            .compile_matcher();
        Ok(Self {
            raw: raw.to_owned(),
            name,
            arg,
        })
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The process name part, for user-facing messages.
    pub fn display_name(&self) -> &str {
        self.raw.split('|').next().unwrap_or(&self.raw)
    }

    pub fn matches(&self, p: &ProcessInfo) -> bool {
        let name = p.name.strip_suffix(".exe").unwrap_or(&p.name);
        if !self.name.is_match(name) {
            return false;
        }
        match &self.arg {
            Some(arg) => p.args.iter().any(|a| a.contains(arg.as_str())),
            None => true,
        }
    }
}

/// A point-in-time list of processes.
#[derive(Debug, Clone, Default)]
pub struct ProcessSnapshot {
    processes: Vec<ProcessInfo>,
}

impl ProcessSnapshot {
    pub fn new(processes: Vec<ProcessInfo>) -> Self {
        Self { processes }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ProcessInfo> {
        self.processes.iter()
    }

    /// Processes matching `pattern`. Invalid patterns match nothing.
    pub fn matching(&self, pattern: &str) -> Vec<&ProcessInfo> {
        match ProcessPattern::parse(pattern) {
            Ok(p) => self
                .processes
                .iter()
                .filter(|proc| p.matches(proc))
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn any_running(&self, pattern: &str) -> bool {
        !self.matching(pattern).is_empty()
    }
}

/// Where process lists come from. Tests inject a fixed list.
pub trait ProcessSource: Send + Sync + std::fmt::Debug {
    fn snapshot(&self) -> ProcessSnapshot;
}

/// Real processes via `sysinfo`.
#[derive(Debug, Default)]
pub struct SystemProcesses;

impl ProcessSource for SystemProcesses {
    fn snapshot(&self) -> ProcessSnapshot {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
        let mut sys = System::new();
        sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always),
        );
        let processes = sys
            .processes()
            .iter()
            .map(|(pid, p)| ProcessInfo {
                pid: pid.as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                args: p
                    .cmd()
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect(),
            })
            .collect();
        ProcessSnapshot::new(processes)
    }
}

/// A fixed list, for tests.
#[derive(Debug, Default, Clone)]
pub struct FixedProcesses(pub Arc<std::sync::Mutex<Vec<ProcessInfo>>>);

impl FixedProcesses {
    pub fn new(list: Vec<ProcessInfo>) -> Self {
        Self(Arc::new(std::sync::Mutex::new(list)))
    }

    pub fn set(&self, list: Vec<ProcessInfo>) {
        if let Ok(mut guard) = self.0.lock() {
            *guard = list;
        }
    }
}

impl ProcessSource for FixedProcesses {
    fn snapshot(&self) -> ProcessSnapshot {
        ProcessSnapshot::new(self.0.lock().map(|g| g.clone()).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(name: &str, args: &[&str]) -> ProcessInfo {
        ProcessInfo {
            pid: 1,
            name: name.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn matches_name_glob_and_args() {
        let snap = ProcessSnapshot::new(vec![
            proc("qemu-system-aarch64", &["-avd", "Pixel"]),
            proc(
                "java",
                &["org.gradle.launcher.daemon.bootstrap.GradleDaemon"],
            ),
            proc("Cursor.exe", &[]),
        ]);
        assert!(snap.any_running("qemu-system-*"));
        assert!(snap.any_running("java|GradleDaemon"));
        assert!(!snap.any_running("java|Kotlin"));
        assert!(snap.any_running("cursor"));
        assert!(!snap.any_running("codex"));
    }
}
