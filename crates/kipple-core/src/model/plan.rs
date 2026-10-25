use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Item, ItemId, PlanId, QuarantineId, SafetyMix, ScanId, Timestamp};

/// What to do with an item (SPEC §4.3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionKind {
    /// Move into kipple's quarantine (same volume). Default for paths.
    Quarantine,
    /// OS trash; used when quarantine would cross volumes.
    Trash,
    /// Integration-defined command, e.g. `docker.image.remove`, `codex.delete`.
    Native { command_id: String },
    /// `sqlite_vacuum`, `docker_desktop_disk` [v1], `wsl_vhdx` [v1].
    Compact { method: String },
    /// [v1] tar+zstd to the archive dir, then quarantine the original.
    Archive,
    /// [v1] Move to another volume and symlink back.
    Relocate { target: PathBuf },
    /// [v1] Edit a tool's own retention setting.
    SetRetention {
        key: String,
        value: serde_json::Value,
    },
}

impl ActionKind {
    pub fn native(command_id: impl Into<String>) -> Self {
        ActionKind::Native {
            command_id: command_id.into(),
        }
    }

    /// Short name: `quarantine`, `trash`, `native`, …
    pub fn name(&self) -> &'static str {
        match self {
            ActionKind::Quarantine => "quarantine",
            ActionKind::Trash => "trash",
            ActionKind::Native { .. } => "native",
            ActionKind::Compact { .. } => "compact",
            ActionKind::Archive => "archive",
            ActionKind::Relocate { .. } => "relocate",
            ActionKind::SetRetention { .. } => "set_retention",
        }
    }

    /// Whether bytes are freed right away (as opposed to held in quarantine).
    pub fn frees_immediately(&self) -> bool {
        !matches!(self, ActionKind::Quarantine | ActionKind::Archive)
    }

    /// Whether `kipple undo` can reverse it.
    pub fn undoable(&self) -> bool {
        matches!(self, ActionKind::Quarantine | ActionKind::Archive)
    }
}

/// `quarantine`, `trash`, `native:<id>`, `compact:<method>`, `archive`.
impl FromStr for ActionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once(':') {
            None => match s {
                "quarantine" => Ok(ActionKind::Quarantine),
                "trash" => Ok(ActionKind::Trash),
                "archive" => Ok(ActionKind::Archive),
                _ => Err(format!("unknown action `{s}`")),
            },
            Some(("native", id)) if !id.is_empty() => Ok(ActionKind::native(id)),
            Some(("compact", method)) if !method.is_empty() => Ok(ActionKind::Compact {
                method: method.to_owned(),
            }),
            _ => Err(format!("unknown action `{s}`")),
        }
    }
}

impl fmt::Display for ActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionKind::Native { command_id } => write!(f, "native:{command_id}"),
            ActionKind::Compact { method } => write!(f, "compact:{method}"),
            ActionKind::Relocate { target } => write!(f, "relocate:{}", target.display()),
            ActionKind::SetRetention { key, .. } => write!(f, "set_retention:{key}"),
            other => f.write_str(other.name()),
        }
    }
}

/// Result of one preflight check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Check {
    pub id: String,
    pub ok: bool,
    pub message: String,
}

impl Check {
    pub fn pass(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ok: true,
            message: message.into(),
        }
    }

    pub fn fail(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ok: false,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlanEntry {
    pub item: Item,
    pub action: ActionKind,
    #[serde(default)]
    pub preflight: Vec<Check>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlanTotals {
    pub entries: u64,
    /// Reclaimable bytes of all entries.
    pub reclaimable: u64,
    pub by_safety: SafetyMix,
    /// Reclaimable bytes per action name.
    pub by_action: BTreeMap<String, u64>,
    /// Freed when the plan runs (native commands, trash, compaction).
    pub freed_now: u64,
    /// Held in quarantine until purged.
    pub held_in_quarantine: u64,
}

/// An ordered list of (item, action) the user approves (SPEC §4.3, §8.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Plan {
    pub id: PlanId,
    #[schemars(with = "String")]
    pub created_at: Timestamp,
    pub scan_id: ScanId,
    pub entries: Vec<PlanEntry>,
    pub totals: PlanTotals,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl Plan {
    pub fn has_review(&self) -> bool {
        self.entries
            .iter()
            .any(|e| e.item.safety == super::Safety::Review)
    }
}

/// What happened to one plan entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    Done { freed: u64 },
    Skipped { reason: String },
    Failed { error: String },
}

impl Outcome {
    pub fn freed(&self) -> u64 {
        match self {
            Outcome::Done { freed } => *freed,
            _ => 0,
        }
    }
}

/// How the user confirmed a plan (SPEC §8.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Confirmation {
    /// Typed `y`. Enough for plans with only Safe entries.
    Yes,
    /// Typed a word. Plans with Review entries need `reclaim`.
    Typed { word: String },
}

impl Confirmation {
    pub const REVIEW_WORD: &'static str = "reclaim";

    /// Whether this confirmation is enough for `plan`.
    pub fn permits(&self, plan: &Plan) -> bool {
        match self {
            Confirmation::Yes => !plan.has_review(),
            Confirmation::Typed { word } => {
                word == Self::REVIEW_WORD || (!plan.has_review() && word == "y")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExecEntry {
    pub item_id: ItemId,
    pub title: String,
    pub action: ActionKind,
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quarantine_id: Option<QuarantineId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExecReport {
    pub plan_id: PlanId,
    pub entries: Vec<ExecEntry>,
    /// Bytes freed right away.
    pub freed_now: u64,
    /// Bytes moved to quarantine; freed after purge.
    pub held_in_quarantine: u64,
    pub done: u64,
    pub skipped: u64,
    pub failed: u64,
    /// Stopped early by cancellation.
    pub cancelled: bool,
}

impl ExecReport {
    pub fn is_partial(&self) -> bool {
        self.skipped > 0 || self.failed > 0 || self.cancelled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_strings_round_trip() {
        for s in [
            "quarantine",
            "trash",
            "archive",
            "native:npm-cache-clean",
            "compact:sqlite_vacuum",
        ] {
            let action: ActionKind = s.parse().unwrap();
            assert_eq!(action.to_string(), s);
        }
        assert!("native:".parse::<ActionKind>().is_err());
        assert!("delete".parse::<ActionKind>().is_err());
    }
}
