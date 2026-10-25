//! Command-line arguments (SPEC §11).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Find and safely reclaim disk space used by developer tools.
#[derive(Debug, Parser)]
#[command(name = "kipple", version, about, max_term_width = 100)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Print machine-readable JSON. No colors, no prompts.
    #[arg(long, global = true)]
    pub json: bool,

    /// Scan this folder instead of your home folder. Repeatable.
    #[arg(long, global = true, value_name = "PATH")]
    pub root: Vec<PathBuf>,

    /// Only these integrations or categories. Repeatable.
    #[arg(long, global = true, value_name = "ID")]
    pub only: Vec<String>,

    /// Leave out paths matching this glob. Repeatable.
    #[arg(long, global = true, value_name = "GLOB")]
    pub exclude: Vec<String>,

    /// Only items last used longer ago than this (e.g. 30d, 6m, 1y).
    #[arg(long, global = true, value_name = "AGE")]
    pub older_than: Option<String>,

    /// Only items at least this big (e.g. 500MB, 2GB).
    #[arg(long, global = true, value_name = "SIZE")]
    pub min_size: Option<String>,

    /// Only items marked safe.
    #[arg(long, global = true)]
    pub safe_only: bool,

    /// Turn off colors.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Print less.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Print more. Repeat for debug output.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Use this config file.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show, check or locate the config file.
    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print the config file path.
    Path,
    /// Print the effective config.
    Show,
    /// Check the config file for errors.
    Validate,
    /// Open the config file in $EDITOR.
    Edit,
}
