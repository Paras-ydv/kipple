//! `ollama`: local models (INTEGRATIONS.md §4.14).
//!
//! Layout (verified 2026-10-25): `<models>/manifests/<host>/<namespace>/<model>/<tag>`
//! (JSON, schemaVersion 2, `config` + `layers` with `digest`) and
//! `<models>/blobs/sha256-<hex>`. Blobs are shared between tags, so a model
//! only reclaims blobs no other manifest uses.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink, SizeSpec};
use kipple_core::model::{
    ActionKind, Category, Check, Detection, IntegrationInfo, IntegrationStatus, Item, Locator, Os,
    Outcome, ReasonCode, Safety, Size,
};
use kipple_core::platform::{Env, fsx};
use kipple_core::safety::reasons::{reason, reason_with};
use serde::Deserialize;

use crate::util;

pub struct Ollama;

const RM: &str = "ollama.rm";

pub fn models_dir(env: &Env) -> Option<PathBuf> {
    util::env_or_home(env, &["OLLAMA_MODELS"], ".ollama/models")
}

/// Linux system-service install; needs elevation to change.
const SYSTEM_MODELS: &str = "/usr/share/ollama/.ollama/models";

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    config: Option<Layer>,
    layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
struct Layer {
    digest: String,
}

/// One `model:tag` with the blobs it uses.
#[derive(Debug)]
struct Model {
    name: String,
    manifest: PathBuf,
    blobs: Vec<String>,
    unknown: bool,
}

fn blob_file(models: &Path, digest: &str) -> PathBuf {
    models.join("blobs").join(digest.replace(':', "-"))
}

/// Display name: `llama3:latest` for the default registry and library,
/// `user/model:tag` or `host/ns/model:tag` otherwise.
fn display_name(host: &str, ns: &str, model: &str, tag: &str) -> String {
    match (host, ns) {
        ("registry.ollama.ai", "library") => format!("{model}:{tag}"),
        ("registry.ollama.ai", ns) => format!("{ns}/{model}:{tag}"),
        (host, ns) => format!("{host}/{ns}/{model}:{tag}"),
    }
}

fn read_models(models: &Path) -> Vec<Model> {
    let mut out = Vec::new();
    let root = models.join("manifests");
    for host in util::children(&root) {
        for ns in util::children(&host) {
            for model in util::children(&ns) {
                for tag in util::children(&model) {
                    if !tag.is_file() {
                        continue;
                    }
                    let name = display_name(
                        &util::file_name(&host),
                        &util::file_name(&ns),
                        &util::file_name(&model),
                        &util::file_name(&tag),
                    );
                    let parsed = std::fs::read_to_string(&tag)
                        .ok()
                        .and_then(|t| serde_json::from_str::<Manifest>(&t).ok())
                        .filter(|m| m.schema_version == 2);
                    let (blobs, unknown) = match parsed {
                        Some(m) => (
                            m.config
                                .into_iter()
                                .chain(m.layers)
                                .map(|l| l.digest)
                                .collect(),
                            false,
                        ),
                        None => (Vec::new(), true),
                    };
                    out.push(Model {
                        name,
                        manifest: tag,
                        blobs,
                        unknown,
                    });
                }
            }
        }
    }
    out
}

fn file_size(p: &Path) -> (u64, u64) {
    p.symlink_metadata()
        .map(|m| (m.len(), fsx::allocated(&m)))
        .unwrap_or((0, 0))
}

impl Adapter for Ollama {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "ollama",
            "Ollama models",
            Category::Model,
            "Models downloaded with Ollama. Removing one means downloading it again to use it.",
            IntegrationStatus::Stable,
            &[],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let dir_ok = models_dir(&ctx.platform.env).is_some_and(|d| d.join("manifests").is_dir());
        if dir_ok || ctx.platform.runner.which("ollama").is_some() {
            Detection::present()
        } else {
            Detection::NotPresent
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        models_dir(&ctx.platform.env)
            .filter(|d| d.is_dir())
            .into_iter()
            .collect()
    }

