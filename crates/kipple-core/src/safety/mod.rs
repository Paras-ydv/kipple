//! Safety engine (SPEC §7).

pub mod protected;
pub mod reasons;

pub use protected::{ProtectedSet, Protection, Refusal, is_actionable};
