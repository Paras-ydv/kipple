//! kipple core: model, scanner, attributor, safety, planner, executor, index,
//! config and platform layer (SPEC §3).

pub mod config;
pub mod dirs;
pub mod index;
pub mod model;
pub mod platform;
pub mod safety;

#[cfg(any(test, feature = "test-support"))]
pub mod testing;

pub use model::*;

/// kipple's version, recorded in scans and JSON output.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
