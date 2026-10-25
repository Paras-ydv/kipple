//! `docker` and `docker-desktop-disk` (INTEGRATIONS.md §4.5).

pub mod client;

use std::collections::BTreeMap;
use std::path::PathBuf;

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink, SizeSpec};
use kipple_core::model::{
    ActionKind, Category, Check, Detection, IntegrationInfo, IntegrationStatus, Item, LastUsed,
    LastUsedSource, Locator, Os, Outcome, Reason, ReasonCode, Safety, Size, Timestamp,
};
use kipple_core::platform::fsx;
use kipple_core::safety::reasons::reason;
use serde_json::Value;

use self::client::{Client, DockerError};
use crate::util;

pub struct Docker;
pub struct DesktopDisk;

const PRUNE: &str = "docker.builder.prune";
const RM_IMAGE: &str = "docker.image.remove";
const RM_CONTAINER: &str = "docker.container.remove";
const RM_VOLUME: &str = "docker.volume.remove";
const COMPOSE_DIR: &str = "com.docker.compose.project.working_dir";
const COMPOSE_PROJECT: &str = "com.docker.compose.project";

fn known(bytes: u64, reclaimable: u64) -> SizeSpec {
    SizeSpec::Known(Size {
        apparent: bytes,
        on_disk: bytes,
        reclaimable,
        file_count: None,
    })
}

fn u64_of(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0).max(0) as u64
}

fn str_of<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn label<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get("Labels")?.get(key)?.as_str()
}

fn created(v: &Value) -> Option<LastUsed> {
    let secs = v.get("Created")?.as_i64()?;
    Some(LastUsed {
        at: Timestamp::from_second(secs).ok()?,
        source: LastUsedSource::ToolMetadata,
    })
}

fn short(id: &str) -> &str {
    let id = id.strip_prefix("sha256:").unwrap_or(id);
    &id[..id.len().min(12)]
}

fn docker_locator(kind: &str, id: &str) -> Locator {
    Locator::Docker {
        kind: kind.into(),
        id: id.into(),
    }
}

/// Containers mounting each volume: (running, stopped) names.
fn volume_users(containers: &[Value]) -> BTreeMap<String, (Vec<String>, Vec<String>)> {
    let mut out: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for c in containers {
        let name = container_name(c);
        let running = str_of(c, "State") == "running";
        for m in c
            .get("Mounts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if str_of(m, "Type") == "volume" {
                let entry = out.entry(str_of(m, "Name").to_owned()).or_default();
                if running {
                    entry.0.push(name.clone());
                } else {
                    entry.1.push(name.clone());
                }
            }
        }
    }
    out
}

fn container_name(c: &Value) -> String {
    c.get("Names")
        .and_then(Value::as_array)
        .and_then(|n| n.first())
        .and_then(Value::as_str)
        .map(|n| n.trim_start_matches('/').to_owned())
        .unwrap_or_else(|| short(str_of(c, "Id")).to_owned())
}

/// Compose project name → working dir, from container labels.
fn compose_dirs(containers: &[Value]) -> BTreeMap<String, PathBuf> {
    containers
        .iter()
        .filter_map(|c| {
            Some((
                label(c, COMPOSE_PROJECT)?.to_owned(),
                PathBuf::from(label(c, COMPOSE_DIR)?),
            ))
        })
        .collect()
}

