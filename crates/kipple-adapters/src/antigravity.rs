//! `antigravity`: Google Antigravity conversations (INTEGRATIONS.md §4.13,
//! docs/integrations/antigravity.md). Experimental.
//!
//! Quarantine only. Antigravity's own `state.vscdb` (its sidebar index) is
//! never opened or modified.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink};
use kipple_core::model::{
    ActionKind, Category, Detection, IntegrationInfo, IntegrationStatus, Locator, Reason,
    ReasonCode, Safety, Timestamp,
};
use serde_json::json;

use crate::ai;
use crate::util::{children, file_name, info};

pub const ID: &str = "antigravity";

/// Both apps ("Antigravity", "Antigravity IDE") and their helpers.
const PROCESS: &str = "Antigravity*";

/// Data roots under `~/.gemini`, most authoritative first.
/// `antigravity-ide`: the Antigravity IDE. `antigravity`: the standalone
/// Antigravity 2.0 app (before May 2026, the IDE). `antigravity-backup`: the
/// snapshot the 2.0 migration made.
pub const ROOTS: &[&str] = &["antigravity-ide", "antigravity", "antigravity-backup"];

/// Conversation file extensions: `.pb` (verified) and `.db` (newer, verify).
const EXTENSIONS: &[&str] = &["pb", "db"];

#[derive(Debug, Default)]
pub struct Antigravity;

/// One conversation's files in one root.
#[derive(Debug, Clone)]
struct Copy {
    root: &'static str,
    paths: Vec<PathBuf>,
    bytes: u64,
}

fn tree_bytes(path: &Path) -> u64 {
    let Ok(meta) = path.symlink_metadata() else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    children(path).iter().map(|c| tree_bytes(c)).sum()
}

impl Antigravity {
    fn gemini(ctx: &Ctx) -> Option<PathBuf> {
        ctx.home().map(|h| h.join(".gemini"))
    }

    /// Conversations per id, per root.
    fn collect(gemini: &Path) -> BTreeMap<String, Vec<Copy>> {
        let mut out: BTreeMap<String, Vec<Copy>> = BTreeMap::new();
        for root in ROOTS {
            let base = gemini.join(root);
            let mut by_id: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
            for f in children(&base.join("conversations")) {
                let ext = f.extension().and_then(|e| e.to_str()).unwrap_or_default();
                if !f.is_file() || !EXTENSIONS.contains(&ext) {
                    continue;
                }
                let Some(id) = f.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                by_id.entry(id.to_owned()).or_default().push(f.clone());
            }
            for d in children(&base.join("brain")) {
                if d.is_dir() {
                    by_id.entry(file_name(&d)).or_default().push(d.clone());
                }
            }
            // Conversation files come first (the primary path), then the
            // working folder.
            for (id, paths) in by_id {
                let bytes = paths.iter().map(|p| tree_bytes(p)).sum();
                out.entry(id).or_default().push(Copy { root, paths, bytes });
            }
        }
        out
    }

    fn draft(id: &str, copy: &Copy, newer: Option<&Copy>) -> ItemDraft {
        let when = copy
            .paths
            .iter()
            .filter_map(|p| ai::mtime(p))
            .max()
            .and_then(|t| Timestamp::try_from(t).ok());
        let date = when
            .map(ai::day)
            .unwrap_or_else(|| "unknown date".to_owned());
        let title = format!("Conversation {} · {date}", ai::short_id(id));
        let locator = if copy.paths.len() == 1 {
            Locator::path(&copy.paths[0])
        } else {
            Locator::Paths {
                paths: copy.paths.clone(),
            }
        };
        let mut d = ItemDraft::new("antigravity_conversation", title, locator);
        d.category = Some(Category::AiHistory);
        d.safety = Safety::Review;
        d.reasons = ai::chat_reasons();
        d.actions = vec![ActionKind::Quarantine];
        d.last_used = when.map(|at| kipple_core::model::LastUsed {
            at,
            source: kipple_core::model::LastUsedSource::ItemMtime,
        });
        if let Some(n) = newer {
            d.reasons.insert(
                0,
                Reason {
                    code: ReasonCode::MigratedCopy,
                    text: format!(
                        "A copy of this conversation with at least as much data is in ~/.gemini/{}.",
                        n.root
                    ),
                },
            );
            if copy.root == "antigravity" {
                d.notes.push(
                    "The Antigravity 2.0 app reads this folder, so it may still list this conversation."
                        .to_owned(),
                );
            }
        }
        let has_conversation = copy.paths.iter().any(|p| p.is_file());
        if !has_conversation {
            d.notes.push(
                "Only the agent's working files are left; the conversation itself is gone."
                    .to_owned(),
            );
        }
        d.details = json!({
            "conversation_id": id,
            "data_root": format!("~/.gemini/{}", copy.root),
            "has_conversation_file": has_conversation,
            "has_brain": copy.paths.iter().any(|p| p.is_dir()),
        });
        d
    }
}

impl Adapter for Antigravity {
    fn info(&self) -> IntegrationInfo {
        info(
            ID,
            "Antigravity",
            Category::AiHistory,
            "Google Antigravity's saved agent conversations and their working files.",
            IntegrationStatus::Experimental,
            &[],
            &[PROCESS],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let found = Self::gemini(ctx).is_some_and(|g| ROOTS.iter().any(|r| g.join(r).is_dir()));
        if found {
            Detection::present()
        } else {
            Detection::NotPresent
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        let Some(g) = Self::gemini(ctx) else {
            return Vec::new();
        };
        ROOTS
            .iter()
            .map(|r| g.join(r))
            .filter(|p| p.is_dir())
            .collect()
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(gemini) = Self::gemini(ctx) else {
            return Ok(());
        };
        for (id, copies) in Self::collect(&gemini) {
            if ctx.cancel.is_cancelled() {
                return Ok(());
            }
            for (i, copy) in copies.iter().enumerate() {
                // A copy is migrated when a more authoritative root has the
                // same conversation with at least as much data.
                let newer = copies[..i].iter().find(|c| c.bytes >= copy.bytes);
                sink.item(Self::draft(&id, copy, newer));
            }
        }
        Ok(())
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        matches!(action, ActionKind::Quarantine).then(|| {
            "Move the conversation file and its working folder to kipple's quarantine. Antigravity's own database is not changed, so its sidebar may still list the conversation until you remove it there.".to_owned()
        })
    }
}
