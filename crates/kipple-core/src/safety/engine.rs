//! Safety evaluation (SPEC §7.1): hard rules, integration rule, age rule,
//! user overrides.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use globset::Glob;

use crate::config::SafetyConfig;
use crate::model::{
    Category, IntegrationId, IntegrationInfo, IntegrationStatus, Item, LastUsed, Reason,
    ReasonCode, Safety, Timestamp,
};
use crate::platform::ProcessSnapshot;
use crate::platform::paths;

use super::protected::{ProtectedSet, Protection};
use super::reasons::{reason, reason_with};

/// Items modified this recently are `Keep` (SPEC §7.2).
pub const RECENT_WRITE: Duration = Duration::from_secs(10 * 60);

/// Marker files whose presence at an item's own level means it holds source.
const SOURCE_MARKERS: &[&str] = &[
    ".git",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
];

/// Shared inputs for evaluating every item of a scan.
pub struct SafetyContext<'a> {
    pub protected: &'a ProtectedSet,
    /// Scan roots plus integration-declared locations.
    pub allowed_roots: &'a [PathBuf],
    pub processes: &'a ProcessSnapshot,
    pub config: &'a SafetyConfig,
    pub now: Timestamp,
    /// Opt-in integrations the user enabled.
    pub opted_in: &'a HashSet<IntegrationId>,
}

/// Per-item facts the engine needs beyond the item itself.
pub struct ItemFacts<'a> {
    pub info: &'a IntegrationInfo,
    pub unknown_format: bool,
    /// Project artifact subject to the age rule.
    pub age_rule: bool,
    /// Matched by the walk inside a project.
    pub from_walk: bool,
    pub project_last_used: Option<LastUsed>,
    pub newest_mtime: Option<SystemTime>,
}

fn days_since(now: Timestamp, then: Timestamp) -> i64 {
    (now.as_second() - then.as_second()).max(0) / 86_400
}

