//! kipple core: model, scanner, attributor, safety, planner, executor, index,
//! config and platform layer (SPEC §3).

pub mod attribute;
pub mod config;
pub mod dirs;
mod doctor;
mod exec;
pub mod format;
pub mod index;
pub mod integration;
pub mod journal;
mod kipple;
pub mod model;
pub mod platform;
pub mod quarantine;
pub mod safety;
pub mod scan;

#[cfg(any(test, feature = "test-support"))]
pub mod testing;

pub use exec::{PLAN_TTL_SECS, build_plan};
pub use kipple::{
    Error, Kipple, OpenOptions, Result, default_action_text, filter_items, find_in, summarize,
};
pub use model::*;

/// kipple's version, recorded in scans and JSON output.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
