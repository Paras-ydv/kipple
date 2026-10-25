//! Rust adapters, one module per integration (SPEC §9.2).

pub mod util;

use std::sync::Arc;

use kipple_core::integration::Adapter;

/// Every built-in adapter.
pub fn all() -> Vec<Arc<dyn Adapter>> {
    vec![]
}
