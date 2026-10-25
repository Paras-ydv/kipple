//! `android-avd` and `android-system-images` (INTEGRATIONS.md §4.6).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink, SizeSpec};
use kipple_core::model::{
    ActionKind, Category, Check, Detection, IntegrationInfo, IntegrationStatus, Item, LastUsed,
    LastUsedSource, Locator, Os, Outcome, Reason, ReasonCode, Safety, Size, Timestamp,
};
use kipple_core::platform::{Env, ProcessSnapshot};
use kipple_core::safety::reasons::{reason, reason_with};
use kipple_core::scan::sizes::measure;

use crate::util;

pub struct Avds;
pub struct SystemImages;

const AVD_DELETE: &str = "avdmanager.delete";
const SDK_UNINSTALL: &str = "sdkmanager.uninstall";

/// `ANDROID_HOME` → `ANDROID_SDK_ROOT` → OS default.
pub fn sdk_root(env: &Env, os: Os) -> Option<PathBuf> {
    if let Some(p) = env
        .get_path("ANDROID_HOME")
        .or_else(|| env.get_path("ANDROID_SDK_ROOT"))
    {
        return Some(p);
    }
    let home = env.home()?;
    Some(match os {
        Os::Macos => home.join("Library/Android/sdk"),
        Os::Windows => env
            .get_path("LOCALAPPDATA")
            .unwrap_or_else(|| home.join("AppData/Local"))
            .join("Android/Sdk"),
        Os::Linux | Os::Wsl => home.join("Android/Sdk"),
    })
}

/// `ANDROID_AVD_HOME` → `ANDROID_USER_HOME/avd` → `~/.android/avd`.
pub fn avd_home(env: &Env) -> Option<PathBuf> {
    env.get_path("ANDROID_AVD_HOME")
        .or_else(|| env.get_path("ANDROID_USER_HOME").map(|p| p.join("avd")))
        .or_else(|| env.home().map(|h| h.join(".android/avd")))
}

fn parse_ini(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect()
}

/// Normalized `system-images/android-35/google_apis/arm64-v8a`.
fn norm_sysdir(s: &str) -> String {
    s.replace('\\', "/").trim_matches('/').to_owned()
}

#[derive(Debug, Clone)]
pub struct Avd {
    pub name: String,
    pub display: String,
    pub ini: PathBuf,
    pub dir: PathBuf,
    pub sysdir: Option<String>,
    pub device: Option<String>,
}

pub fn read_avds(home: &Path) -> Vec<Avd> {
    let mut out = Vec::new();
    for ini in util::children(home) {
        if ini.extension().is_none_or(|e| e != "ini") || !ini.is_file() {
            continue;
        }
        let name = ini
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let meta = parse_ini(&std::fs::read_to_string(&ini).unwrap_or_default());
        let dir = meta
            .get("path")
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .unwrap_or_else(|| home.join(format!("{name}.avd")));
        if !dir.is_dir() {
            continue;
        }
        let config =
            parse_ini(&std::fs::read_to_string(dir.join("config.ini")).unwrap_or_default());
        out.push(Avd {
            display: config
                .get("avd.ini.displayname")
                .cloned()
                .unwrap_or_else(|| name.replace('_', " ")),
            sysdir: config.get("image.sysdir.1").map(|s| norm_sysdir(s)),
            device: config.get("hw.device.name").cloned(),
            name,
            ini,
            dir,
        });
    }
    out
}

/// Whether an emulator runs this AVD (`-avd <name>` or `@<name>`).
fn avd_running(procs: &ProcessSnapshot, name: &str) -> bool {
    procs.iter().any(|p| {
        let n = p.name.to_ascii_lowercase();
        (n.starts_with("qemu-system") || n.starts_with("emulator"))
            && p.args.windows(2).any(|w| w[0] == "-avd" && w[1] == name)
            || p.args.iter().any(|a| a == &format!("@{name}"))
    })
}

fn newest_mtime(dir: &Path) -> Option<LastUsed> {
    let mut newest = None;
    for p in util::children(dir) {
        let name = util::file_name(&p);
        if name.starts_with("userdata-qemu.img") || name == "snapshots" {
            if let Ok(t) = p.symlink_metadata().and_then(|m| m.modified()) {
                newest = newest.max(Some(t));
            }
        }
    }
    newest
        .and_then(|t| Timestamp::try_from(t).ok())
        .map(|at| LastUsed {
            at,
            source: LastUsedSource::ItemMtime,
        })
}

fn tool(sdk: &Path, name: &str) -> PathBuf {
    let exe = if cfg!(windows) {
        format!("{name}.bat")
    } else {
        name.to_owned()
    };
    sdk.join("cmdline-tools/latest/bin").join(exe)
}

fn find_tool(ctx: &Ctx, name: &str) -> Option<PathBuf> {
    sdk_root(&ctx.platform.env, ctx.platform.os)
        .map(|s| tool(&s, name))
        .filter(|p| p.is_file())
        .or_else(|| ctx.platform.runner.which(name))
}

