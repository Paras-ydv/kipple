//! Request and response types of the public API (SPEC §3.4).

use std::collections::BTreeMap;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    ActionKind, Category, IntegrationId, Item, ItemId, Outcome, PlanId, ProjectId, ProjectSummary,
    QuarantineId, Safety, SafetyMix, ScanId, Size, Timestamp,
};

/// Current JSON schema version (SPEC §4.4).
pub const SCHEMA_VERSION: u32 = 1;

/// Wrapper for every JSON document kipple prints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Envelope<T> {
    pub schema_version: u32,
    pub kipple_version: String,
    pub data: T,
}

impl<T> Envelope<T> {
    pub fn new(data: T) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            kipple_version: crate::VERSION.to_owned(),
            data,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationKind {
    Pack,
    Adapter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum IntegrationStatus {
    #[default]
    Stable,
    /// Paths not yet verified on every OS; items capped at Review.
    Experimental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Macos,
    Linux,
    Windows,
    Wsl,
}

/// Static description of an integration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IntegrationInfo {
    pub id: IntegrationId,
    pub name: String,
    pub kind: IntegrationKind,
    pub category: Category,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    #[serde(default)]
    pub status: IntegrationStatus,
    /// Empty means every OS.
    #[serde(default)]
    pub os: Vec<Os>,
    /// Process names that make this integration's data in use.
    #[serde(default)]
    pub in_use_processes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Detection {
    NotPresent,
    Present {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
    Unsupported {
        reason: String,
    },
}

impl Detection {
    pub fn present() -> Self {
        Detection::Present { version: None }
    }

    pub fn is_present(&self) -> bool {
        matches!(self, Detection::Present { .. })
    }
}

/// An integration and whether it was detected on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IntegrationStatusReport {
    #[serde(flatten)]
    pub info: IntegrationInfo,
    pub enabled: bool,
    pub detection: Detection,
}

/// Selects items. Shared by `items`, `plan`, the CLI and MCP.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ItemFilter {
    /// Integration ids or category ids.
    pub only: Vec<String>,
    /// Glob patterns matched against item paths.
    pub exclude: Vec<String>,
    /// Item ids (or unique prefixes).
    pub ids: Vec<String>,
    /// Only items under these roots.
    pub roots: Vec<PathBuf>,
    /// Last used at least this many seconds ago.
    pub older_than_secs: Option<u64>,
    /// Reclaimable at least this many bytes.
    pub min_size: Option<u64>,
    pub safe_only: bool,
    /// Restrict to these levels. Empty means all.
    pub safety: Vec<Safety>,
    pub project: Option<ProjectId>,
}

impl ItemFilter {
    /// Whether the filter selects anything narrower than "everything".
    pub fn is_selective(&self) -> bool {
        !self.only.is_empty()
            || !self.ids.is_empty()
            || !self.roots.is_empty()
            || self.older_than_secs.is_some()
            || self.min_size.is_some()
            || self.safe_only
            || !self.safety.is_empty()
            || self.project.is_some()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ItemSort {
    #[default]
    Size,
    Age,
    Name,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ItemQuery {
    pub filter: ItemFilter,
    pub sort: ItemSort,
    pub limit: Option<usize>,
    /// Include `Keep` items.
    pub include_keep: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProjectSort {
    #[default]
    Size,
    Age,
    Name,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ProjectQuery {
    pub orphaned: bool,
    pub sort: ProjectSort,
    pub limit: Option<usize>,
    pub min_size: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ScanRequest {
    /// Replaces the configured roots when non-empty.
    pub roots: Vec<PathBuf>,
    /// Integration or category ids. Empty means all enabled.
    pub only: Vec<String>,
    /// Extra exclude globs.
    pub exclude: Vec<String>,
    pub follow_symlinks: Option<bool>,
}

/// One row of the per-category summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CategorySummary {
    pub category: Category,
    pub items: u64,
    pub reclaimable: u64,
    pub by_safety: SafetyMix,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ScanInfo {
    pub id: ScanId,
    #[schemars(with = "String")]
    pub started_at: Timestamp,
    #[schemars(with = "Option<String>")]
    pub finished_at: Option<Timestamp>,
    pub roots: Vec<PathBuf>,
    pub kipple_version: String,
    pub status: ScanStatus,
    /// Directory entries visited by the walk.
    pub entries_seen: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Running,
    Complete,
    Cancelled,
    Failed,
}

/// Report numbers for one scan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    /// All items, including `Keep`.
    pub items: u64,
    /// Bytes of `Safe` and `Review` items.
    pub reclaimable: u64,
    pub by_safety: SafetyMix,
    pub categories: Vec<CategorySummary>,
    /// Bytes held in quarantine, not yet freed.
    pub quarantine_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ScanResult {
    pub scan: ScanInfo,
    pub summary: Summary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Latest scan report: summary plus heaviest projects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Report {
    pub scan: ScanInfo,
    pub summary: Summary,
    pub top_projects: Vec<ProjectSummary>,
    /// What `kipple clean --safe-only` would do.
    pub next: Option<NextStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NextStep {
    pub command: String,
    pub reclaimable: u64,
    /// Part of `reclaimable` that can be undone during the quarantine window.
    pub undoable: u64,
    pub keep_days: u32,
}

/// Everything about one item (`kipple explain`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Explanation {
    pub item: Item,
    pub integration: IntegrationInfo,
    /// What each allowed action does, in order of preference.
    pub actions: Vec<ActionInfo>,
    /// `apparent` and `reclaimable` differ by more than 10%.
    pub size_differs: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ActionInfo {
    pub action: ActionKind,
    pub description: String,
    pub undoable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PlanRequest {
    pub filter: ItemFilter,
    /// Item ids to leave out.
    pub exclude_ids: Vec<String>,
    /// Use this action instead of the preferred one.
    pub action_overrides: BTreeMap<ItemId, ActionKind>,
}

/// One quarantined item (SPEC §8.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QuarantineEntry {
    pub id: QuarantineId,
    pub plan_id: PlanId,
    #[schemars(with = "String")]
    pub created_at: Timestamp,
    #[schemars(with = "String")]
    pub expires_at: Timestamp,
    pub integration: IntegrationId,
    /// Original location of each moved path.
    pub original_paths: Vec<PathBuf>,
    /// Snapshot of the item when it was quarantined.
    pub item: Item,
    pub size: Size,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RestoreReport {
    pub restored: Vec<RestoredEntry>,
    pub skipped: Vec<SkippedRestore>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RestoredEntry {
    pub id: QuarantineId,
    pub title: String,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SkippedRestore {
    pub id: QuarantineId,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PurgeRequest {
    /// Purge entries older than this many seconds. `None` uses `keep_days`.
    pub older_than_secs: Option<u64>,
    /// Purge everything regardless of age.
    pub all: bool,
    pub ids: Vec<QuarantineId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PurgeReport {
    pub purged: Vec<QuarantineId>,
    pub freed: u64,
    #[serde(default)]
    pub errors: Vec<String>,
}

/// One executed action (SPEC §8.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct JournalEntry {
    #[schemars(with = "String")]
    pub ts: Timestamp,
    pub plan_id: PlanId,
    pub item_id: ItemId,
    pub integration: IntegrationId,
    pub title: String,
    pub action: ActionKind,
    pub outcome: Outcome,
    pub freed: u64,
    pub original_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quarantine_id: Option<QuarantineId>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SizeScope {
    #[default]
    Total,
    Category,
    Integration,
    Project,
}

impl SizeScope {
    pub fn as_str(self) -> &'static str {
        match self {
            SizeScope::Total => "total",
            SizeScope::Category => "category",
            SizeScope::Integration => "integration",
            SizeScope::Project => "project",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct HistoryRequest {
    pub scope: SizeScope,
    /// Category, integration or project id. Ignored for `total`.
    pub key: Option<String>,
    #[schemars(with = "Option<String>")]
    pub since: Option<Timestamp>,
}

/// Reclaimable bytes at one point in time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SizePoint {
    #[schemars(with = "String")]
    pub ts: Timestamp,
    pub scope: SizeScope,
    pub key: String,
    pub reclaimable: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PlatformInfo {
    pub os: Os,
    pub arch: String,
    pub wsl: bool,
    pub trash_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LocationCheck {
    pub path: PathBuf,
    pub exists: bool,
    pub readable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ToolCheck {
    pub name: String,
    pub found: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DoctorIntegration {
    pub info: IntegrationInfo,
    pub enabled: bool,
    pub detection: Detection,
    pub locations: Vec<LocationCheck>,
    pub tools: Vec<ToolCheck>,
    /// Actionable fixes, one sentence each.
    pub fixes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DoctorReport {
    pub platform: PlatformInfo,
    pub config_path: PathBuf,
    pub config_exists: bool,
    pub data_dir: PathBuf,
    pub integrations: Vec<DoctorIntegration>,
    pub warnings: Vec<String>,
}