    fn tools(&self) -> Vec<String> {
        vec!["ollama".into()]
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(dir) = models_dir(&ctx.platform.env) else {
            return Ok(());
        };
        if ctx.platform.os == Os::Linux
            && Path::new(SYSTEM_MODELS).is_dir()
            && dir != Path::new(SYSTEM_MODELS)
        {
            sink.warning(format!(
                "Models in {SYSTEM_MODELS} belong to the Ollama system service; kipple doesn't change them because that needs administrator rights."
            ));
        }
        if !dir.join("manifests").is_dir() {
            return Ok(());
        }
        let models = read_models(&dir);
        let mut users: BTreeMap<&str, usize> = BTreeMap::new();
        for m in &models {
            for b in m.blobs.iter().collect::<BTreeSet<_>>() {
                *users.entry(b.as_str()).or_default() += 1;
            }
        }
        let has_cli = ctx.platform.runner.which("ollama").is_some();
        for m in &models {
            let mut size = Size::default();
            let mut files = 0;
            for b in m.blobs.iter().collect::<BTreeSet<_>>() {
                let (len, alloc) = file_size(&blob_file(&dir, b));
                size.apparent += len;
                size.on_disk += alloc;
                files += 1;
                if users.get(b.as_str()) == Some(&1) {
                    size.reclaimable += alloc;
                }
            }
            size.file_count = Some(files);
            let mut d = ItemDraft::new(
                "ollama_model",
                format!("{} (Ollama)", m.name),
                Locator::Logical {
                    store: "ollama".into(),
                    key: m.name.clone(),
                },
            );
            d.size = SizeSpec::Known(size);
            d.last_used = util::mtime_used(&m.manifest);
            d.safety = Safety::Review;
            d.reasons = vec![reason(ReasonCode::LargeRedownload)];
            d.unknown_format = m.unknown;
            d.restore_hint = Some(format!("ollama pull {}", m.name));
            d.details = serde_json::json!({ "model": m.name, "blobs": m.blobs.len() });
            if size.reclaimable < size.on_disk {
                d.notes
                    .push("Some of its files are shared with other models and stay.".into());
            }
            if has_cli {
                d.actions = vec![ActionKind::native(RM)];
            } else {
                d.safety = Safety::Keep;
                d.reasons.insert(
                    0,
                    reason_with(ReasonCode::NativeToolMissing, &[("tool", "ollama".into())]),
                );
            }
            sink.item(d);
        }

        // Blobs no manifest references (leftovers of removed models).
        let referenced: BTreeSet<PathBuf> = models
            .iter()
            .flat_map(|m| m.blobs.iter().map(|b| blob_file(&dir, b)))
            .collect();
        let orphans: Vec<PathBuf> = util::children(&dir.join("blobs"))
            .into_iter()
            .filter(|p| {
                let name = util::file_name(p);
                p.is_file()
                    && name.starts_with("sha256-")
                    && !name.ends_with("-partial")
                    && !name.contains("-partial-")
                    && !referenced.contains(p)
            })
            .collect();
        if !orphans.is_empty() && !models.iter().any(|m| m.unknown) {
            let mut d = ItemDraft::new(
                "ollama_orphan_blobs",
                format!("Unused Ollama files ({})", orphans.len()),
                Locator::Paths { paths: orphans },
            );
            d.safety = Safety::Safe;
            d.reasons = vec![kipple_core::model::Reason {
                code: ReasonCode::Regenerable,
                text: "No installed model uses these files.".into(),
            }];
            d.actions = vec![ActionKind::Quarantine];
            sink.item(d);
        }
        Ok(())
    }

    fn preflight(&self, ctx: &Ctx, _item: &Item, action: &ActionKind) -> Vec<Check> {
        if !matches!(action, ActionKind::Native { command_id } if command_id == RM) {
            return Vec::new();
        }
        vec![match ctx.platform.runner.which("ollama") {
            Some(_) => Check::pass("tool", "ollama is installed"),
            None => Check::fail("tool", "ollama isn't installed"),
        }]
    }

    fn apply(
        &self,
        ctx: &Ctx,
        item: &Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        let (ActionKind::Native { command_id }, Locator::Logical { key, .. }) =
            (action, &item.locator)
        else {
            return Err(IntegrationError::Unsupported(format!(
                "ollama can't run `{action}`"
            )));
        };
        if command_id != RM {
            return Err(IntegrationError::other(format!(
                "unknown command `{command_id}`"
            )));
        }
        let out = ctx.platform.runner.run(
            &["ollama", "rm", key.as_str()],
            None,
            Duration::from_secs(120),
        )?;
        if out.success() {
            Ok(Outcome::Done {
                freed: item.size.reclaimable,
            })
        } else {
            let msg = out.stderr.trim();
            let hint = if msg.contains("connect") || msg.contains("could not") {
                " Start Ollama first, then try again."
            } else {
                ""
            };
            Ok(Outcome::Failed {
                error: format!("`ollama rm {key}` failed: {msg}{hint}"),
            })
        }
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        matches!(action, ActionKind::Native { command_id } if command_id == RM).then(|| {
            "Runs `ollama rm`, which removes the model and any files no other model uses. Ollama must be running.".to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(
            display_name("registry.ollama.ai", "library", "llama3", "latest"),
            "llama3:latest"
        );
        assert_eq!(
            display_name("registry.ollama.ai", "me", "m", "q4"),
            "me/m:q4"
        );
        assert_eq!(display_name("hf.co", "org", "m", "q8"), "hf.co/org/m:q8");
    }
}
