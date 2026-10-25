use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Category, LastUsed, ProjectId, Safety, Size};

/// Reclaimable bytes per safety level.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SafetyMix {
    pub safe: u64,
    pub review: u64,
    pub keep: u64,
}

impl SafetyMix {
    pub fn add(&mut self, safety: Safety, bytes: u64) {
        match safety {
            Safety::Safe => self.safe += bytes,
            Safety::Review => self.review += bytes,
            Safety::Keep => self.keep += bytes,
        }
    }

    pub fn get(&self, safety: Safety) -> u64 {
        match safety {
            Safety::Safe => self.safe,
            Safety::Review => self.review,
            Safety::Keep => self.keep,
        }
    }

    pub fn total(&self) -> u64 {
        self.safe + self.review + self.keep
    }
}

/// A source-code directory and the items attributed to it (SPEC §4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub root: PathBuf,
    /// Package name if known, else folder name.
    pub name: String,
    /// Marker files found at the root: `package.json`, `.git`, …
    pub markers: Vec<String>,
    pub last_used: Option<LastUsed>,
    /// Sum of attributed items.
    pub total: Size,
    /// Reclaimable bytes per category.
    pub by_category: BTreeMap<Category, u64>,
    pub safety_mix: SafetyMix,
    /// Number of attributed items.
    #[serde(default)]
    pub item_count: u64,
    /// `false` when tool metadata points to a folder that no longer exists.
    pub exists: bool,
}
