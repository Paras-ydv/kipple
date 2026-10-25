//! Tauri commands named after the core API (SPEC §3.4), one to one.
//!
//! They return the schema types from `docs/schema/` as JSON. Until the app is
//! wired to `kipple-core`, every core command answers with the
//! `unavailable` error ("not available yet"); the frontend shows a notice and
//! the browser preview uses `MockClient` instead.

use serde::Serialize;
use serde_json::Value;
use tauri::ipc::Channel;

/// Error sent to the frontend; `KippleError.from` in `@kipple/client` reads it.
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    fn unavailable() -> Self {
        Self {
            code: "unavailable",
            message: "not available yet".to_owned(),
        }
    }

    fn io(message: String) -> Self {
        Self {
            code: "io",
            message,
        }
    }
}

type CmdResult<T = Value> = Result<T, CommandError>;

#[tauri::command]
pub fn integrations() -> CmdResult {
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn scan(req: Value, op: String, on_event: Channel<Value>) -> CmdResult {
    let _ = (req, op, on_event);
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn items(query: Value) -> CmdResult {
    let _ = query;
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn projects(query: Value) -> CmdResult {
    let _ = query;
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn explain(item: String) -> CmdResult {
    let _ = item;
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn plan(req: Value) -> CmdResult {
    let _ = req;
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn execute(plan: Value, confirm: Value, op: String, on_event: Channel<Value>) -> CmdResult {
    let _ = (plan, confirm, op, on_event);
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn quarantine() -> CmdResult {
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn restore(entry: String) -> CmdResult {
    let _ = entry;
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn purge(req: Value) -> CmdResult {
    let _ = req;
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn history(req: Value) -> CmdResult {
    let _ = req;
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn doctor() -> CmdResult {
    Err(CommandError::unavailable())
}

#[tauri::command]
pub fn report() -> CmdResult {
    Err(CommandError::unavailable())
}

/// Cancels a running `scan` or `execute` by its operation id (the core's `CancelToken`).
#[tauri::command]
pub fn cancel(op: String) -> CmdResult<()> {
    let _ = op;
    Err(CommandError::unavailable())
}

// ---- App extras (not part of SPEC §3.4) --------------------------------------

/// The volume that holds the home folder.
#[tauri::command]
pub fn volume() -> CmdResult {
    Err(CommandError::unavailable())
}

/// The action journal (SPEC §8.5).
#[tauri::command]
pub fn journal() -> CmdResult {
    Err(CommandError::unavailable())
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
