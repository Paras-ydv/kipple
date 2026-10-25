//! The `kipple` binary.

use std::process::ExitCode;

use clap::Parser;

/// Find and safely reclaim disk space used by developer tools.
#[derive(Debug, Parser)]
#[command(name = "kipple", version, about)]
struct Cli {}

fn main() -> ExitCode {
    let _cli = Cli::parse();
    ExitCode::SUCCESS
}
