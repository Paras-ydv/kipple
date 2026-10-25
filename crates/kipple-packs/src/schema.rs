//! Pack file format (INTEGRATIONS.md §2).

use std::collections::BTreeMap;

use kipple_core::model::{Category, IntegrationStatus, Os, ReasonCode, Safety};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A pack: one declarative integration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    /// Kebab-case, unique, stable forever.
    pub id: String,
    pub name: String,
    pub category: Category,
    /// One or two plain sentences, user-facing.
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    /// Restrict to these systems. Empty means all.
    #[serde(default)]
    pub os: Vec<Os>,
    #[serde(default)]
    pub status: IntegrationStatus,
    /// Disabled unless listed in `integrations.enable`.
    #[serde(default)]
    pub opt_in: bool,
    #[serde(default)]
    pub detect: Detect,
    #[serde(default)]
    pub vars: BTreeMap<String, VarDef>,
    #[serde(default)]
    pub locations: Vec<LocationDef>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactDef>,
    #[serde(default)]
    pub commands: BTreeMap<String, CommandDef>,
    /// Process names that make items `Keep` (`name` or `name|arg`).
    #[serde(default)]
    pub in_use_processes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Detect {
    /// Active only if one of these is on PATH.
    #[serde(default)]
    pub any_command: Vec<String>,
    /// Or one of these paths exists.
    #[serde(default)]
    pub any_path: Vec<String>,
}

/// A named path. Resolution order: `command` → `env` → OS key → `default`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VarDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub macos: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linux: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<String>,
    /// Used when no OS key matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Environment variable that overrides the OS default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    /// Command whose stdout (trimmed) wins over everything when it succeeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    /// Appended to `command` and `env` results (e.g. `_cacache`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Each {
    /// One item for the path.
    #[default]
    #[serde(rename = "self")]
    Itself,
    /// One item per child entry.
    Child,
}

/// A fixed location (caches).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocationDef {
    /// Path or glob; may use `{var}`.
    pub path: String,
    #[serde(default)]
    pub each: Each,
    pub kind: String,
    /// May use `{name}` (the child's name) and `{parent}` (its folder's
    /// name) when `each = "child"`.
    pub title: String,
    pub safety: Safety,
    pub reasons: Vec<ReasonCode>,
    /// Ordered: `native:<command-id>`, `quarantine`, `trash`.
    pub actions: Vec<String>,
    /// With `each = "child"`: keep the N newest children as `Keep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_latest: Option<u32>,
    /// Skip items smaller than this (e.g. `10MB`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_hint: Option<String>,
    /// Only on these systems.
    #[serde(default)]
    pub os: Vec<Os>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Aggregate {
    /// Merge all matches in one project into one item.
    Project,
}

/// An in-project artifact found by the walk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDef {
    /// Directory name or glob (`cmake-build-*`, `*`).
    pub dir: String,
    #[serde(default)]
    pub requires_sibling_any: Vec<String>,
    #[serde(default)]
    pub requires_sibling_all: Vec<String>,
    #[serde(default)]
    pub requires_inside_any: Vec<String>,
    /// Only if git ignores this directory (protects committed `build/`).
    #[serde(default)]
    pub requires_gitignored: bool,
    /// Only if the directory has a valid `CACHEDIR.TAG`.
    #[serde(default)]
    pub requires_cachedir_tag: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<Aggregate>,
    pub kind: String,
    /// Defaults to the directory name. May use `{name}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub safety: Safety,
    pub reasons: Vec<ReasonCode>,
    pub actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_hint: Option<String>,
    /// Safety when not inside any project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_outside_project: Option<Safety>,
    /// Lower is tried first. Defaults to 100; generic `*` rules use more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

/// A native command (argv, never a shell).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandDef {
    /// Program and fixed arguments. Arguments may use `{var}` and `{path}`
    /// (the item's path) only.
    pub run: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// `before-after`: freed bytes = size before − size after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<Measure>,
    /// Processes that must not run.
    #[serde(default)]
    pub requires_closed: Vec<String>,
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Measure {
    BeforeAfter,
}
