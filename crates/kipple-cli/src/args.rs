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
    /// Scan and print a summary. Nothing is changed.
    Scan,
    /// Print the latest scan's report again without scanning.
    Report(ReportArgs),
    /// Projects and what they hold.
    Projects(ProjectsArgs),
    /// Every item, with ids for `explain` and `clean`.
    Items(ItemsArgs),
    /// Everything about one item.
    Explain {
        /// Item id (or its first characters) or path.
        item: String,
    },
    /// Make a plan, confirm it, and reclaim the space.
    Clean(CleanArgs),
    /// Put back everything the last clean moved to quarantine.
    Undo,
    /// List, restore or delete quarantined items.
    #[command(subcommand)]
    Quarantine(QuarantineCommand),
    /// What kipple has done, or how space grew over time.
    History(HistoryArgs),
    /// Check what kipple can see and how to fix what it can't.
    Doctor,
    /// Print shell completions.
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Inspect and test packs.
    #[command(subcommand)]
    Packs(PacksCommand),
    /// Show, check or locate the config file.
    #[command(subcommand)]
    Config(ConfigCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ReportBy {
    Category,
    Project,
    Integration,
}

#[derive(Debug, Args)]
pub struct ReportArgs {
    /// Group the table by this.
    #[arg(long, value_enum, default_value_t = ReportBy::Category)]
    pub by: ReportBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SortKey {
    Size,
    Age,
    Name,
}

#[derive(Debug, Args)]
pub struct ProjectsArgs {
    /// Only projects whose folder no longer exists.
    #[arg(long)]
    pub orphaned: bool,
    #[arg(long, value_enum, default_value_t = SortKey::Size)]
    pub sort: SortKey,
    /// Show at most this many.
    #[arg(long)]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct ItemsArgs {
    #[arg(long, value_enum, default_value_t = SortKey::Size)]
    pub sort: SortKey,
    /// Show at most this many.
    #[arg(long)]
    pub limit: Option<usize>,
    /// Include items marked keep.
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Subcommand)]
pub enum PacksCommand {
    /// List built-in packs.
    List,
    /// Print a built-in pack.
    Show { id: String },
    /// Check pack files for errors.
    Validate {
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Scan with only this integration and list what it finds.
    Test { id: String },
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

#[derive(Debug, Args)]
pub struct CleanArgs {
    /// Item ids (or their first characters) or paths. Combine with filters.
    pub items: Vec<String>,
    /// Write the plan and print its id without doing anything.
    #[arg(long, conflicts_with = "plan")]
    pub plan_only: bool,
    /// Run a plan saved with --plan-only.
    #[arg(long, value_name = "ID")]
    pub plan: Option<String>,
    /// Leave this item out of the plan. Repeatable.
    #[arg(long, value_name = "ID")]
    pub skip: Vec<String>,
    /// Don't ask. Only allowed together with --safe-only.
    #[arg(short, long)]
    pub yes: bool,
    /// Delete quarantined items right away instead of keeping them for undo.
    #[arg(long)]
    pub purge_now: bool,
}

#[derive(Debug, Subcommand)]
pub enum QuarantineCommand {
    /// What's in quarantine.
    List,
    /// Put one item back.
    Restore {
        id: String,
        /// Restore into this folder instead of the original place.
        #[arg(long, value_name = "FOLDER")]
        to: Option<PathBuf>,
    },
    /// Delete quarantined items for good: expired ones, or those older than
    /// --older-than.
    Purge {
        /// Everything in quarantine.
        #[arg(long)]
        all: bool,
        /// Don't ask.
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Show how reclaimable space grew per category.
    #[arg(long)]
    pub growth: bool,
    /// Show at most this many entries.
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}
