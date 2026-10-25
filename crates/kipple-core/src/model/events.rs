//! Progress events and cancellation (SPEC §3.4).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ExecEntry, Item, PlanId, ScanResult};

/// Shared, cloneable cancellation flag.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ScanEvent {
    Started {
        roots: Vec<PathBuf>,
    },
    Progress {
        done: u64,
        total: Option<u64>,
        bytes: u64,
    },
    ItemFound {
        item: Box<Item>,
    },
    Warning {
        message: String,
    },
    Finished {
        result: Box<ScanResult>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ExecEvent {
    Started { plan_id: PlanId, entries: u64 },
    EntryStarted { index: u64, title: String },
    EntryFinished { index: u64, entry: Box<ExecEntry> },
    Warning { message: String },
    Finished,
}

/// Receives scan progress. Called from worker threads.
pub trait ScanSink: Sync {
    fn event(&self, event: ScanEvent);

    /// Polled regularly; returning a cancelled token stops the scan.
    fn cancel_token(&self) -> Option<&CancelToken> {
        None
    }
}

/// Receives execution progress.
pub trait ExecSink: Sync {
    fn event(&self, event: ExecEvent);

    fn cancel_token(&self) -> Option<&CancelToken> {
        None
    }
}

/// Discards events.
#[derive(Debug, Default, Clone)]
pub struct NoopSink {
    pub cancel: CancelToken,
}

impl ScanSink for NoopSink {
    fn event(&self, _event: ScanEvent) {}

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}

impl ExecSink for NoopSink {
    fn event(&self, _event: ExecEvent) {}

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}
