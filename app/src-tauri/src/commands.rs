//! Tauri commands named after the core API (SPEC §3.4), one to one. They
//! return the schema types from `docs/schema/` as JSON. Long commands run off
//! the main thread and can be cancelled by their `op` id.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use kipple_core::model::*;
use kipple_core::platform::Env;
use kipple_core::{Kipple, OpenOptions};
use serde::Serialize;
use serde_json::Value;
use tauri::State;
use tauri::ipc::Channel;

/// Error sent to the frontend; `KippleError.from` in `@kipple/client` reads it.
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn io(message: String) -> Self {
        Self::new("io", message)
    }
}

impl From<kipple_core::Error> for CommandError {
    fn from(e: kipple_core::Error) -> Self {
        use kipple_core::Error as E;
        let code = match &e {
            E::NoScan => "no_scan",
            E::ItemNotFound(_) | E::AmbiguousItem(_) => "not_found",
            E::Invalid(_) | E::UnknownSelector(_) => "invalid",
            E::Config(_) => "config",
            _ => "error",
        };
        Self::new(code, e.to_string())
    }
}

type CmdResult<T = Value> = Result<T, CommandError>;

fn json<T: Serialize>(v: T) -> CmdResult {
    serde_json::to_value(v).map_err(|e| CommandError::new("error", e.to_string()))
}

/// Shared app state: the core (reopened after a config change) and running
/// operations.
#[derive(Default)]
pub struct AppState {
    core: Mutex<Option<Arc<Kipple>>>,
    ops: Mutex<HashMap<String, CancelToken>>,
}

fn open_core() -> Result<Kipple, String> {
    let mut integrations = kipple_packs::builtin().map_err(|e| e.to_string())?;
    integrations.extend(kipple_adapters::all());
    Kipple::open(OpenOptions::new(Env::from_process(), integrations)).map_err(|e| e.to_string())
}

impl AppState {
    fn core(&self) -> CmdResult<Arc<Kipple>> {
        let mut slot = self
            .core
            .lock()
            .map_err(|_| CommandError::new("error", "app state is unavailable"))?;
        if let Some(k) = slot.as_ref() {
            return Ok(Arc::clone(k));
        }
        let k = Arc::new(open_core().map_err(|m| CommandError::new("config", m))?);
        *slot = Some(Arc::clone(&k));
        Ok(k)
    }

    /// Drops the open core so the next command reads the new config.
    fn reload(&self) {
        if let Ok(mut slot) = self.core.lock() {
            *slot = None;
        }
    }

    fn start(&self, op: &str) -> CancelToken {
        let token = CancelToken::new();
        if let Ok(mut ops) = self.ops.lock() {
            ops.insert(op.to_owned(), token.clone());
        }
        token
    }

    fn finish(&self, op: &str) {
        if let Ok(mut ops) = self.ops.lock() {
            ops.remove(op);
        }
    }
}

struct ScanToChannel {
    channel: Channel<Value>,
    cancel: CancelToken,
}

impl ScanSink for ScanToChannel {
    fn event(&self, event: ScanEvent) {
        if let Ok(v) = serde_json::to_value(&event) {
            let _ = self.channel.send(v);
        }
    }

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}

struct ExecToChannel {
    channel: Channel<Value>,
    cancel: CancelToken,
}

impl ExecSink for ExecToChannel {
    fn event(&self, event: ExecEvent) {
        if let Ok(v) = serde_json::to_value(&event) {
            let _ = self.channel.send(v);
        }
    }

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}

#[tauri::command(async)]
pub fn integrations(state: State<'_, AppState>) -> CmdResult {
    json(state.core()?.integrations())
}

#[tauri::command(async)]
pub fn scan(
    state: State<'_, AppState>,
    req: ScanRequest,
    op: String,
    on_event: Channel<Value>,
) -> CmdResult {
    let k = state.core()?;
    let sink = ScanToChannel {
        channel: on_event,
        cancel: state.start(&op),
    };
    let result = k.scan(req, &sink);
    state.finish(&op);
    json(result?)
}

#[tauri::command(async)]
pub fn items(state: State<'_, AppState>, query: ItemQuery) -> CmdResult {
    json(state.core()?.items(query)?)
}

#[tauri::command(async)]
pub fn projects(state: State<'_, AppState>, query: ProjectQuery) -> CmdResult {
    json(state.core()?.projects(query)?)
}

