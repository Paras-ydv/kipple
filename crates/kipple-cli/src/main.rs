//! The `kipple` binary.

mod args;
mod cmd;
mod context;
mod output;

use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use kipple_core::platform::Env;

use crate::args::{Cli, Command};
use crate::context::Context;

/// Exit codes (SPEC §11.4).
pub mod exit {
    pub const ERROR: u8 = 1;
    pub const USAGE: u8 = 2;
    pub const PARTIAL: u8 = 3;
    pub const ABORTED: u8 = 4;
    pub const INTERRUPTED: u8 = 130;
}

fn init_tracing(verbose: u8, quiet: bool) {
    let level = match (quiet, verbose) {
        (true, _) => "error",
        (false, 0) => "warn",
        (false, 1) => "info",
        (false, 2) => "debug",
        _ => "trace",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_env("KIPPLE_LOG").unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(format!(
            "kipple={level},kipple_core={level},kipple_packs={level},kipple_adapters={level}"
        ))
    });
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.global.verbose, cli.global.quiet);
    let Some(command) = cli.command else {
        // The TUI (UI.md §4) opens here once it exists.
        let _ = Cli::command().print_help();
        return ExitCode::SUCCESS;
    };
    let mut ctx = match Context::new(cli.global, Env::from_process()) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(exit::ERROR);
        }
    };
    let result = match &command {
        Command::Scan => cmd::scan::scan(&mut ctx),
        Command::Report(a) => cmd::scan::report(&mut ctx, a),
        Command::Projects(a) => cmd::browse::projects(&mut ctx, a),
        Command::Items(a) => cmd::browse::items(&mut ctx, a),
        Command::Explain { item } => cmd::browse::explain(&mut ctx, item),
        Command::Packs(c) => cmd::packs::run(&mut ctx, c),
        Command::Config(c) => cmd::config::run(&ctx, c),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            if ctx.global.json {
                let msg = serde_json::json!({ "error": format!("{e:#}") });
                eprintln!("{msg}");
            } else {
                eprintln!("error: {e:#}");
            }
            ExitCode::from(exit::ERROR)
        }
    }
}
