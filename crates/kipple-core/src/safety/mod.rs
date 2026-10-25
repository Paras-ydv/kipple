//! Safety engine (SPEC §7).

pub mod engine;
pub mod protected;
pub mod reasons;

pub use protected::{ProtectedSet, Protection, Refusal, is_actionable};