#[tauri::command(async)]
pub fn explain(state: State<'_, AppState>, item: String) -> CmdResult {
    json(state.core()?.explain(&item)?)
}

#[tauri::command(async)]
pub fn plan(state: State<'_, AppState>, req: PlanRequest) -> CmdResult {
    json(state.core()?.plan(req)?)
}

/// Runs a plan. The core refuses it without the right confirmation (typing
/// `reclaim` for Review items), whatever the frontend sends.
#[tauri::command(async)]
pub fn execute(
    state: State<'_, AppState>,
    plan: Plan,
    confirm: Confirmation,
    op: String,
    on_event: Channel<Value>,
) -> CmdResult {
    let k = state.core()?;
    let sink = ExecToChannel {
        channel: on_event,
        cancel: state.start(&op),
    };
    let result = k.execute(&plan, confirm, &sink);
    state.finish(&op);
    json(result?)
}

#[tauri::command(async)]
pub fn quarantine(state: State<'_, AppState>) -> CmdResult {
    json(state.core()?.quarantine()?)
}

#[tauri::command(async)]
pub fn restore(state: State<'_, AppState>, entry: QuarantineId) -> CmdResult {
    json(state.core()?.restore(&entry, None)?)
}

#[tauri::command(async)]
pub fn purge(state: State<'_, AppState>, req: PurgeRequest) -> CmdResult {
    json(state.core()?.purge(req)?)
}

#[tauri::command(async)]
pub fn history(state: State<'_, AppState>, req: HistoryRequest) -> CmdResult {
    json(state.core()?.history(req)?)
}

#[tauri::command(async)]
pub fn doctor(state: State<'_, AppState>) -> CmdResult {
    json(state.core()?.doctor()?)
}

#[tauri::command(async)]
pub fn report(state: State<'_, AppState>) -> CmdResult {
    json(state.core()?.report()?)
}

/// Cancels a running scan or plan. A plan finishes its current item first.
#[tauri::command]
pub fn cancel(state: State<'_, AppState>, op: String) -> CmdResult<()> {
    if let Some(t) = state.ops.lock().ok().and_then(|ops| ops.get(&op).cloned()) {
        t.cancel();
    }
    Ok(())
}

// ---- App extras (not part of SPEC §3.4) --------------------------------------

/// The effective config (SPEC §10.3).
#[tauri::command(async)]
pub fn config(state: State<'_, AppState>) -> CmdResult {
    json(state.core()?.config())
}

/// Validates and saves the config, then reopens the core with it.
#[tauri::command(async)]
pub fn set_config(state: State<'_, AppState>, config: kipple_core::config::Config) -> CmdResult {
    state.core()?.save_config(&config)?;
    state.reload();
    json(state.core()?.config())
}

#[derive(Serialize)]
struct VolumeInfo {
    name: String,
    mount: String,
    capacity: u64,
    free: u64,
    home: String,
}

/// The volume that holds the home folder.
#[tauri::command(async)]
pub fn volume() -> CmdResult {
    let home = Env::from_process()
        .home()
        .ok_or_else(|| CommandError::io("home folder unknown".into()))?;
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk = disks
        .list()
        .iter()
        .filter(|d| home.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .ok_or_else(|| CommandError::io("no disk found for the home folder".into()))?;
    let name = disk.name().to_string_lossy().into_owned();
    json(VolumeInfo {
        name: if name.is_empty() {
            disk.mount_point().display().to_string()
        } else {
            name
        },
        mount: disk.mount_point().display().to_string(),
        capacity: disk.total_space(),
        free: disk.available_space(),
        home: home.display().to_string(),
    })
}

/// The action journal (SPEC §8.5).
#[tauri::command(async)]
pub fn journal(state: State<'_, AppState>) -> CmdResult {
    json(state.core()?.journal())
}

/// Shows a path in Finder, Explorer or the file manager. Read-only.
#[tauri::command]
pub fn reveal(path: String) -> CmdResult<()> {
    let p = std::path::Path::new(&path);
    if !p.is_absolute() || !p.exists() {
        return Err(CommandError::io(format!("{path} doesn't exist.")));
    }
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg("-R").arg(p).status();
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("explorer")
        .arg(format!("/select,{}", p.display()))
        .status();
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let status = std::process::Command::new("xdg-open")
        .arg(if p.is_dir() {
            p
        } else {
            p.parent().unwrap_or(p)
        })
        .status();
    status
        .map(|_| ())
        .map_err(|e| CommandError::io(format!("Could not open {path}: {e}")))
}