fn run_tool(
    ctx: &Ctx,
    name: &str,
    args: &[String],
    item: &Item,
) -> Result<Outcome, IntegrationError> {
    let Some(program) = find_tool(ctx, name) else {
        return Ok(Outcome::Skipped {
            reason: format!("{name} isn't installed."),
        });
    };
    let mut argv = vec![program.to_string_lossy().into_owned()];
    argv.extend(args.iter().cloned());
    let out = ctx
        .platform
        .runner
        .run(&argv, None, Duration::from_secs(300))?;
    if out.success() {
        Ok(Outcome::Done {
            freed: item.size.reclaimable,
        })
    } else {
        Ok(Outcome::Failed {
            error: format!(
                "{name} failed: {}",
                out.stderr
                    .lines()
                    .chain(out.stdout.lines())
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("no output")
                    .trim()
            ),
        })
    }
}

impl Adapter for Avds {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "android-avd",
            "Android emulators",
            Category::Mobile,
            "Android Virtual Devices. Each holds the emulator's disk, apps and saved state.",
            IntegrationStatus::Stable,
            &[],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        match avd_home(&ctx.platform.env) {
            Some(h) if h.is_dir() => Detection::present(),
            _ => Detection::NotPresent,
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        avd_home(&ctx.platform.env)
            .filter(|h| h.is_dir())
            .into_iter()
            .collect()
    }

    fn tools(&self) -> Vec<String> {
        vec!["avdmanager".into()]
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(home) = avd_home(&ctx.platform.env) else {
            return Ok(());
        };
        let has_tool = find_tool(ctx, "avdmanager").is_some();
        for avd in read_avds(&home) {
            let running = avd_running(ctx.processes, &avd.name);
            let snapshots = avd.dir.join("snapshots");
            let mut d = ItemDraft::new(
                "android_avd",
                format!("{} emulator", avd.display),
                Locator::Paths {
                    paths: vec![avd.ini.clone(), avd.dir.clone()],
                },
            );
            d.safety = Safety::Review;
            d.reasons = vec![reason(ReasonCode::ContainsUserData)];
            d.actions = if has_tool {
                vec![ActionKind::native(AVD_DELETE), ActionKind::Quarantine]
            } else {
                vec![ActionKind::Quarantine]
            };
            d.last_used = newest_mtime(&avd.dir).or_else(|| util::mtime_used(&avd.dir));
            d.details = serde_json::json!({ "name": avd.name, "system_image": avd.sysdir, "device": avd.device });
            if running {
                d.safety = Safety::Keep;
                d.reasons.insert(
                    0,
                    reason_with(
                        ReasonCode::ToolRunning,
                        &[("tool", format!("The {} emulator", avd.display))],
                    ),
                );
                d.actions.clear();
            }
            if snapshots.is_dir() && !running {
                // Size without the snapshots, which are their own item.
                let cancel = Default::default();
                let (all, _) = measure(&[avd.ini.clone(), avd.dir.clone()], &cancel).finish();
                let (snap, _) = measure(std::slice::from_ref(&snapshots), &cancel).finish();
                d.size = SizeSpec::Known(Size {
                    apparent: all.apparent,
                    on_disk: all.on_disk,
                    reclaimable: all.reclaimable.saturating_sub(snap.reclaimable),
                    file_count: all.file_count,
                });
                d.notes.push("Its Quick Boot snapshots are listed separately; removing the emulator removes them too.".into());
                let mut s = ItemDraft::new(
                    "android_avd_snapshots",
                    format!("{} Quick Boot snapshots", avd.display),
                    Locator::path(&snapshots),
                );
                s.safety = Safety::Review;
                s.reasons = vec![Reason {
                    code: ReasonCode::Regenerable,
                    text: "The emulator makes new snapshots, but cold boots once and loses the saved state.".into(),
                }];
                s.actions = vec![ActionKind::Quarantine];
                s.last_used = util::mtime_used(&snapshots);
                sink.item(s);
            }
            sink.item(d);
        }
        Ok(())
    }

    fn preflight(&self, ctx: &Ctx, item: &Item, _action: &ActionKind) -> Vec<Check> {
        let name = item
            .details
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if avd_running(&ctx.platform.processes.snapshot(), name) {
            vec![Check::fail("closed", "Close the emulator first.")]
        } else {
            Vec::new()
        }
    }

    fn apply(
        &self,
        ctx: &Ctx,
        item: &Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        let ActionKind::Native { command_id } = action else {
            return Err(IntegrationError::Unsupported(format!(
                "can't run `{action}`"
            )));
        };
        if command_id != AVD_DELETE {
            return Err(IntegrationError::other(format!(
                "unknown command `{command_id}`"
            )));
        }
        let name = item
            .details
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IntegrationError::other("item has no AVD name"))?;
        run_tool(
            ctx,
            "avdmanager",
            &["delete".into(), "avd".into(), "-n".into(), name.into()],
            item,
        )
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        matches!(action, ActionKind::Native { command_id } if command_id == AVD_DELETE)
            .then(|| "Runs `avdmanager delete avd`, the SDK's own way to remove an emulator. This can't be undone.".into())
    }
}

