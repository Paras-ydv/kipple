//! The public API every interface uses (SPEC §3.4).

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::attribute;
use crate::config::Config;
use crate::dirs::Dirs;
use crate::index::{Index, KEEP_SCANS};
use crate::integration::{Adapter, Ctx, SizeSpec};
use crate::model::*;
use crate::platform::{Env, Platform, ProcessSource, paths};
use crate::safety::engine::{self, ItemFacts, SafetyContext};
use crate::safety::{ProtectedSet, reasons};
use crate::scan::projects::ProjectSet;
use crate::scan::walker::WalkOptions;
use crate::scan::{self, RawItem, ScanInput};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[error(transparent)]
    Index(#[from] crate::index::IndexError),
    #[error(transparent)]
    NoHome(#[from] crate::dirs::NoHomeDir),
    #[error(transparent)]
    Scan(#[from] crate::scan::ScanError),
    #[error("{0}")]
    Path(#[from] crate::platform::paths::ExpandError),
    #[error("no scan yet; run `kipple scan` first")]
    NoScan,
    #[error("no item matches `{0}`")]
    ItemNotFound(String),
    #[error("`{0}` matches more than one item; use more characters")]
    AmbiguousItem(String),
    #[error("unknown integration or category `{0}`")]
    UnknownSelector(String),
    #[error("{0}")]
    Invalid(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("data error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// How to open kipple.
pub struct OpenOptions {
    pub env: Env,
    /// Packs and adapters. The core knows none on its own.
    pub integrations: Vec<Arc<dyn Adapter>>,
    /// Overrides the config file location.
    pub config_file: Option<PathBuf>,
    /// Replaces the process list (tests).
    pub processes: Option<Arc<dyn ProcessSource>>,
}

impl OpenOptions {
    pub fn new(env: Env, integrations: Vec<Arc<dyn Adapter>>) -> Self {
        Self {
            env,
            integrations,
            config_file: None,
            processes: None,
        }
    }
}

pub struct Kipple {
    platform: Platform,
    dirs: Dirs,
    config: Config,
    integrations: Vec<Arc<dyn Adapter>>,
    index: Mutex<Index>,
}

/// A summary of `items` per category. `Keep` items only show up in
/// `by_safety.keep`.
pub fn summarize(items: &[Item]) -> Summary {
    let mut by_cat: BTreeMap<Category, CategorySummary> = BTreeMap::new();
    let mut by_safety = SafetyMix::default();
    let mut reclaimable = 0;
    for item in items {
        let bytes = item.size.reclaimable;
        by_safety.add(item.safety, bytes);
        // Keep items are reported but never count as reclaimable.
        if item.safety == Safety::Keep {
            continue;
        }
        reclaimable += bytes;
        let row = by_cat.entry(item.category).or_insert(CategorySummary {
            category: item.category,
            items: 0,
            reclaimable: 0,
            by_safety: SafetyMix::default(),
        });
        row.items += 1;
        row.reclaimable += bytes;
        row.by_safety.add(item.safety, bytes);
    }
    let mut categories: Vec<CategorySummary> = by_cat.into_values().collect();
    categories.sort_by(|a, b| {
        b.reclaimable
            .cmp(&a.reclaimable)
            .then(a.category.cmp(&b.category))
    });
    Summary {
        items: items.len() as u64,
        reclaimable,
        by_safety,
        categories,
        quarantine_bytes: 0,
    }
}

impl Kipple {
    /// Loads config, opens the index and registers integrations.
    pub fn open(opts: OpenOptions) -> Result<Self> {
        let mut platform = Platform::new(opts.env);
        if let Some(p) = opts.processes {
            platform = platform.with_processes(p);
        }
        let mut dirs = Dirs::resolve(&platform.env, platform.os)?;
        if let Some(c) = opts.config_file {
            dirs.config_file = c;
        }
        let config = Config::load(&dirs.config_file)?;
        let index = Index::open(&dirs.index_db())?;
        Ok(Self {
            platform,
            dirs,
            config,
            integrations: opts.integrations,
            index: Mutex::new(index),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn dirs(&self) -> &Dirs {
        &self.dirs
    }

    pub fn platform(&self) -> &Platform {
        &self.platform
    }

    fn index(&self) -> MutexGuard<'_, Index> {
        self.index.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn adapter(&self, id: &IntegrationId) -> Option<&Arc<dyn Adapter>> {
        self.integrations.iter().find(|a| &a.id() == id)
    }

    fn opted_in(&self) -> HashSet<IntegrationId> {
        self.config
            .integrations
            .enable
            .iter()
            .map(|s| IntegrationId::from(s.as_str()))
            .collect()
    }

    fn is_enabled(&self, a: &dyn Adapter) -> bool {
        let id = a.id();
        let info = a.info();
        let disabled = self
            .config
            .integrations
            .disable
            .iter()
            .any(|d| d == id.as_str());
        let opt_ok = !a.opt_in()
            || self
                .config
                .integrations
                .enable
                .iter()
                .any(|e| e == id.as_str());
        let os_ok = crate::platform::os::os_matches(&info.os, self.platform.os, self.platform.wsl);
        !disabled && opt_ok && os_ok
    }

    /// Validates `--only` selectors.
    fn check_selectors(&self, only: &[String]) -> Result<()> {
        for s in only {
            let known = s.parse::<Category>().is_ok()
                || self.integrations.iter().any(|a| a.id().as_str() == s);
            if !known {
                return Err(Error::UnknownSelector(s.clone()));
            }
        }
        Ok(())
    }

    fn selected(a: &dyn Adapter, only: &[String]) -> bool {
        if only.is_empty() {
            return true;
        }
        let info = a.info();
        only.iter().any(|s| {
            s == info.id.as_str() || s.parse::<Category>().is_ok_and(|c| c == info.category)
        })
    }

    fn ctx<'a>(
        &'a self,
        cancel: &'a CancelToken,
        procs: &'a crate::platform::ProcessSnapshot,
    ) -> Ctx<'a> {
        Ctx::new(&self.platform, &self.config, &self.dirs, cancel, procs)
    }

    /// Every registered integration and whether it is present.
    pub fn integrations(&self) -> Vec<IntegrationStatusReport> {
        let cancel = CancelToken::new();
        let procs = crate::platform::ProcessSnapshot::default();
        let ctx = self.ctx(&cancel, &procs);
        self.integrations
            .iter()
            .map(|a| IntegrationStatusReport {
                info: a.info(),
                enabled: self.is_enabled(a.as_ref()),
                detection: a.detect(&ctx),
            })
            .collect()
    }

    pub fn adapters(&self) -> &[Arc<dyn Adapter>] {
        &self.integrations
    }

    fn expand(&self, s: &str) -> Result<PathBuf> {
        Ok(paths::expand(s, &self.platform.env, &BTreeMap::new())?)
    }

    fn resolve_roots(
        &self,
        requested: &[PathBuf],
        warnings: &mut Vec<String>,
    ) -> Result<Vec<PathBuf>> {
        let raw: Vec<PathBuf> = if requested.is_empty() {
            self.config
                .scan
                .roots
                .iter()
                .map(|r| self.expand(r))
                .collect::<Result<_>>()?
        } else {
            requested.to_vec()
        };
        let mut roots = Vec::new();
        for r in raw {
            match r.canonicalize() {
                Ok(c) if c.is_dir() => roots.push(paths::strip_verbatim(c)),
                _ => warnings.push(format!(
                    "{} doesn't exist or isn't a folder; skipped.",
                    r.display()
                )),
            }
        }
        roots.sort();
        roots.dedup();
        Ok(roots)
    }

    fn skip_dirs(&self) -> Vec<PathBuf> {
        let mut skip = vec![self.dirs.data_dir.clone()];
        if let Some(home) = self.platform.home() {
            skip.push(home.join(".Trash"));
            if self.platform.os == Os::Macos {
                // Avoids TCC prompts; integrations reach their paths directly.
                skip.push(home.join("Library"));
            }
        }
        for sys in [
            "/proc",
            "/sys",
            "/dev",
            "/System",
            "/usr",
            "/private/var/vm",
            "/Volumes",
        ] {
            skip.push(PathBuf::from(sys));
        }
        if !self.config.scan.include_network {
            skip.extend(crate::platform::mounts::network_mounts());
        }
        skip
    }

    /// Scans, judges and stores the result (SPEC §5–7).
    pub fn scan(&self, req: ScanRequest, sink: &dyn ScanSink) -> Result<ScanResult> {
        self.check_selectors(&req.only)?;
        let started_at = Timestamp::now();
        let clock = std::time::Instant::now();
        let mut warnings = Vec::new();
        let roots = self.resolve_roots(&req.roots, &mut warnings)?;
        let integrations: Vec<Arc<dyn Adapter>> = self
            .integrations
            .iter()
            .filter(|a| self.is_enabled(a.as_ref()) && Self::selected(a.as_ref(), &req.only))
            .cloned()
            .collect();
        let mut exclude: Vec<String> = Vec::new();
        for g in self.config.scan.exclude.iter().chain(req.exclude.iter()) {
            exclude.push(self.expand(g)?.to_string_lossy().into_owned());
        }

        let cancel = sink.cancel_token().cloned().unwrap_or_default();
        let procs = self.platform.processes.snapshot();
        let ctx = self.ctx(&cancel, &procs);
        let out = scan::run(
            &ctx,
            ScanInput {
                integrations: integrations.clone(),
                roots: roots.clone(),
                exclude,
                skip: self.skip_dirs(),
                walk: WalkOptions {
                    follow_symlinks: req
                        .follow_symlinks
                        .unwrap_or(self.config.scan.follow_symlinks),
                    ..WalkOptions::default()
                },
            },
            sink,
        )?;
        warnings.extend(out.warnings.iter().cloned());

        let mut allowed = roots.clone();
        allowed.extend(out.locations.iter().cloned());
        let protect: Vec<PathBuf> = self
            .config
            .safety
            .protect
            .iter()
            .map(|p| self.expand(p))
            .collect::<Result<_>>()?;
        let protected = ProtectedSet::new(
            &self.platform.env,
            self.platform.os,
            &self.dirs,
            &protect,
            &roots,
        );
        let opted = self.opted_in();
        let safety_ctx = SafetyContext {
            protected: &protected,
            allowed_roots: &allowed,
            processes: &procs,
            config: &self.config.safety,
            now: Timestamp::now(),
            opted_in: &opted,
        };

        let mut projects = out.projects;
        let infos: BTreeMap<IntegrationId, IntegrationInfo> =
            integrations.iter().map(|a| (a.id(), a.info())).collect();
        let (items, project_summaries) =
            self.finalize(out.items, &mut projects, &infos, &safety_ctx);

        for item in &items {
            sink.event(ScanEvent::ItemFound {
                item: Box::new(item.clone()),
            });
        }

        let cancelled = out.cancelled;
        let mut summary = summarize(&items);
        summary.quarantine_bytes = self.quarantine_bytes();
        let result = ScanResult {
            scan: ScanInfo {
                id: ScanId::generate(),
                started_at,
                finished_at: Some(Timestamp::now()),
                roots,
                kipple_version: crate::VERSION.to_owned(),
                status: if cancelled {
                    ScanStatus::Cancelled
                } else {
                    ScanStatus::Complete
                },
                entries_seen: out.entries_seen,
                duration_ms: clock.elapsed().as_millis() as u64,
            },
            summary,
            warnings,
        };
        {
            let mut index = self.index();
            index.put_scan(&result)?;
            index.put_scan_contents(&result.scan.id, &items, &project_summaries)?;
            if !cancelled {
                index.put_size_points(&size_points(&result, &items))?;
            }
            index.prune(KEEP_SCANS)?;
        }
        sink.event(ScanEvent::Finished {
            result: Box::new(result.clone()),
        });
        Ok(result)
    }

    /// Turns raw items into judged items and project summaries.
    fn finalize(
        &self,
        raw: Vec<RawItem>,
        projects: &mut ProjectSet,
        infos: &BTreeMap<IntegrationId, IntegrationInfo>,
        safety: &SafetyContext,
    ) -> (Vec<Item>, Vec<ProjectSummary>) {
        // Make sure metadata-referenced projects exist in the set.
        for r in &raw {
            if let Some(hint) = &r.draft.project_hint {
                projects.ensure(hint);
            }
        }
        let mut last_used_cache: BTreeMap<PathBuf, Option<LastUsed>> = BTreeMap::new();
        let mut items = Vec::with_capacity(raw.len());
        for r in raw {
            let Some(info) = infos.get(&r.integration) else {
                continue;
            };
            let d = r.draft;
            let primary = d.locator.primary_path().cloned();
            let attribution = match (&d.project_hint, &primary) {
                (Some(hint), _) => projects.attribute(&paths::lexical_normalize(hint), false),
                (None, Some(p)) if r.from_walk => projects.attribute(p, true),
                _ => None,
            };
            let project_last_used = attribution.as_ref().and_then(|a| {
                last_used_cache
                    .entry(a.project.root.clone())
                    .or_insert_with(|| {
                        let has_git = projects.get(&a.project.root).is_some_and(|p| p.has_git());
                        attribute::project_last_used(&a.project.root, has_git)
                    })
                    .to_owned()
            });

            let (size, shared, newest) = match (&d.size, &r.measured) {
                (SizeSpec::Known(s), _) => (*s, 0, None),
                (SizeSpec::Measure, Some(m)) => {
                    let (s, shared) = m.finish();
                    (s, shared, m.newest_mtime)
                }
                (SizeSpec::Measure, None) => (Size::default(), 0, None),
            };
            let mut notes = d.notes.clone();
            if shared > 0 {
                notes.push(format!(
                    "{} of this is hard-linked with files elsewhere and won't be freed.",
                    crate::format::size(shared, crate::config::Units::Si)
                ));
            }
            let last_used = d.last_used.or_else(|| {
                newest
                    .and_then(|t| Timestamp::try_from(t).ok())
                    .map(|at| LastUsed {
                        at,
                        source: LastUsedSource::ItemMtime,
                    })
            });
            let mut details = d.details.clone();
            if let Some(sub) = attribution.as_ref().and_then(|a| a.subproject.as_ref()) {
                if !details.is_object() {
                    details = serde_json::json!({});
                }
                details["subproject"] = serde_json::json!(sub);
            }
            let identity = primary
                .as_ref()
                .and_then(|p| crate::platform::fsx::identity(p).ok());
            let mut reasons = d.reasons.clone();
            if reasons.is_empty() {
                reasons.push(reasons::reason(ReasonCode::UnknownFormat));
            }
            if d.min_size.is_some_and(|m| size.reclaimable < m) {
                continue;
            }
            let mut safety_level = d.safety;
            let mut reasons_extra = Vec::new();
            if attribution.is_none()
                && let Some(outside) = d.safety_outside_project
                && outside > safety_level
            {
                safety_level = outside;
                reasons_extra.push(reasons::reason_with(ReasonCode::ContainsUserData, &[]));
            }
            reasons.extend(reasons_extra);
            let mut item = Item {
                id: ItemId::new(&r.integration, &d.locator),
                integration: r.integration.clone(),
                category: r.category,
                kind: d.kind.clone(),
                title: d.title.clone(),
                locator: d.locator.clone(),
                size,
                project: attribution.as_ref().map(|a| a.project.clone()),
                last_used,
                safety: safety_level,
                reasons,
                actions: d.actions.clone(),
                details,
                notes,
                restore_hint: d.restore_hint.clone(),
                identity,
            };
            let facts = ItemFacts {
                info,
                unknown_format: d.unknown_format,
                age_rule: d.age_rule,
                from_walk: r.from_walk,
                project_last_used,
                newest_mtime: newest,
            };
            engine::evaluate(&mut item, &facts, safety);
            if let Some(p) = &item.project
                && !p.root.exists()
                && !item.has_reason(ReasonCode::OrphanedProject)
            {
                item.reasons
                    .push(reasons::reason(ReasonCode::OrphanedProject));
            }
            items.push(item);
        }
        items.sort_by(|a, b| {
            b.size
                .reclaimable
                .cmp(&a.size.reclaimable)
                .then(a.id.cmp(&b.id))
        });
        items.dedup_by(|a, b| a.id == b.id);

        // Project summaries for projects with items.
        let mut summaries: BTreeMap<ProjectId, ProjectSummary> = BTreeMap::new();
        for item in &items {
            let Some(p) = &item.project else { continue };
            let s = summaries.entry(p.id.clone()).or_insert_with(|| {
                let markers = projects
                    .get(&p.root)
                    .map(|r| r.markers.iter().cloned().collect())
                    .unwrap_or_default();
                ProjectSummary {
                    id: p.id.clone(),
                    root: p.root.clone(),
                    name: p.name.clone(),
                    markers,
                    last_used: last_used_cache.get(&p.root).cloned().flatten(),
                    total: Size::default(),
                    by_category: BTreeMap::new(),
                    safety_mix: SafetyMix::default(),
                    item_count: 0,
                    exists: p.root.exists(),
                }
            });
            s.safety_mix.add(item.safety, item.size.reclaimable);
            if item.safety == Safety::Keep {
                s.item_count += 1;
                continue;
            }
            s.total.add(&item.size);
            *s.by_category.entry(item.category).or_default() += item.size.reclaimable;
            s.item_count += 1;
        }
        (items, summaries.into_values().collect())
    }

    /// Bytes currently held in quarantine.
    pub fn quarantine_bytes(&self) -> u64 {
        crate::quarantine::Quarantine::new(&self.dirs, self.config.quarantine.keep_days)
            .total_bytes()
    }

    fn latest(&self) -> Result<ScanResult> {
        self.index().latest_scan()?.ok_or(Error::NoScan)
    }

    pub fn latest_scan(&self) -> Result<Option<ScanResult>> {
        Ok(self.index().latest_scan()?)
    }

    /// All items of the latest scan.
    pub fn all_items(&self) -> Result<(ScanResult, Vec<Item>)> {
        let scan = self.latest()?;
        let items = self.index().items(&scan.scan.id)?;
        Ok((scan, items))
    }

    /// Items from the latest scan, filtered and sorted.
    pub fn items(&self, query: ItemQuery) -> Result<Vec<Item>> {
        self.check_selectors(&query.filter.only)?;
        let (_, items) = self.all_items()?;
        let mut out = filter_items(items, &query.filter, query.include_keep, &self.platform.env)?;
        match query.sort {
            ItemSort::Size => out.sort_by_key(|a| std::cmp::Reverse(a.size.reclaimable)),
            ItemSort::Age => out.sort_by_key(|i| i.last_used.map(|l| l.at)),
            ItemSort::Name => out.sort_by_key(|a| a.title.to_lowercase()),
        }
        if let Some(limit) = query.limit {
            out.truncate(limit);
        }
        Ok(out)
    }

    /// Projects of the latest scan.
    pub fn projects(&self, query: ProjectQuery) -> Result<Vec<ProjectSummary>> {
        let scan = self.latest()?;
        let mut out: Vec<ProjectSummary> = self
            .index()
            .projects(&scan.scan.id)?
            .into_iter()
            .filter(|p| !query.orphaned || !p.exists)
            .filter(|p| query.min_size.is_none_or(|m| p.total.reclaimable >= m))
            .collect();
        match query.sort {
            ProjectSort::Size => out.sort_by_key(|a| std::cmp::Reverse(a.total.reclaimable)),
            ProjectSort::Age => out.sort_by_key(|p| p.last_used.map(|l| l.at)),
            ProjectSort::Name => out.sort_by_key(|a| a.name.to_lowercase()),
        }
        if let Some(limit) = query.limit {
            out.truncate(limit);
        }
        Ok(out)
    }

    /// The latest scan's report: summary, heaviest projects, next step.
    pub fn report(&self) -> Result<Report> {
        let (scan, items) = self.all_items()?;
        let top_projects = self.projects(ProjectQuery {
            limit: Some(5),
            ..Default::default()
        })?;
        let mut summary = summarize(&items);
        summary.quarantine_bytes = self.quarantine_bytes();
        let safe: Vec<&Item> = items.iter().filter(|i| i.safety == Safety::Safe).collect();
        let next = (!safe.is_empty()).then(|| NextStep {
            command: "kipple clean --safe-only".to_owned(),
            reclaimable: safe.iter().map(|i| i.size.reclaimable).sum(),
            undoable: safe
                .iter()
                .filter(|i| i.actions.first().is_some_and(ActionKind::undoable))
                .map(|i| i.size.reclaimable)
                .sum(),
            keep_days: self.config.quarantine.keep_days,
        });
        Ok(Report {
            scan: scan.scan,
            summary,
            top_projects,
            next,
        })
    }

    /// Finds an item of the latest scan by id, unique id prefix or path.
    pub fn find_item(&self, key: &str) -> Result<Item> {
        let (_, items) = self.all_items()?;
        find_in(&items, key, &self.platform.env)
    }

    /// Everything about one item (`kipple explain`).
    pub fn explain(&self, key: &str) -> Result<Explanation> {
        let item = self.find_item(key)?;
        let adapter = self.adapter(&item.integration);
        let info = adapter
            .map(|a| a.info())
            .unwrap_or_else(|| IntegrationInfo {
                id: item.integration.clone(),
                name: item.integration.to_string(),
                kind: IntegrationKind::Pack,
                category: item.category,
                description: String::new(),
                docs_url: None,
                status: IntegrationStatus::Stable,
                os: vec![],
                in_use_processes: vec![],
            });
        let actions = item
            .actions
            .iter()
            .map(|a| ActionInfo {
                action: a.clone(),
                description: adapter
                    .and_then(|ad| ad.describe_action(a))
                    .unwrap_or_else(|| default_action_text(a, self.config.quarantine.keep_days)),
                undoable: a.undoable(),
            })
            .collect();
        let s = item.size;
        let size_differs = s.apparent.abs_diff(s.reclaimable) * 10 > s.apparent.max(1);
        Ok(Explanation {
            item,
            integration: info,
            actions,
            size_differs,
        })
    }

    /// Growth timeline (SPEC §3.4).
    pub fn history(&self, req: HistoryRequest) -> Result<Vec<SizePoint>> {
        Ok(self.index().size_points(&req)?)
    }
}

/// Plain description of a core action.
pub fn default_action_text(action: &ActionKind, keep_days: u32) -> String {
    match action {
        ActionKind::Quarantine => format!(
            "Move to kipple's quarantine. You can undo this for {keep_days} days, then it is deleted."
        ),
        ActionKind::Trash => "Move to the trash. Restore it from there if needed.".to_owned(),
        ActionKind::Native { command_id } => format!("Run the tool's own cleanup ({command_id})."),
        ActionKind::Compact { method } => format!("Compact the database ({method})."),
        ActionKind::Archive => "Compress into an archive, then quarantine the original.".to_owned(),
        ActionKind::Relocate { target } => {
            format!("Move to {} and link it back.", target.display())
        }
        ActionKind::SetRetention { key, .. } => format!("Change the tool's `{key}` setting."),
    }
}

/// Finds an item by id, unique id prefix (≥ 4 chars) or exact path.
pub fn find_in(items: &[Item], key: &str, env: &Env) -> Result<Item> {
    if let Some(i) = items.iter().find(|i| i.id.as_str() == key) {
        return Ok(i.clone());
    }
    if key.len() >= 4 && key.chars().all(|c| c.is_ascii_hexdigit()) {
        let hits: Vec<&Item> = items
            .iter()
            .filter(|i| i.id.as_str().starts_with(key))
            .collect();
        match hits.len() {
            1 => return Ok(hits[0].clone()),
            n if n > 1 => return Err(Error::AmbiguousItem(key.to_owned())),
            _ => {}
        }
    }
    let path = paths::expand(key, env, &BTreeMap::new())
        .ok()
        .map(|p| p.canonicalize().map(paths::strip_verbatim).unwrap_or(p));
    if let Some(path) = path {
        let norm = paths::lexical_normalize(&path);
        if let Some(i) = items.iter().find(|i| {
            i.locator
                .paths()
                .iter()
                .any(|p| paths::lexical_normalize(p) == norm)
        }) {
            return Ok(i.clone());
        }
    }
    Err(Error::ItemNotFound(key.to_owned()))
}

/// Applies an [`ItemFilter`]. `Keep` items are dropped unless `include_keep`.
pub fn filter_items(
    items: Vec<Item>,
    f: &ItemFilter,
    include_keep: bool,
    env: &Env,
) -> Result<Vec<Item>> {
    let mut exclude = globset::GlobSetBuilder::new();
    for g in &f.exclude {
        let expanded = paths::expand(g, env, &BTreeMap::new())?;
        exclude.add(
            globset::GlobBuilder::new(&expanded.to_string_lossy())
                .case_insensitive(paths::CASE_INSENSITIVE)
                .build()
                .map_err(|e| Error::Invalid(format!("bad glob `{g}`: {e}")))?,
        );
    }
    let exclude = exclude.build().map_err(|e| Error::Invalid(e.to_string()))?;
    let roots: Vec<PathBuf> = f
        .roots
        .iter()
        .map(|r| {
            r.canonicalize()
                .map(paths::strip_verbatim)
                .unwrap_or_else(|_| r.clone())
        })
        .collect();
    let now = Timestamp::now().as_second();
    let ids: Option<Vec<Item>> = if f.ids.is_empty() {
        None
    } else {
        Some(
            f.ids
                .iter()
                .map(|k| find_in(&items, k, env))
                .collect::<Result<_>>()?,
        )
    };
    let pool = ids.unwrap_or(items);
    Ok(pool
        .into_iter()
        .filter(|i| include_keep || i.safety != Safety::Keep)
        .filter(|i| {
            f.only.is_empty()
                || f.only.iter().any(|s| {
                    s == i.integration.as_str()
                        || s.parse::<Category>().is_ok_and(|c| c == i.category)
                })
        })
        .filter(|i| !i.locator.paths().iter().any(|p| exclude.is_match(p)))
        .filter(|i| {
            roots.is_empty()
                || i.locator
                    .paths()
                    .iter()
                    .any(|p| roots.iter().any(|r| paths::is_within(p, r)))
        })
        .filter(|i| {
            f.older_than_secs.is_none_or(|secs| {
                i.last_used
                    .is_some_and(|lu| now - lu.at.as_second() >= secs as i64)
            })
        })
        .filter(|i| f.min_size.is_none_or(|m| i.size.reclaimable >= m))
        .filter(|i| !f.safe_only || i.safety == Safety::Safe)
        .filter(|i| f.safety.is_empty() || f.safety.contains(&i.safety))
        .filter(|i| {
            f.project
                .as_ref()
                .is_none_or(|p| i.project.as_ref().is_some_and(|ip| &ip.id == p))
        })
        .collect())
}

fn size_points(result: &ScanResult, items: &[Item]) -> Vec<SizePoint> {
    let ts = result.scan.started_at;
    let mut points = vec![SizePoint {
        ts,
        scope: SizeScope::Total,
        key: "total".into(),
        reclaimable: result.summary.reclaimable,
    }];
    for c in &result.summary.categories {
        points.push(SizePoint {
            ts,
            scope: SizeScope::Category,
            key: c.category.id().into(),
            reclaimable: c.reclaimable,
        });
    }
    let mut by_int: BTreeMap<&str, u64> = BTreeMap::new();
    let mut by_proj: BTreeMap<&str, u64> = BTreeMap::new();
    for i in items {
        *by_int.entry(i.integration.as_str()).or_default() += i.size.reclaimable;
        if let Some(p) = &i.project {
            *by_proj.entry(p.id.as_str()).or_default() += i.size.reclaimable;
        }
    }
    for (k, v) in by_int {
        points.push(SizePoint {
            ts,
            scope: SizeScope::Integration,
            key: k.into(),
            reclaimable: v,
        });
    }
    for (k, v) in by_proj {
        points.push(SizePoint {
            ts,
            scope: SizeScope::Project,
            key: k.into(),
            reclaimable: v,
        });
    }
    points
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Kipple>();
};
