//! `xcode-derived-data` and `ios-device-support` (INTEGRATIONS.md §4.7,
//! macOS only).

use std::path::{Path, PathBuf};

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink};
use kipple_core::model::{
    ActionKind, Category, Detection, IntegrationInfo, IntegrationStatus, Locator, Os, Reason,
    ReasonCode, Safety,
};
use kipple_core::safety::reasons::{reason, reason_with};

use crate::util;

pub struct DerivedData;
pub struct DeviceSupport;

fn derived_data_dir(ctx: &Ctx) -> Option<PathBuf> {
    let custom = ctx.command_path(&[
        "defaults".into(),
        "read".into(),
        "com.apple.dt.Xcode".into(),
        "IDECustomDerivedDataLocation".into(),
    ]);
    custom.or_else(|| {
        ctx.home()
            .map(|h| h.join("Library/Developer/Xcode/DerivedData"))
    })
}

/// `WorkspacePath` from a DerivedData `info.plist` (XML plists only; binary
/// ones are left unattributed).
pub fn workspace_path(info_plist: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(info_plist).ok()?;
    let after = text.split("<key>WorkspacePath</key>").nth(1)?;
    let start = after.find("<string>")? + "<string>".len();
    let end = after[start..].find("</string>")? + start;
    let raw = after[start..end].trim();
    let unescaped = raw
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&apos;", "'")
        .replace("&quot;", "\"");
    Some(PathBuf::from(unescaped))
}

fn xcode_running(ctx: &Ctx) -> bool {
    ctx.processes.any_running("Xcode")
}

impl Adapter for DerivedData {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "xcode-derived-data",
            "Xcode DerivedData",
            Category::Mobile,
            "Xcode's build products and indexes. Xcode rebuilds them on the next build.",
            IntegrationStatus::Stable,
            &[Os::Macos],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        match derived_data_dir(ctx) {
            Some(d) if d.is_dir() => Detection::present(),
            _ => Detection::NotPresent,
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        derived_data_dir(ctx)
            .filter(|d| d.is_dir())
            .into_iter()
            .collect()
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(dir) = derived_data_dir(ctx) else {
            return Ok(());
        };
        let running = xcode_running(ctx);
        for child in util::children(&dir) {
            if !child.is_dir() {
                continue;
            }
            let name = util::file_name(&child);
            let workspace = workspace_path(&child.join("info.plist"));
            let title = match name.rsplit_once('-') {
                Some((project, hash)) if hash.len() >= 20 => {
                    format!("{project} build data (Xcode)")
                }
                _ if name == "ModuleCache.noindex" => "Xcode module cache".to_owned(),
                _ => format!("{name} (Xcode)"),
            };
            let mut d = ItemDraft::new("xcode_derived_data", title, Locator::path(&child));
            d.project_hint = workspace
                .as_ref()
                .and_then(|w| w.parent().map(Path::to_path_buf));
            d.last_used = util::mtime_used(&child);
            d.reasons = vec![reason(ReasonCode::Regenerable)];
            d.actions = vec![ActionKind::Quarantine];
            if let Some(w) = &workspace {
                d.details = serde_json::json!({ "workspace": w });
            }
            if running {
                d.safety = Safety::Review;
                d.reasons.insert(
                    0,
                    reason_with(ReasonCode::ToolRunning, &[("tool", "Xcode".into())]),
                );
            } else {
                d.safety = Safety::Safe;
            }
            sink.item(d);
        }
        Ok(())
    }
}

const SUPPORT_DIRS: &[&str] = &[
    "iOS DeviceSupport",
    "watchOS DeviceSupport",
    "tvOS DeviceSupport",
    "visionOS DeviceSupport",
];

/// First dotted number in a folder name: `17.4 (21E219)`,
/// `iPhone15,2 17.4 (21E219)` → [17, 4].
pub fn os_version(name: &str) -> Vec<u32> {
    name.split_whitespace()
        .find(|tok| {
            tok.contains('.')
                && tok
                    .split('.')
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        })
        .map(|tok| tok.split('.').filter_map(|p| p.parse().ok()).collect())
        .unwrap_or_default()
}

impl Adapter for DeviceSupport {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "ios-device-support",
            "Device support files",
            Category::Mobile,
            "Symbols Xcode copies from each iPhone, Watch or TV OS version you debug on. Xcode copies them again when you connect a device.",
            IntegrationStatus::Stable,
            &[Os::Macos],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        if self.locations(ctx).is_empty() {
            Detection::NotPresent
        } else {
            Detection::present()
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        let Some(home) = ctx.home() else {
            return Vec::new();
        };
        SUPPORT_DIRS
            .iter()
            .map(|d| home.join("Library/Developer/Xcode").join(d))
            .filter(|p| p.is_dir())
            .collect()
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        for base in self.locations(ctx) {
            let platform = util::file_name(&base).replace(" DeviceSupport", "");
            let mut versions: Vec<(Vec<u32>, PathBuf)> = util::children(&base)
                .into_iter()
                .filter(|p| p.is_dir())
                .map(|p| (os_version(&util::file_name(&p)), p))
                .collect();
            versions.sort();
            let newest = versions.last().map(|(_, p)| p.clone());
            for (_, path) in versions {
                let name = util::file_name(&path);
                let mut d = ItemDraft::new(
                    "device_support",
                    format!("{platform} {name} device support"),
                    Locator::path(&path),
                );
                d.last_used = util::mtime_used(&path);
                d.actions = vec![ActionKind::Quarantine];
                d.reasons = vec![reason(ReasonCode::Redownloadable)];
                if Some(&path) == newest.as_ref() {
                    d.safety = Safety::Review;
                    d.reasons.insert(
                        0,
                        Reason {
                            code: ReasonCode::RecentlyUsed,
                            text: format!(
                                "This is the newest {platform} version you have debugged on."
                            ),
                        },
                    );
                } else {
                    d.safety = Safety::Safe;
                }
                sink.item(d);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_workspace_path() {
        let home = kipple_core::testing::TestHome::new().unwrap();
        let plist = home
            .write(
                "info.plist",
                "<?xml version=\"1.0\"?><plist><dict><key>LastAccessedDate</key><date>2026-01-01T00:00:00Z</date>\n<key>WorkspacePath</key>\n\t<string>/Users/a/code/A&amp;B/App.xcodeproj</string></dict></plist>",
            )
            .unwrap();
        assert_eq!(
            workspace_path(&plist),
            Some(PathBuf::from("/Users/a/code/A&B/App.xcodeproj"))
        );
    }

    #[test]
    fn parses_os_versions() {
        assert_eq!(os_version("17.4 (21E219)"), vec![17, 4]);
        assert_eq!(os_version("iPhone15,2 17.4.1 (21E236)"), vec![17, 4, 1]);
        assert_eq!(os_version("junk"), Vec::<u32>::new());
    }
}
