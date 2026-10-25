//! The integration interface (SPEC §9.2). Packs and adapters both implement
//! [`Adapter`]; the core knows nothing about specific tools.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::config::Config;
use crate::dirs::Dirs;
use crate::model::{
    ActionKind, CancelToken, Category, Check, Detection, IntegrationId, IntegrationInfo, LastUsed,
    Locator, Outcome, Reason, Safety, Size,
};
use crate::platform::{Platform, ProcessSnapshot};

#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Command(#[from] crate::platform::CommandError),
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Other(String),
}

impl IntegrationError {
    pub fn other(msg: impl Into<String>) -> Self {
        IntegrationError::Other(msg.into())
    }
}

/// Everything an integration may use. Built once per scan or execution.
pub struct Ctx<'a> {
    pub platform: &'a Platform,
    pub config: &'a Config,
    pub dirs: &'a Dirs,
    pub cancel: &'a CancelToken,
    /// Processes running when the scan started.
    pub processes: &'a ProcessSnapshot,
    memo: Mutex<HashMap<Vec<String>, Option<PathBuf>>>,
}

impl<'a> Ctx<'a> {
    pub fn new(
        platform: &'a Platform,
        config: &'a Config,
        dirs: &'a Dirs,
        cancel: &'a CancelToken,
        processes: &'a ProcessSnapshot,
    ) -> Self {
        Self {
            platform,
            config,
            dirs,
            cancel,
            processes,
            memo: Mutex::new(HashMap::new()),
        }
    }

    pub fn home(&self) -> Option<PathBuf> {
        self.platform.env.home()
    }

    /// Runs `argv` once per context and returns its stdout as a path
    /// (`npm config get cache`, `go env GOCACHE`). Memoized.
    pub fn command_path(&self, argv: &[String]) -> Option<PathBuf> {
        if let Ok(memo) = self.memo.lock()
            && let Some(hit) = memo.get(argv)
        {
            return hit.clone();
        }
        let result = self
            .platform
            .runner
            .output_path(argv, Duration::from_secs(10));
        if let Ok(mut memo) = self.memo.lock() {
            memo.insert(argv.to_vec(), result.clone());
        }
        result
    }

    /// Opens a third-party SQLite database strictly read-only (SPEC §9.2,
    /// INTEGRATIONS.md §3). `immutable=1` also stops SQLite from creating
    /// `-wal`/`-shm` files next to it.
    pub fn open_readonly_immutable(&self, path: &Path) -> rusqlite::Result<rusqlite::Connection> {
        open_readonly_immutable(path)
    }
}

/// See [`Ctx::open_readonly_immutable`].
pub fn open_readonly_immutable(path: &Path) -> rusqlite::Result<rusqlite::Connection> {
    use rusqlite::OpenFlags;
    let mut uri = String::from("file:");
    for c in path.to_string_lossy().chars() {
        match c {
            '?' => uri.push_str("%3f"),
            '#' => uri.push_str("%23"),
            '%' => uri.push_str("%25"),
            '\\' => uri.push('/'),
            c => uri.push(c),
        }
    }
    uri.push_str("?mode=ro&immutable=1");
    rusqlite::Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
}

/// How the core should size an item.
#[derive(Debug, Clone, PartialEq)]
pub enum SizeSpec {
    /// Walk the locator's paths.
    Measure,
    /// The integration knows the size (Docker API, SQLite stats, …).
    Known(Size),
}

/// An item as reported by an integration, before the core assigns its id,
/// identity, measured size, project and final safety.
#[derive(Debug, Clone)]
pub struct ItemDraft {
    pub kind: String,
    pub title: String,
    /// Defaults to the integration's category.
    pub category: Option<Category>,
    pub locator: Locator,
    pub size: SizeSpec,
    /// Project directory from tool metadata (session `cwd`, compose dir).
    pub project_hint: Option<PathBuf>,
    pub last_used: Option<LastUsed>,
    pub safety: Safety,
    pub reasons: Vec<Reason>,
    pub actions: Vec<ActionKind>,
    pub details: serde_json::Value,
    pub notes: Vec<String>,
    pub restore_hint: Option<String>,
    /// Storage version not recognised; forces Keep.
    pub unknown_format: bool,
    /// Apply the project age rule (project artifacts).
    pub age_rule: bool,
    /// Merge with other drafts sharing this key in the same project
    /// (`aggregate = "project"`). Value is the title stem.
    pub aggregate: Option<String>,
    /// Drop the item when its reclaimable size is below this.
    pub min_size: Option<u64>,
    /// Safety when the item isn't inside any project (e.g. a standalone
    /// virtualenv someone may rely on).
    pub safety_outside_project: Option<Safety>,
}