#[cfg(unix)]
fn parent_writable(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Some(parent) = path.parent() else {
        return false;
    };
    let Ok(c) = std::ffi::CString::new(parent.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: `c` is a valid NUL-terminated path; access() only reads it.
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

#[cfg(not(unix))]
fn parent_writable(path: &Path) -> bool {
    path.parent()
        .and_then(|p| p.metadata().ok())
        .is_some_and(|m| !m.permissions().readonly())
}

fn holds_source(path: &Path) -> bool {
    SOURCE_MARKERS
        .iter()
        .any(|m| path.join(m).symlink_metadata().is_ok())
}

/// Evaluates `item` in place: safety, reasons and actions.
pub fn evaluate(item: &mut Item, facts: &ItemFacts, ctx: &SafetyContext) {
    if let Some(hard) = hard_rule(item, facts, ctx) {
        item.safety = Safety::Keep;
        item.reasons.insert(0, hard);
        item.actions.clear();
        finish(item);
        return;
    }

    // Integration rule is already in the item (from the draft). Experimental
    // integrations are capped at Review.
    if facts.info.status == IntegrationStatus::Experimental && item.safety == Safety::Safe {
        item.safety = Safety::Review;
        item.reasons.push(reason(ReasonCode::Experimental));
    }

    // Age rule for project artifacts.
    if facts.age_rule && item.safety != Safety::Keep {
        let recent = i64::from(ctx.config.recent_days);
        match facts.project_last_used {
            Some(lu) => {
                let days = days_since(ctx.now, lu.at);
                if days < recent {
                    if item.safety == Safety::Safe {
                        item.safety = Safety::Review;
                    }
                    item.reasons.push(reason_with(
                        ReasonCode::RecentlyUsed,
                        &[("days", recent.to_string())],
                    ));
                } else {
                    item.reasons.push(reason_with(
                        ReasonCode::InactiveProject,
                        &[("days", days.to_string())],
                    ));
                }
            }
            None if item.project.is_none() => {}
            None => {}
        }
    }

    apply_overrides(item, ctx);
    finish(item);
}

fn hard_rule(item: &Item, facts: &ItemFacts, ctx: &SafetyContext) -> Option<Reason> {
    for path in item.locator.paths() {
        match ctx.protected.check(path) {
            Protection::Protected { label } | Protection::ContainsProtected { label } => {
                return Some(reason_with(ReasonCode::ProtectedPath, &[("path", label)]));
            }
            Protection::InsideFolder { label } => {
                let allowed = matches!(
                    item.category,
                    Category::ProjectArtifact | Category::BuildCache
                ) || ctx.opted_in.contains(&item.integration);
                if !allowed {
                    return Some(reason_with(ReasonCode::ProtectedPath, &[("path", label)]));
                }
            }
            Protection::None => {}
        }
        let lexical = paths::lexical_normalize(path);
        let inside = ctx
            .allowed_roots
            .iter()
            .any(|r| paths::is_within(&lexical, r) && !paths::is_within(r, &lexical));
        if !inside {
            return Some(reason(ReasonCode::OutsideRoots));
        }
        if lexical
            .components()
            .any(|c| c.as_os_str().eq_ignore_ascii_case(".git"))
            || (facts.from_walk && holds_source(path))
            || path.join(".git").symlink_metadata().is_ok()
        {
            return Some(reason(ReasonCode::ContainsSource));
        }
    }
    if facts.unknown_format {
        return Some(reason(ReasonCode::UnknownFormat));
    }
    for pattern in &facts.info.in_use_processes {
        if ctx.processes.any_running(pattern) {
            let tool = pattern.split('|').next().unwrap_or(pattern).to_owned();
            return Some(reason_with(ReasonCode::ToolRunning, &[("tool", tool)]));
        }
    }
    if let Some(mtime) = facts.newest_mtime {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(ctx.now.as_second().max(0) as u64);
        if now
            .duration_since(mtime)
            .is_ok_and(|age| age < RECENT_WRITE)
            || mtime > now + Duration::from_secs(60)
        {
            return Some(reason(ReasonCode::RecentlyWritten));
        }
    }
    if let Some(path) = item.locator.primary_path() {
        if !item.actions.is_empty() && !parent_writable(path) {
            return Some(reason(ReasonCode::NeedsElevation));
        }
    }
    None
}

fn apply_overrides(item: &mut Item, ctx: &SafetyContext) {
    for o in &ctx.config.overrides {
        if o.integration
            .as_ref()
            .is_some_and(|i| i != item.integration.as_str())
        {
            continue;
        }
        if o.kind.as_ref().is_some_and(|k| k != &item.kind) {
            continue;
        }
        if let Some(g) = &o.glob {
            let Ok(glob) = Glob::new(g) else { continue };
            let m = glob.compile_matcher();
            if !item.locator.paths().iter().any(|p| m.is_match(p)) {
                continue;
            }
        }
        if o.safety > item.safety || (o.allow_relax && o.safety != item.safety) {
            item.safety = o.safety;
            item.reasons.insert(0, reason(ReasonCode::UserOverride));
        }
    }
}

fn finish(item: &mut Item) {
    if item.safety == Safety::Keep {
        item.actions.clear();
    }
    if item.actions.is_empty() && item.safety != Safety::Keep {
        item.safety = Safety::Keep;
    }
    if item.reasons.is_empty() {
        item.reasons.push(reason(ReasonCode::UnknownFormat));
    }
    item.reasons.dedup_by(|a, b| a.code == b.code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dirs::Dirs;
    use crate::model::*;
    use crate::platform::{ProcessInfo, os::current_os};
    use crate::testing::TestHome;

    struct Fixture {
        home: TestHome,
        protected: ProtectedSet,
        roots: Vec<PathBuf>,
        config: SafetyConfig,
        info: IntegrationInfo,
    }

    fn fixture() -> Fixture {
        let home = TestHome::new().unwrap();
        let dirs = Dirs::resolve(home.env(), current_os()).unwrap();
        let roots = vec![home.path().to_path_buf()];
        let protected = ProtectedSet::new(home.env(), current_os(), &dirs, &[], &roots);
        Fixture {
            home,
            protected,
            roots,
            config: SafetyConfig::default(),
            info: IntegrationInfo {
                id: IntegrationId::from("node-modules"),
                name: "node_modules".into(),
                kind: IntegrationKind::Pack,
                category: Category::ProjectArtifact,
                description: String::new(),
                docs_url: None,
                status: IntegrationStatus::Stable,
                os: vec![],
                in_use_processes: vec![],
            },
        }
    }

    fn item(path: PathBuf, safety: Safety) -> Item {
        let integration = IntegrationId::from("node-modules");
        let locator = Locator::Path { path };
        Item {
            id: ItemId::new(&integration, &locator),
            integration,
            category: Category::ProjectArtifact,
            kind: "node_modules".into(),
            title: "node_modules".into(),
            locator,
            size: Size::default(),
            project: None,
            last_used: None,
            safety,
            reasons: vec![reason(ReasonCode::Regenerable)],
            actions: vec![ActionKind::Quarantine],
            details: serde_json::Value::Null,
            notes: vec![],
            restore_hint: None,
            identity: None,
        }
    }

    fn eval(f: &Fixture, it: &mut Item, facts: ItemFacts, procs: &ProcessSnapshot) {
        let opted = HashSet::new();
        let ctx = SafetyContext {
            protected: &f.protected,
            allowed_roots: &f.roots,
            processes: procs,
            config: &f.config,
            now: Timestamp::now(),
            opted_in: &opted,
        };
        evaluate(it, &facts, &ctx);
    }

    fn facts(f: &Fixture) -> ItemFacts<'_> {
        ItemFacts {
            info: &f.info,
            unknown_format: false,
            age_rule: true,
            from_walk: true,
            project_last_used: None,
            newest_mtime: None,
        }
    }

    fn ago(days: i64) -> LastUsed {
        LastUsed {
            at: Timestamp::from_second(Timestamp::now().as_second() - days * 86_400).unwrap(),
            source: LastUsedSource::GitCommit,
        }
    }

    #[test]
    fn inactive_project_stays_safe() {
        let f = fixture();
        let p = f.home.mkdir("code/app/node_modules").unwrap();
        let mut it = item(p, Safety::Safe);
        let mut fa = facts(&f);
        fa.project_last_used = Some(ago(40));
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Safe);
        assert!(it.has_reason(ReasonCode::InactiveProject));
    }

    #[test]
    fn recent_project_downgrades_to_review() {
        let f = fixture();
        let p = f.home.mkdir("code/app/node_modules").unwrap();
        let mut it = item(p, Safety::Safe);
        let mut fa = facts(&f);
        fa.project_last_used = Some(ago(3));
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Review);
        assert!(it.has_reason(ReasonCode::RecentlyUsed));
        assert!(!it.actions.is_empty());
    }

    #[test]
    fn protected_paths_are_keep_without_actions() {
        let f = fixture();
        let p = f.home.mkdir(".ssh/node_modules").unwrap();
        let mut it = item(p, Safety::Safe);
        eval(&f, &mut it, facts(&f), &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Keep);
        assert!(it.actions.is_empty());
        assert_eq!(it.reasons[0].code, ReasonCode::ProtectedPath);
    }

    #[test]
    fn caches_in_personal_folders_are_keep_but_artifacts_are_not() {
        let f = fixture();
        let p = f.home.mkdir("Desktop/dev/app/node_modules").unwrap();
        let mut it = item(p.clone(), Safety::Safe);
        let mut fa = facts(&f);
        fa.project_last_used = Some(ago(90));
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Safe);

        let mut cache = item(p, Safety::Safe);
        cache.category = Category::PackageCache;
        eval(&f, &mut cache, facts(&f), &ProcessSnapshot::default());
        assert_eq!(cache.safety, Safety::Keep);
    }

    #[test]
    fn outside_roots_is_keep() {
        let f = fixture();
        let mut it = item(
            std::env::temp_dir().join("elsewhere/node_modules"),
            Safety::Safe,
        );
        eval(&f, &mut it, facts(&f), &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Keep);
        assert_eq!(it.reasons[0].code, ReasonCode::OutsideRoots);
    }

    #[test]
    fn git_repos_are_keep() {
        let f = fixture();
        let p = f.home.mkdir("code/app/node_modules/.git").unwrap();
        let mut it = item(p.parent().unwrap().to_path_buf(), Safety::Safe);
        eval(&f, &mut it, facts(&f), &ProcessSnapshot::default());
        assert_eq!(it.reasons[0].code, ReasonCode::ContainsSource);
    }

    #[test]
    fn running_tool_is_keep() {
        let mut f = fixture();
        f.info.in_use_processes = vec!["Cursor".into()];
        let p = f.home.mkdir("code/app/node_modules").unwrap();
        let mut it = item(p, Safety::Review);
        let procs = ProcessSnapshot::new(vec![ProcessInfo {
            pid: 1,
            name: "Cursor".into(),
            args: vec![],
        }]);
        eval(&f, &mut it, facts(&f), &procs);
        assert_eq!(it.safety, Safety::Keep);
        assert_eq!(
            it.reasons[0].text,
            "Cursor is running. Quit it first, then scan again."
        );
    }

    #[test]
    fn recently_written_is_keep() {
        let f = fixture();
        let p = f.home.mkdir("code/app/node_modules").unwrap();
        let mut it = item(p, Safety::Safe);
        let mut fa = facts(&f);
        fa.newest_mtime = Some(SystemTime::now() - Duration::from_secs(30));
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.reasons[0].code, ReasonCode::RecentlyWritten);
    }

    #[test]
    fn unknown_format_is_keep() {
        let f = fixture();
        let p = f.home.mkdir("code/app/node_modules").unwrap();
        let mut it = item(p, Safety::Review);
        let mut fa = facts(&f);
        fa.unknown_format = true;
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Keep);
    }

    #[test]
    fn experimental_caps_at_review() {
        let mut f = fixture();
        f.info.status = IntegrationStatus::Experimental;
        let p = f.home.mkdir("code/app/node_modules").unwrap();
        let mut it = item(p, Safety::Safe);
        let mut fa = facts(&f);
        fa.age_rule = false;
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Review);
        assert!(it.has_reason(ReasonCode::Experimental));
    }

    #[test]
    fn overrides_only_tighten_unless_allowed() {
        let mut f = fixture();
        let p = f.home.mkdir("code/app/node_modules").unwrap();
        f.config.overrides = vec![crate::config::SafetyOverride {
            integration: Some("node-modules".into()),
            kind: None,
            glob: None,
            safety: Safety::Safe,
            allow_relax: false,
        }];
        let mut it = item(p.clone(), Safety::Review);
        let mut fa = facts(&f);
        fa.age_rule = false;
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Review, "relaxing needs allow_relax");

        f.config.overrides[0].safety = Safety::Keep;
        let mut it = item(p, Safety::Safe);
        let mut fa = facts(&f);
        fa.age_rule = false;
        eval(&f, &mut it, fa, &ProcessSnapshot::default());
        assert_eq!(it.safety, Safety::Keep);
        assert!(it.actions.is_empty());
        assert!(it.has_reason(ReasonCode::UserOverride));
    }
}