/// Every installed system image: (`system-images/…` relative key, path).
fn system_images(sdk: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for api in util::children(&sdk.join("system-images")) {
        for tag in util::children(&api) {
            for abi in util::children(&tag) {
                if abi.is_dir() {
                    let key = format!(
                        "system-images/{}/{}/{}",
                        util::file_name(&api),
                        util::file_name(&tag),
                        util::file_name(&abi)
                    );
                    out.push((key, abi));
                }
            }
        }
    }
    out
}

impl Adapter for SystemImages {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "android-system-images",
            "Android system images",
            Category::Mobile,
            "Android OS images emulators boot from. Images no emulator uses can go; the SDK manager downloads them again.",
            IntegrationStatus::Stable,
            &[],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        match sdk_root(&ctx.platform.env, ctx.platform.os) {
            Some(s) if s.join("system-images").is_dir() => Detection::present(),
            _ => Detection::NotPresent,
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        sdk_root(&ctx.platform.env, ctx.platform.os)
            .map(|s| s.join("system-images"))
            .filter(|p| p.is_dir())
            .into_iter()
            .collect()
    }

    fn tools(&self) -> Vec<String> {
        vec!["sdkmanager".into()]
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(sdk) = sdk_root(&ctx.platform.env, ctx.platform.os) else {
            return Ok(());
        };
        let avds = avd_home(&ctx.platform.env)
            .map(|h| read_avds(&h))
            .unwrap_or_default();
        let has_tool = find_tool(ctx, "sdkmanager").is_some();
        for (key, path) in system_images(&sdk) {
            let users: Vec<&str> = avds
                .iter()
                .filter(|a| a.sysdir.as_deref() == Some(key.as_str()))
                .map(|a| a.display.as_str())
                .collect();
            let package = key.replace('/', ";");
            let mut d = ItemDraft::new(
                "android_system_image",
                format!(
                    "Android image {}",
                    key.trim_start_matches("system-images/").replace('/', " ")
                ),
                Locator::path(&path),
            );
            d.last_used = util::mtime_used(&path);
            d.details = serde_json::json!({ "package": package, "used_by": users });
            d.restore_hint = Some(format!("sdkmanager \"{package}\""));
            if users.is_empty() {
                d.safety = Safety::Safe;
                d.reasons = vec![
                    reason(ReasonCode::UnusedByAnyEmulator),
                    reason(ReasonCode::Redownloadable),
                ];
                d.actions = if has_tool {
                    vec![ActionKind::native(SDK_UNINSTALL), ActionKind::Quarantine]
                } else {
                    vec![ActionKind::Quarantine]
                };
            } else {
                d.safety = Safety::Keep;
                d.reasons = vec![reason_with(
                    ReasonCode::UsedByEmulator,
                    &[("names", users.join(", "))],
                )];
            }
            sink.item(d);
        }
        Ok(())
    }

    fn apply(
        &self,
        ctx: &Ctx,
        item: &Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        let ActionKind::Native { command_id } = action else {
            return Err(IntegrationError::Unsupported(format!(
                "can't run `{action}`"
            )));
        };
        if command_id != SDK_UNINSTALL {
            return Err(IntegrationError::other(format!(
                "unknown command `{command_id}`"
            )));
        }
        let package = item
            .details
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IntegrationError::other("item has no package"))?;
        run_tool(
            ctx,
            "sdkmanager",
            &["--uninstall".into(), package.into()],
            item,
        )
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        matches!(action, ActionKind::Native { command_id } if command_id == SDK_UNINSTALL)
            .then(|| "Runs `sdkmanager --uninstall`, the SDK's own way to remove an image. It downloads again with sdkmanager.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kipple_core::platform::ProcessInfo;

    #[test]
    fn detects_running_emulators() {
        let snap = ProcessSnapshot::new(vec![ProcessInfo {
            pid: 1,
            name: "qemu-system-aarch64".into(),
            args: vec!["-avd".into(), "Pixel_8".into()],
        }]);
        assert!(avd_running(&snap, "Pixel_8"));
        assert!(!avd_running(&snap, "Pixel_9"));
        let snap = ProcessSnapshot::new(vec![ProcessInfo {
            pid: 1,
            name: "emulator".into(),
            args: vec!["@Tab".into()],
        }]);
        assert!(avd_running(&snap, "Tab"));
    }

    #[test]
    fn normalizes_sysdirs() {
        assert_eq!(
            norm_sysdir("system-images\\android-35\\google_apis\\arm64-v8a\\"),
            "system-images/android-35/google_apis/arm64-v8a"
        );
    }
}
