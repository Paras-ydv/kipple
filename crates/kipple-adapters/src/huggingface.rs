//! `huggingface`: the Hugging Face Hub cache (INTEGRATIONS.md §4.15).
//!
//! Layout (verified 2026-10-25 against the huggingface_hub cache docs):
//! `<hub>/{models,datasets,spaces}--<org>--<name>/{blobs,refs,snapshots}`.
//! `refs/<branch>` holds a commit hash; `snapshots/<commit>/` holds symlinks
//! into `blobs/` (plain files on Windows without symlink support). The Xet
//! cache lives next to the hub cache in `xet/`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink, SizeSpec};
use kipple_core::model::{
    ActionKind, Category, Detection, IntegrationInfo, IntegrationStatus, Locator, Reason,
    ReasonCode, Safety, Size,
};
use kipple_core::platform::{Env, fsx};
use kipple_core::safety::reasons::reason;

use crate::util;

pub struct HuggingFace;

/// `HF_HUB_CACHE` → `HF_HOME/hub` → `~/.cache/huggingface/hub`.
pub fn hub_dir(env: &Env) -> Option<PathBuf> {
    env.get_path("HF_HUB_CACHE")
        .or_else(|| env.get_path("HF_HOME").map(|h| h.join("hub")))
        .or_else(|| env.home().map(|h| h.join(".cache/huggingface/hub")))
}

fn hf_home(env: &Env) -> Option<PathBuf> {
    env.get_path("HF_HOME")
        .or_else(|| env.home().map(|h| h.join(".cache/huggingface")))
}

const KINDS: &[(&str, &str)] = &[
    ("models--", "model"),
    ("datasets--", "dataset"),
    ("spaces--", "space"),
];

/// `models--org--name` → ("model", "org/name").
fn parse_repo(dir: &str) -> Option<(&'static str, String)> {
    KINDS.iter().find_map(|(prefix, kind)| {
        dir.strip_prefix(prefix)
            .map(|rest| (*kind, rest.replace("--", "/")))
    })
}

/// Blob names each snapshot uses (symlink targets, or file names when files
/// were copied).
fn snapshot_blobs(snapshot: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![snapshot.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for p in util::children(&dir) {
            let Ok(meta) = p.symlink_metadata() else {
                continue;
            };
            if meta.file_type().is_symlink() {
                if let Ok(target) = std::fs::read_link(&p) {
                    out.insert(util::file_name(&target));
                }
            } else if meta.is_dir() {
                stack.push(p);
            }
        }
    }
    out
}

fn blob_size(path: &Path) -> (u64, u64) {
    path.symlink_metadata()
        .map(|m| (m.len(), fsx::allocated(&m)))
        .unwrap_or((0, 0))
}

impl Adapter for HuggingFace {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "huggingface",
            "Hugging Face cache",
            Category::Model,
            "Models and datasets downloaded from the Hugging Face Hub. They download again when code asks for them.",
            IntegrationStatus::Stable,
            &[],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        match hub_dir(&ctx.platform.env) {
            Some(d) if d.is_dir() => Detection::present(),
            _ => Detection::NotPresent,
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        let env = &ctx.platform.env;
        let mut out: Vec<PathBuf> = hub_dir(env).into_iter().collect();
        if let Some(home) = hf_home(env) {
            out.push(home.join("xet"));
            out.push(
                env.get_path("HF_DATASETS_CACHE")
                    .unwrap_or_else(|| home.join("datasets")),
            );
        }
        out.retain(|p| p.is_dir());
        out
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let env = &ctx.platform.env;
        if let Some(hub) = hub_dir(env) {
            for repo in util::children(&hub) {
                let Some((kind, name)) = parse_repo(&util::file_name(&repo)) else {
                    continue;
                };
                if repo.is_dir() {
                    discover_repo(&repo, kind, &name, sink);
                }
            }
        }
        if let Some(home) = hf_home(env) {
            let extras = [
                (home.join("xet"), "hf_xet_cache", "Hugging Face Xet cache"),
                (
                    env.get_path("HF_DATASETS_CACHE")
                        .unwrap_or_else(|| home.join("datasets")),
                    "hf_datasets_cache",
                    "Hugging Face datasets cache",
                ),
            ];
            for (path, kind, title) in extras {
                if !path.is_dir() {
                    continue;
                }
                let mut d = ItemDraft::new(kind, title, Locator::path(&path));
                d.safety = Safety::Review;
                d.reasons = vec![reason(ReasonCode::LargeRedownload)];
                d.actions = vec![ActionKind::Quarantine];
                d.last_used = util::mtime_used(&path);
                sink.item(d);
            }
        }
        Ok(())
    }
}

