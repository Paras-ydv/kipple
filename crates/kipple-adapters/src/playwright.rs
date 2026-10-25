//! `playwright`: browsers downloaded by Playwright (INTEGRATIONS.md §4.8).
//!
//! Layout (verified 2026-10-25): `<browsers>/<name>-<revision>/`, e.g.
//! `chromium-1187`, `chromium_headless_shell-1187`, `firefox-1489`,
//! `webkit-2090`, `ffmpeg-1011`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink};
use kipple_core::model::{
    ActionKind, Category, Detection, IntegrationInfo, IntegrationStatus, Locator, Os, Reason,
    ReasonCode, Safety,
};
use kipple_core::platform::Env;
use kipple_core::safety::reasons::reason;

use crate::util;

pub struct Playwright;

/// Browser folder: env override, then OS default. `PLAYWRIGHT_BROWSERS_PATH=0`
/// means browsers live inside each project's node_modules.
pub fn browsers_dir(env: &Env, os: Os) -> Option<PathBuf> {
    if let Some(v) = env.get_str("PLAYWRIGHT_BROWSERS_PATH") {
        return (v != "0").then(|| PathBuf::from(v));
    }
    let home = env.home()?;
    Some(match os {
        Os::Macos => home.join("Library/Caches/ms-playwright"),
        Os::Windows => env
            .get_path("LOCALAPPDATA")
            .unwrap_or_else(|| home.join("AppData/Local"))
            .join("ms-playwright"),
        Os::Linux | Os::Wsl => env
            .get_path("XDG_CACHE_HOME")
            .unwrap_or_else(|| home.join(".cache"))
            .join("ms-playwright"),
    })
}

/// `chromium_headless_shell-1187` → (`chromium_headless_shell`, 1187).
fn parse(name: &str) -> Option<(String, u64)> {
    let (browser, rev) = name.rsplit_once('-')?;
    Some((browser.to_owned(), rev.parse().ok()?))
}

impl Adapter for Playwright {
    fn info(&self) -> IntegrationInfo {
        util::info(
            "playwright",
            "Playwright browsers",
            Category::PackageCache,
            "Browsers Playwright downloaded for testing. Older revisions aren't used by your current Playwright.",
            IntegrationStatus::Stable,
            &[],
            &[],
        )
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        match browsers_dir(&ctx.platform.env, ctx.platform.os) {
            Some(d) if d.is_dir() => Detection::present(),
            _ => Detection::NotPresent,
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        browsers_dir(&ctx.platform.env, ctx.platform.os)
            .filter(|d| d.is_dir())
            .into_iter()
            .collect()
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(dir) = browsers_dir(&ctx.platform.env, ctx.platform.os) else {
            return Ok(());
        };
        let mut by_browser: BTreeMap<String, Vec<(u64, PathBuf)>> = BTreeMap::new();
        for child in util::children(&dir) {
            if !child.is_dir() {
                continue;
            }
            if let Some((browser, rev)) = parse(&util::file_name(&child)) {
                by_browser.entry(browser).or_default().push((rev, child));
            }
        }
        for (browser, mut revs) in by_browser {
            revs.sort();
            let newest = revs.last().map(|(r, _)| *r);
            for (rev, path) in revs {
                sink.item(draft(&browser, rev, &path, Some(rev) == newest));
            }
        }
        Ok(())
    }
}

fn draft(browser: &str, rev: u64, path: &Path, newest: bool) -> ItemDraft {
    let label = browser.replace('_', " ");
    let mut d = ItemDraft::new(
        "playwright_browser",
        format!("Playwright {label} {rev}"),
        Locator::path(path),
    );
    d.last_used = util::mtime_used(path);
    d.actions = vec![ActionKind::Quarantine];
    d.restore_hint = Some("npx playwright install".into());
    d.details = serde_json::json!({ "browser": browser, "revision": rev });
    d.reasons = vec![reason(ReasonCode::Redownloadable)];
    if newest {
        d.safety = Safety::Review;
        d.reasons.insert(
            0,
            Reason {
                code: ReasonCode::RecentlyUsed,
                text: "This is the newest revision you have, which your Playwright probably uses."
                    .into(),
            },
        );
    } else {
        d.safety = Safety::Safe;
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_folder_names() {
        assert_eq!(parse("chromium-1187"), Some(("chromium".into(), 1187)));
        assert_eq!(
            parse("chromium_headless_shell-1187"),
            Some(("chromium_headless_shell".into(), 1187))
        );
        assert_eq!(parse(".links"), None);
        assert_eq!(parse("chromium-tip-of-tree"), None);
    }

    #[test]
    fn zero_means_no_shared_folder() {
        let env = Env::from_pairs([("HOME", "/h"), ("PLAYWRIGHT_BROWSERS_PATH", "0")]);
        assert_eq!(browsers_dir(&env, Os::Linux), None);
    }
}
