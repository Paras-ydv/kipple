pub mod browse;
pub mod clean;
pub mod config;
pub mod doctor;
pub mod packs;
pub mod scan;
pub mod share;

use std::process::ExitCode;

use anyhow::Result;
use kipple_core::model::ScanRequest;

use crate::context::Context;

/// Opens the TUI; scans first when asked or when there's no scan yet.
pub fn tui(ctx: &mut Context, scan_first: bool) -> Result<ExitCode> {
    let k = ctx.open()?;
    let opts = crate::tui::Options {
        theme: ctx.theme(),
        units: k.config().ui.units,
        home: ctx.home(),
        scan_first,
        request: ScanRequest {
            roots: ctx.global.root.clone(),
            only: ctx.global.only.clone(),
            exclude: ctx.global.exclude.clone(),
            follow_symlinks: None,
        },
    };
    crate::tui::run(k, opts)?;
    Ok(ExitCode::SUCCESS)
}