fn discover_from(df: &Value, sink: &mut dyn ItemSink) {
    let empty = Vec::new();
    let containers = df
        .get("Containers")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let projects = compose_dirs(containers);

    // Build cache: every record not in use, as one item.
    let cache = df
        .get("BuildCache")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let (mut all, mut shared) = (0, 0);
    for r in cache
        .iter()
        .filter(|r| !r.get("InUse").and_then(Value::as_bool).unwrap_or(false))
    {
        let size = u64_of(r, "Size");
        all += size;
        if r.get("Shared").and_then(Value::as_bool).unwrap_or(false) {
            shared += size;
        }
    }
    if all > 0 {
        let mut d = ItemDraft::new(
            "docker_build_cache",
            "Docker build cache",
            docker_locator("build-cache", "all"),
        );
        d.size = known(all, all - shared);
        d.safety = Safety::Safe;
        d.reasons = vec![reason(ReasonCode::Regenerable)];
        d.actions = vec![ActionKind::native(PRUNE)];
        if shared > 0 {
            d.notes
                .push("Part of the build cache is shared with images and won't be freed.".into());
        }
        sink.item(d);
    }

    // Images no container uses (running or stopped).
    for img in df.get("Images").and_then(Value::as_array).unwrap_or(&empty) {
        let users = img.get("Containers").and_then(Value::as_i64).unwrap_or(-1);
        if users > 0 {
            continue;
        }
        let id = str_of(img, "Id");
        let tags: Vec<&str> = img
            .get("RepoTags")
            .and_then(Value::as_array)
            .map(|t| {
                t.iter()
                    .filter_map(Value::as_str)
                    .filter(|t| *t != "<none>:<none>")
                    .collect()
            })
            .unwrap_or_default();
        let size = u64_of(img, "Size");
        let shared = img
            .get("SharedSize")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0) as u64;
        let title = match tags.first() {
            Some(t) => format!("{t} image"),
            None => format!("Untagged image {}", short(id)),
        };
        let mut d = ItemDraft::new("docker_image", title, docker_locator("image", id));
        d.size = known(size, size.saturating_sub(shared));
        d.last_used = created(img);
        d.details = serde_json::json!({ "id": id, "tags": tags });
        d.actions = vec![ActionKind::native(RM_IMAGE)];
        if users < 0 {
            d.unknown_format = true;
        } else if tags.is_empty() {
            d.safety = Safety::Safe;
            d.reasons = vec![Reason {
                code: ReasonCode::Regenerable,
                text: "An untagged image left behind by a newer build or pull.".into(),
            }];
        } else {
            d.safety = Safety::Review;
            d.reasons = vec![Reason {
                code: ReasonCode::Redownloadable,
                text:
                    "No container uses this image. Pulling or building it again can take a while."
                        .into(),
            }];
        }
        if shared > 0 {
            d.notes.push("Layers shared with other images stay.".into());
        }
        sink.item(d);
    }

    // Stopped containers: their writable layer holds whatever they wrote.
    for c in containers {
        let state = str_of(c, "State");
        if !matches!(state, "exited" | "created" | "dead") {
            continue;
        }
        let id = str_of(c, "Id");
        let name = container_name(c);
        let size = u64_of(c, "SizeRw");
        let mut d = ItemDraft::new(
            "docker_container",
            format!("Container {name} ({})", str_of(c, "Image")),
            docker_locator("container", id),
        );
        d.size = known(size, size);
        d.last_used = created(c);
        d.project_hint = label(c, COMPOSE_DIR).map(PathBuf::from);
        d.safety = Safety::Review;
        d.reasons = vec![reason(ReasonCode::ContainsUserData)];
        d.actions = vec![ActionKind::native(RM_CONTAINER)];
        d.details = serde_json::json!({ "id": id, "name": name, "image": str_of(c, "Image"), "status": str_of(c, "Status") });
        sink.item(d);
    }

    // Volumes.
    let users = volume_users(containers);
    for v in df
        .get("Volumes")
        .and_then(Value::as_array)
        .unwrap_or(&empty)
    {
        let name = str_of(v, "Name");
        let usage = v.get("UsageData");
        let size = usage.map(|u| u64_of(u, "Size")).unwrap_or(0);
        let refs = usage
            .and_then(|u| u.get("RefCount"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let project = label(v, COMPOSE_PROJECT);
        let title = match project {
            Some(p) => format!("Volume {name} ({p})"),
            None => format!("Volume {name}"),
        };
        let mut d = ItemDraft::new("docker_volume", title, docker_locator("volume", name));
        d.size = known(size, size);
        d.project_hint = project.and_then(|p| projects.get(p).cloned());
        d.details = serde_json::json!({ "name": name, "compose_project": project });
        let (running, stopped) = users.get(name).cloned().unwrap_or_default();
        if !running.is_empty() {
            d.safety = Safety::Keep;
            d.reasons = vec![Reason {
                code: ReasonCode::InUseByContainer,
                text: format!(
                    "The running container {} uses this volume.",
                    running.join(", ")
                ),
            }];
        } else if !stopped.is_empty() || refs > 0 {
            d.safety = Safety::Keep;
            d.reasons = vec![Reason {
                code: ReasonCode::InUseByContainer,
                text: if stopped.is_empty() {
                    "A container uses this volume.".into()
                } else {
                    format!(
                        "The stopped container {} uses this volume; remove the container first.",
                        stopped.join(", ")
                    )
                },
            }];
        } else {
            d.safety = Safety::Review;
            d.reasons = vec![reason(ReasonCode::ContainsUserData)];
            d.actions = vec![ActionKind::native(RM_VOLUME)];
        }
        sink.item(d);
    }
}

fn connect(ctx: &Ctx) -> Result<Client, DockerError> {
    Client::connect(&ctx.platform.env)
}

impl Adapter for Docker {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "docker",
            "Docker",
            Category::Container,
            "Docker images, stopped containers, unused volumes and build cache, measured through Docker itself.",
            IntegrationStatus::Stable,
            &[],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        match connect(ctx) {
            Ok(c) => {
                let version = c
                    .get_json("/version")
                    .ok()
                    .and_then(|v| v.get("Version").and_then(Value::as_str).map(str::to_owned));
                Detection::Present { version }
            }
            Err(_) if ctx.platform.runner.which("docker").is_some() => Detection::Unsupported {
                reason: "Docker isn't running. Start it to measure images and volumes.".into(),
            },
            Err(_) => Detection::NotPresent,
        }
    }

    fn tools(&self) -> Vec<String> {
        vec!["docker".into()]
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let client = match connect(ctx) {
            Ok(c) => c,
            Err(e) => {
                if ctx.platform.runner.which("docker").is_some() {
                    sink.warning(format!("{e}. Start Docker to measure images and volumes."));
                }
                return Ok(());
            }
        };
        let df = client
            .get_json("/system/df")
            .map_err(|e| IntegrationError::other(e.to_string()))?;
        discover_from(&df, sink);
        Ok(())
    }

    fn preflight(&self, ctx: &Ctx, item: &Item, _action: &ActionKind) -> Vec<Check> {
        let Locator::Docker { kind, id } = &item.locator else {
            return vec![Check::fail("locator", "not a Docker item")];
        };
        let client = match connect(ctx) {
            Ok(c) => c,
            Err(e) => return vec![Check::fail("docker", e.to_string())],
        };
        // Re-check identity and use right before acting (SPEC §8.3).
        let check = match kind.as_str() {
            "container" => match client.get_json(&format!("/containers/{id}/json")) {
                Ok(c) if c.pointer("/State/Running").and_then(Value::as_bool) == Some(false) => {
                    Check::pass("state", "container is stopped")
                }
                Ok(_) => Check::fail("state", "The container is running now."),
                Err(e) => Check::fail("exists", e.to_string()),
            },
            "image" => match client.get_json(&format!(
                "/containers/json?all=1&filters=%7B%22ancestor%22%3A%5B%22{id}%22%5D%7D"
            )) {
                Ok(list) if list.as_array().is_some_and(Vec::is_empty) => {
                    Check::pass("unused", "no container uses it")
                }
                Ok(_) => Check::fail("unused", "A container uses this image now."),
                Err(e) => Check::fail("docker", e.to_string()),
            },
            "volume" => match client.get_json(&format!(
                "/containers/json?all=1&filters=%7B%22volume%22%3A%5B%22{id}%22%5D%7D"
            )) {
                Ok(list) if list.as_array().is_some_and(Vec::is_empty) => {
                    Check::pass("unused", "no container mounts it")
                }
                Ok(_) => Check::fail("unused", "A container uses this volume now."),
                Err(e) => Check::fail("docker", e.to_string()),
            },
            _ => Check::pass("docker", "reachable"),
        };
        vec![check]
    }

    fn apply(
        &self,
        ctx: &Ctx,
        item: &Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        let (ActionKind::Native { command_id }, Locator::Docker { id, .. }) =
            (action, &item.locator)
        else {
            return Err(IntegrationError::Unsupported(format!(
                "docker can't run `{action}`"
            )));
        };
        let client = connect(ctx).map_err(|e| IntegrationError::other(e.to_string()))?;
        let result = match command_id.as_str() {
            PRUNE => client
                .send("POST", "/build/prune")
                .map(|v| u64_of(&v, "SpaceReclaimed")),
            RM_IMAGE => client
                .send("DELETE", &format!("/images/{id}"))
                .map(|_| item.size.reclaimable),
            RM_CONTAINER => client
                .send("DELETE", &format!("/containers/{id}"))
                .map(|_| item.size.reclaimable),
            RM_VOLUME => client
                .send("DELETE", &format!("/volumes/{id}"))
                .map(|_| item.size.reclaimable),
            other => {
                return Err(IntegrationError::other(format!(
                    "unknown command `{other}`"
                )));
            }
        };
        Ok(match result {
            Ok(freed) => Outcome::Done { freed },
            Err(e) => Outcome::Failed {
                error: e.to_string(),
            },
        })
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        let ActionKind::Native { command_id } = action else {
            return None;
        };
        Some(
            match command_id.as_str() {
                PRUNE => "Asks Docker to prune its build cache (like `docker builder prune`). Builds recreate it.",
                RM_IMAGE => "Asks Docker to remove the image (like `docker image rm`). Pull or build it again to get it back.",
                RM_CONTAINER => "Asks Docker to remove the stopped container (like `docker rm`). Files it wrote inside are lost.",
                RM_VOLUME => "Asks Docker to remove the volume (like `docker volume rm`). Its data is lost for good.",
                _ => return None,
            }
            .to_owned(),
        )
    }
}