impl ItemDraft {
    pub fn new(kind: impl Into<String>, title: impl Into<String>, locator: Locator) -> Self {
        Self {
            kind: kind.into(),
            title: title.into(),
            category: None,
            locator,
            size: SizeSpec::Measure,
            project_hint: None,
            last_used: None,
            safety: Safety::Review,
            reasons: Vec::new(),
            actions: Vec::new(),
            details: serde_json::Value::Null,
            notes: Vec::new(),
            restore_hint: None,
            unknown_format: false,
            age_rule: false,
            aggregate: None,
            min_size: None,
            safety_outside_project: None,
        }
    }
}

/// Receives items from [`Adapter::discover`].
pub trait ItemSink {
    fn item(&mut self, draft: ItemDraft);
    fn warning(&mut self, message: String);
}

impl ItemSink for Vec<ItemDraft> {
    fn item(&mut self, draft: ItemDraft) {
        self.push(draft);
    }

    fn warning(&mut self, message: String) {
        tracing::warn!("{message}");
    }
}

/// A rule matched against directories during the shared walk (packs'
/// `[[artifacts]]`, INTEGRATIONS.md §2.4).
#[derive(Debug, Clone)]
pub struct ArtifactRule {
    pub integration: IntegrationId,
    /// Directory name or glob (`cmake-build-*`, `*`).
    pub dir: String,
    /// Names (or globs) of files/dirs next to the directory; any must exist.
    pub requires_sibling_any: Vec<String>,
    /// All must exist.
    pub requires_sibling_all: Vec<String>,
    /// Names inside the directory; any must exist.
    pub requires_inside_any: Vec<String>,
    /// Only when git ignores the directory.
    pub requires_gitignored: bool,
    /// Directory must contain a valid `CACHEDIR.TAG`.
    pub requires_cachedir_tag: bool,
    /// Template for matched items.
    pub template: ItemDraft,
    /// Lower runs first. Generic rules (`*`) use a high value.
    pub priority: i32,
}

/// One integration (SPEC §9.2).
pub trait Adapter: Send + Sync {
    fn info(&self) -> IntegrationInfo;

    fn id(&self) -> IntegrationId {
        self.info().id
    }

    fn detect(&self, ctx: &Ctx) -> Detection;

    /// Rules for the shared walk. Most adapters have none.
    fn artifact_rules(&self) -> Vec<ArtifactRule> {
        Vec::new()
    }

    /// Absolute locations this integration owns. The walk skips them, they
    /// count as allowed roots for actions, and `doctor` checks them.
    fn locations(&self, _ctx: &Ctx) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Native tools this integration may run (for `doctor`).
    fn tools(&self) -> Vec<String> {
        Vec::new()
    }

    /// Emits items found outside the walk. No side effects.
    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError>;

    /// Adds integration-specific details to an item matched by the walk.
    fn enrich(&self, _draft: &mut ItemDraft, _path: &Path) {}

    /// Checks before acting; any failed check skips the entry.
    fn preflight(
        &self,
        _ctx: &Ctx,
        _item: &crate::model::Item,
        _action: &ActionKind,
    ) -> Vec<Check> {
        Vec::new()
    }

    /// Runs a native or compact action. Quarantine and trash are done by the
    /// core executor, never by integrations.
    fn apply(
        &self,
        _ctx: &Ctx,
        _item: &crate::model::Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        Err(IntegrationError::Unsupported(format!(
            "this integration can't run `{action}`"
        )))
    }

    /// Human description of a native action, for `explain` and plans.
    fn describe_action(&self, _action: &ActionKind) -> Option<String> {
        None
    }

    fn in_use_processes(&self) -> Vec<String> {
        self.info().in_use_processes
    }

    /// Opt-in integrations are disabled unless listed in
    /// `integrations.enable`.
    fn opt_in(&self) -> bool {
        false
    }
}