fn discover_repo(repo: &Path, kind: &str, name: &str, sink: &mut dyn ItemSink) {
    // Revisions pointed to by any ref are current.
    let mut current: BTreeSet<String> = BTreeSet::new();
    let mut stack = vec![repo.join("refs")];
    while let Some(dir) = stack.pop() {
        for p in util::children(&dir) {
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(hash) = std::fs::read_to_string(&p) {
                current.insert(hash.trim().to_owned());
            }
        }
    }
    let snapshots: BTreeMap<String, PathBuf> = util::children(&repo.join("snapshots"))
        .into_iter()
        .filter(|p| p.is_dir())
        .map(|p| (util::file_name(&p), p))
        .collect();
    let mut current_blobs = BTreeSet::new();
    let mut old: Vec<(&String, &PathBuf, BTreeSet<String>)> = Vec::new();
    for (rev, path) in &snapshots {
        let blobs = snapshot_blobs(path);
        // Without refs we can't tell what's current: treat all as current.
        if current.is_empty() || current.contains(rev) {
            current_blobs.extend(blobs);
        } else {
            old.push((rev, path, blobs));
        }
    }
    let blobs_dir = repo.join("blobs");

    // Old revisions: their snapshot folders plus blobs only they use.
    let mut old_size = Size::default();
    let mut old_paths: Vec<PathBuf> = Vec::new();
    let mut old_only: BTreeSet<String> = BTreeSet::new();
    for (_, path, blobs) in &old {
        old_paths.push((*path).clone());
        old_only.extend(
            blobs
                .iter()
                .filter(|b| !current_blobs.contains(*b))
                .cloned(),
        );
    }
    for b in &old_only {
        let p = blobs_dir.join(b);
        let (len, alloc) = blob_size(&p);
        old_size.apparent += len;
        old_size.on_disk += alloc;
        old_size.reclaimable += alloc;
        if p.is_file() {
            old_paths.push(p);
        }
    }

    let mut d = ItemDraft::new(
        format!("hf_{kind}"),
        format!("{name} (Hugging Face {kind})"),
        Locator::path(repo),
    );
    d.safety = Safety::Review;
    d.reasons = vec![reason(ReasonCode::LargeRedownload)];
    d.actions = vec![ActionKind::Quarantine];
    d.last_used = util::mtime_used(&blobs_dir).or_else(|| util::mtime_used(repo));
    d.details = serde_json::json!({ "repo": name, "type": kind, "revisions": snapshots.len() });
    if !old.is_empty() {
        // Measured by the core, minus the old revisions listed separately.
        let total = kipple_core::scan::sizes::measure(&[repo.to_path_buf()], &Default::default())
            .finish()
            .0;
        d.size = SizeSpec::Known(Size {
            apparent: total.apparent,
            on_disk: total.on_disk,
            reclaimable: total.reclaimable.saturating_sub(old_size.reclaimable),
            file_count: total.file_count,
        });
        d.notes.push(format!(
            "{} listed separately; removing the whole repo removes them too.",
            util::plural(old.len(), "old revision is", "old revisions are")
        ));
    }
    sink.item(d);

    if !old.is_empty() {
        let mut o = ItemDraft::new(
            "hf_old_revisions",
            format!(
                "{name}: {}",
                util::plural(old.len(), "old revision", "old revisions")
            ),
            Locator::Paths { paths: old_paths },
        );
        o.size = SizeSpec::Known(Size {
            file_count: Some(old_only.len() as u64),
            ..old_size
        });
        o.safety = Safety::Safe;
        o.reasons = vec![Reason {
            code: ReasonCode::Redownloadable,
            text: "No branch or tag points to these revisions any more.".into(),
        }];
        o.actions = vec![ActionKind::Quarantine];
        o.details = serde_json::json!({ "repo": name, "revisions": old.iter().map(|(r, _, _)| r.as_str()).collect::<Vec<_>>() });
        sink.item(o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repo_folders() {
        assert_eq!(
            parse_repo("models--openai--whisper-large-v3"),
            Some(("model", "openai/whisper-large-v3".into()))
        );
        assert_eq!(
            parse_repo("datasets--squad"),
            Some(("dataset", "squad".into()))
        );
        assert_eq!(parse_repo(".locks"), None);
    }

    #[test]
    fn hub_dir_resolution() {
        let env = Env::from_pairs([("HOME", "/h"), ("HF_HOME", "/hf")]);
        assert_eq!(hub_dir(&env), Some(PathBuf::from("/hf/hub")));
        let env = Env::from_pairs([("HOME", "/h"), ("HF_HUB_CACHE", "/c")]);
        assert_eq!(hub_dir(&env), Some(PathBuf::from("/c")));
    }
}