/// Docker Desktop's virtual disk file, per OS (verify per Docker Desktop
/// version).
fn desktop_disks(ctx: &Ctx) -> Vec<PathBuf> {
    let Some(home) = ctx.home() else {
        return Vec::new();
    };
    match ctx.platform.os {
        Os::Macos => {
            vec![home.join("Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw")]
        }
        Os::Windows => {
            let local = ctx
                .platform
                .env
                .get_path("LOCALAPPDATA")
                .unwrap_or_else(|| home.join("AppData/Local"));
            vec![
                local.join("Docker/wsl/disk/docker_data.vhdx"),
                local.join("Docker/wsl/data/ext4.vhdx"),
            ]
        }
        Os::Linux | Os::Wsl => vec![home.join(".docker/desktop/vms/0/data/Docker.raw")],
    }
    .into_iter()
    .filter(|p| p.is_file())
    .collect()
}

impl Adapter for DesktopDisk {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "docker-desktop-disk",
            "Docker Desktop disk",
            Category::Container,
            "The virtual disk Docker Desktop keeps everything in. Report only: kipple shows its size but doesn't shrink it yet.",
            IntegrationStatus::Experimental,
            &[],
            &[],
        )
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        if desktop_disks(ctx).is_empty() {
            Detection::NotPresent
        } else {
            Detection::present()
        }
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        for path in desktop_disks(ctx) {
            let Ok(meta) = path.symlink_metadata() else {
                continue;
            };
            let mut d = ItemDraft::new(
                "docker_desktop_disk",
                "Docker Desktop disk image",
                Locator::path(&path),
            );
            d.size = SizeSpec::Known(Size {
                apparent: meta.len(),
                on_disk: fsx::allocated(&meta),
                reclaimable: 0,
                file_count: Some(1),
            });
            d.safety = Safety::Keep;
            d.reasons = vec![Reason {
                code: ReasonCode::ContainsUserData,
                text: "All your Docker data lives in this file. Remove images and volumes inside Docker to free space in it.".into(),
            }];
            d.notes.push(
                "Whether macOS or Windows gets the space back depends on your Docker Desktop version.".into(),
            );
            sink.item(d);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kipple_core::safety::reasons::reason_with;

    fn df() -> Value {
        serde_json::json!({
            "Images": [
                {"Id": "sha256:aaa111", "RepoTags": ["postgres:16"], "Size": 400, "SharedSize": 100, "Containers": 0, "Created": 1700000000},
                {"Id": "sha256:bbb222", "RepoTags": [], "Size": 50, "SharedSize": 0, "Containers": 0, "Created": 1700000000},
                {"Id": "sha256:ccc333", "RepoTags": ["app:latest"], "Size": 900, "SharedSize": 0, "Containers": 1}
            ],
            "Containers": [
                {"Id": "c1", "Names": ["/nomad-db-1"], "Image": "postgres:16", "State": "exited", "Status": "Exited (0) 2 weeks ago", "SizeRw": 30, "Created": 1700000000,
                 "Labels": {"com.docker.compose.project": "nomad", "com.docker.compose.project.working_dir": "/w/nomad"},
                 "Mounts": [{"Type": "volume", "Name": "nomad_pgdata"}]},
                {"Id": "c2", "Names": ["/web"], "Image": "app:latest", "State": "running", "SizeRw": 5,
                 "Mounts": [{"Type": "volume", "Name": "cache"}]}
            ],
            "Volumes": [
                {"Name": "nomad_pgdata", "Labels": {"com.docker.compose.project": "nomad"}, "UsageData": {"Size": 6000, "RefCount": 1}},
                {"Name": "cache", "UsageData": {"Size": 10, "RefCount": 1}},
                {"Name": "orphan", "Labels": {"com.docker.compose.project": "nomad"}, "UsageData": {"Size": 70, "RefCount": 0}}
            ],
            "BuildCache": [
                {"ID": "b1", "InUse": false, "Shared": false, "Size": 300},
                {"ID": "b2", "InUse": false, "Shared": true, "Size": 20},
                {"ID": "b3", "InUse": true, "Shared": false, "Size": 1000}
            ]
        })
    }

    #[test]
    fn classifies_df() {
        let mut items: Vec<ItemDraft> = Vec::new();
        discover_from(&df(), &mut items);
        let by_title: BTreeMap<String, &ItemDraft> =
            items.iter().map(|d| (d.title.clone(), d)).collect();
        let cache = by_title["Docker build cache"];
        assert_eq!(cache.safety, Safety::Safe);
        assert_eq!(cache.size, known(320, 300));
        assert_eq!(by_title["postgres:16 image"].safety, Safety::Review);
        assert_eq!(by_title["postgres:16 image"].size, known(400, 300));
        assert_eq!(by_title["Untagged image bbb222"].safety, Safety::Safe);
        assert!(
            !by_title.contains_key("app:latest image"),
            "images in use aren't listed"
        );
        let c = by_title["Container nomad-db-1 (postgres:16)"];
        assert_eq!(c.safety, Safety::Review);
        assert_eq!(c.project_hint, Some(PathBuf::from("/w/nomad")));
        assert!(
            !by_title.keys().any(|t| t.contains("web")),
            "running containers aren't listed"
        );
        assert_eq!(by_title["Volume nomad_pgdata (nomad)"].safety, Safety::Keep);
        assert!(
            by_title["Volume nomad_pgdata (nomad)"].reasons[0]
                .text
                .contains("stopped container nomad-db-1")
        );
        assert_eq!(by_title["Volume cache"].safety, Safety::Keep);
        let orphan = by_title["Volume orphan (nomad)"];
        assert_eq!(orphan.safety, Safety::Review);
        assert_eq!(orphan.project_hint, Some(PathBuf::from("/w/nomad")));
        assert_eq!(orphan.actions, vec![ActionKind::native(RM_VOLUME)]);
        assert_eq!(
            reason_with(ReasonCode::ContainsUserData, &[]).code,
            orphan.reasons[0].code
        );
    }
}
