//! kipple core: model, scanner, attributor, safety, planner, executor, index,
//! config and platform layer (SPEC §3).

pub mod platform;

#[cfg(any(test, feature = "test-support"))]
pub mod testing;
