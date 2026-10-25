//! Scanner (SPEC §5): one parallel walk over the roots for artifact rules and
//! project markers, adapters' own discovery alongside it, then sizing.

pub mod matcher;
pub mod projects;
pub mod sizes;
pub mod walker;

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use globset::{GlobSet, GlobSetBuilder};
use rayon::prelude::*;

use crate::integration::{Adapter, ArtifactRule, Ctx, ItemDraft, ItemSink, SizeSpec};
use crate::model::{CancelToken, Category, IntegrationId, Locator, ScanEvent, ScanSink};
use crate::platform::paths;

use self::matcher::{Markers, Matcher};
use self::projects::{ProjectCollector, ProjectSet};
use self::sizes::Measured;
use self::walker::{EntryKind, IgnoreWalker, Visit, Visitor, WalkEntry, WalkOptions, Walker};

/// Inputs for one scan.
pub struct ScanInput {
    pub integrations: Vec<Arc<dyn Adapter>>,
    /// Existing, canonical directories.
    pub roots: Vec<PathBuf>,
    /// Absolute-path globs to leave out.
    pub exclude: Vec<String>,
    /// Directories the walk never enters (kipple's data dir, `~/Library`, …).
    pub skip: Vec<PathBuf>,
    pub walk: WalkOptions,
}

/// An item from an integration with its measurement.
#[derive(Debug, Clone)]
pub struct RawItem {
    pub integration: IntegrationId,
    pub category: Category,
    pub draft: ItemDraft,
    pub measured: Option<Measured>,
    /// Found by the walk (inside a project) rather than by discovery.
    pub from_walk: bool,
}

pub struct ScanOutput {
    pub items: Vec<RawItem>,
    pub projects: ProjectSet,
    pub entries_seen: u64,
    pub warnings: Vec<String>,
    pub cancelled: bool,
    /// Integration-declared locations (allowed roots for actions).
    pub locations: Vec<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("invalid pattern: {0}")]
    Glob(#[from] globset::Error),
}

/// Compiles exclude globs. `a/**` also excludes `a` itself.
fn compile_excludes(globs: &[String]) -> Result<GlobSet, globset::Error> {
    let mut b = GlobSetBuilder::new();
    for g in globs {
        let g = g.trim_end_matches(['/', '\\']);
        b.add(
            globset::GlobBuilder::new(g)
                .case_insensitive(paths::CASE_INSENSITIVE)
                .build()?,
        );
        if let Some(stem) = g.strip_suffix("/**") {
            b.add(
                globset::GlobBuilder::new(stem)
                    .case_insensitive(paths::CASE_INSENSITIVE)
                    .build()?,
            );
        }
    }
    b.build()
}

struct ScanVisitor {
    matcher: Matcher,
    markers: Markers,
    skip: HashSet<PathBuf>,
    exclude: GlobSet,
    found: Mutex<Vec<(PathBuf, usize)>>,
    collector: ProjectCollector,
    entries: AtomicU64,
    unreadable: AtomicU64,
    first_errors: Mutex<Vec<String>>,
    cancel: CancelToken,
    stopped: AtomicBool,
}

impl Visitor for ScanVisitor {
    fn visit(&self, e: &WalkEntry<'_>) -> Visit {
        self.entries.fetch_add(1, Ordering::Relaxed);
        match e.kind {
            EntryKind::Dir => {
                if self.skip.contains(e.path) || self.exclude.is_match(e.path) {
                    return Visit::Skip;
                }
                if let Some(hit) = self.markers.hit(e.path) {
                    // `.git`, `*.xcodeproj` and friends: record, don't enter.
                    self.collector.add(hit.root, hit.marker);
                    return Visit::Skip;
                }
                if let Some(idx) = self.matcher.match_dir(e.path) {
                    if let Ok(mut found) = self.found.lock() {
                        found.push((e.path.to_path_buf(), idx));
                    }
                    return Visit::Skip;
                }
                Visit::Continue
            }
            EntryKind::File => {
                if let Some(hit) = self.markers.hit(e.path) {
                    self.collector.add(hit.root, hit.marker);
                }
                Visit::Continue
            }
            EntryKind::Symlink | EntryKind::Other => Visit::Skip,
        }
    }

    fn error(&self, path: Option<&Path>, message: String) {
        self.unreadable.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut errs) = self.first_errors.lock() {
            if errs.len() < 5 {
                errs.push(match path {
                    Some(p) => format!("{}: {message}", p.display()),
                    None => message,
                });
            }
        }
    }

    fn cancelled(&self) -> bool {
        if self.cancel.is_cancelled() {
            self.stopped.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

struct DraftSink<'a> {
    integration: IntegrationId,
    category: Category,
    items: &'a mut Vec<RawItem>,
    warnings: &'a mut Vec<String>,
}

impl ItemSink for DraftSink<'_> {
    fn item(&mut self, draft: ItemDraft) {
        self.items.push(RawItem {
            integration: self.integration.clone(),
            category: draft.category.unwrap_or(self.category),
            draft,
            measured: None,
            from_walk: false,
        });
    }

    fn warning(&mut self, message: String) {
        self.warnings
            .push(format!("{}: {message}", self.integration));
    }
}

/// Runs discovery for every integration in parallel.
fn discover_all(ctx: &Ctx, integrations: &[Arc<dyn Adapter>]) -> (Vec<RawItem>, Vec<String>) {
    let results: Vec<(Vec<RawItem>, Vec<String>)> = integrations
        .par_iter()
        .map(|adapter| {
            let info = adapter.info();
            let mut items = Vec::new();
            let mut warnings = Vec::new();
            let mut sink = DraftSink {
                integration: info.id.clone(),
                category: info.category,
                items: &mut items,
                warnings: &mut warnings,
            };
            if let Err(e) = adapter.discover(ctx, &mut sink) {
                warnings.push(format!("{}: {e}", info.id));
            }
            (items, warnings)
        })
        .collect();
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    for (i, w) in results {
        items.extend(i);
        warnings.extend(w);
    }
    (items, warnings)
}

/// Builds a draft for a walk match from its rule's template.
fn draft_for_match(rule: &ArtifactRule, path: &Path) -> ItemDraft {
    let mut draft = rule.template.clone();
    draft.locator = Locator::path(path);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    draft.title = draft.title.replace("{name}", &name);
    draft
}

/// Merges drafts with `aggregate` set into one item per (integration,
/// project or parent folder).
fn aggregate(items: Vec<RawItem>, projects: &ProjectSet) -> Vec<RawItem> {
    let mut out = Vec::with_capacity(items.len());
    let mut groups: BTreeMap<(String, PathBuf), Vec<RawItem>> = BTreeMap::new();
    for item in items {
        let path = item.draft.locator.primary_path().cloned();
        match (&item.draft.aggregate, path) {
            (Some(_), Some(path)) => {
                let key_root = projects
                    .enclosing(&path)
                    .map(|p| p.root.clone())
                    .or_else(|| path.parent().map(Path::to_path_buf))
                    .unwrap_or(path);
                groups
                    .entry((item.integration.to_string(), key_root))
                    .or_default()
                    .push(item);
            }
            _ => out.push(item),
        }
    }
    for ((_, root), mut group) in groups {
        group.sort_by(|a, b| {
            a.draft
                .locator
                .canonical_key()
                .cmp(&b.draft.locator.canonical_key())
        });
        let mut first = group.remove(0);
        if group.is_empty() {
            out.push(first);
            continue;
        }
        let mut paths_all: Vec<PathBuf> =
            first.draft.locator.paths().into_iter().cloned().collect();
        for g in &group {
            paths_all.extend(g.draft.locator.paths().into_iter().cloned());
        }
        paths_all.sort();
        let stem = first.draft.aggregate.clone().unwrap_or_default();
        first.draft.title = format!("{stem} ({} folders)", paths_all.len());
        first.draft.locator = Locator::Paths { paths: paths_all };
        first.draft.project_hint = Some(root);
        out.push(first);
    }
    out
}

/// Runs the scan. Progress goes to `sink`; cancellation via `ctx.cancel`.
pub fn run(ctx: &Ctx, input: ScanInput, sink: &dyn ScanSink) -> Result<ScanOutput, ScanError> {
    let started = Instant::now();
    let cancel = ctx.cancel.clone();
    sink.event(ScanEvent::Started {
        roots: input.roots.clone(),
    });

    // Locations first: the walk must not enter them.
    let locations: Vec<PathBuf> = input
        .integrations
        .par_iter()
        .flat_map_iter(|a| a.locations(ctx))
        .collect();
    let mut skip: HashSet<PathBuf> = HashSet::new();
    for p in input.skip.iter().chain(locations.iter()) {
        skip.insert(paths::lexical_normalize(p));
        if let Ok(c) = p.canonicalize() {
            skip.insert(paths::strip_verbatim(c));
        }
    }

    let rules: Vec<ArtifactRule> = input
        .integrations
        .iter()
        .flat_map(|a| a.artifact_rules())
        .collect();
    let by_id: BTreeMap<IntegrationId, Arc<dyn Adapter>> = input
        .integrations
        .iter()
        .map(|a| (a.id(), Arc::clone(a)))
        .collect();

    let visitor = Arc::new(ScanVisitor {
        matcher: Matcher::new(rules)?,
        markers: Markers::default(),
        skip,
        exclude: compile_excludes(&input.exclude)?,
        found: Mutex::new(Vec::new()),
        collector: ProjectCollector::default(),
        entries: AtomicU64::new(0),
        unreadable: AtomicU64::new(0),
        first_errors: Mutex::new(Vec::new()),
        cancel: cancel.clone(),
        stopped: AtomicBool::new(false),
    });

    let walker = IgnoreWalker;
    let walk_done = AtomicBool::new(false);
    let (mut discovered, mut warnings) = std::thread::scope(|s| {
        // Progress ticker.
        s.spawn(|| {
            while !walk_done.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
                if let Some(t) = sink.cancel_token() {
                    if t.is_cancelled() {
                        cancel.cancel();
                    }
                }
                sink.event(ScanEvent::Progress {
                    done: visitor.entries.load(Ordering::Relaxed),
                    total: None,
                    bytes: 0,
                });
            }
        });
        let discovery = s.spawn(|| discover_all(ctx, &input.integrations));
        let v: Arc<dyn Visitor> = visitor.clone();
        walker.walk(&input.roots, &input.walk, v);
        let result = discovery.join().unwrap_or_default();
        walk_done.store(true, Ordering::Relaxed);
        result
    });

    let found = visitor
        .found
        .lock()
        .map(|mut f| std::mem::take(&mut *f))
        .unwrap_or_default();
    let entries_seen = visitor.entries.load(Ordering::Relaxed);
    let unreadable = visitor.unreadable.load(Ordering::Relaxed);
    if unreadable > 0 {
        let examples = visitor
            .first_errors
            .lock()
            .map(|e| e.join("; "))
            .unwrap_or_default();
        warnings.push(format!(
            "Couldn't read {unreadable} entries (for example: {examples})."
        ));
    }
    let Ok(visitor) = Arc::try_unwrap(visitor) else {
        unreachable!("walker threads have finished");
    };
    let projects = visitor.collector.finish();

    let mut items: Vec<RawItem> = found
        .into_iter()
        .map(|(path, idx)| {
            let rule = visitor.matcher.rule(idx);
            let mut draft = draft_for_match(rule, &path);
            let category = draft.category;
            if let Some(adapter) = by_id.get(&rule.integration) {
                adapter.enrich(&mut draft, &path);
            }
            let category = category
                .or(draft.category)
                .or_else(|| by_id.get(&rule.integration).map(|a| a.info().category))
                .unwrap_or(Category::ProjectArtifact);
            RawItem {
                integration: rule.integration.clone(),
                category,
                draft,
                measured: None,
                from_walk: true,
            }
        })
        .collect();
    items.append(&mut discovered);
    let mut items = aggregate(items, &projects);

    // Size everything that needs measuring.
    items.par_iter_mut().for_each(|item| {
        if item.draft.size == SizeSpec::Measure && !cancel.is_cancelled() {
            let paths: Vec<PathBuf> = item.draft.locator.paths().into_iter().cloned().collect();
            if !paths.is_empty() {
                item.measured = Some(sizes::measure(&paths, &cancel));
            }
        }
    });

    tracing::info!(
        entries = entries_seen,
        items = items.len(),
        walker = walker.name(),
        ms = started.elapsed().as_millis() as u64,
        "walk finished"
    );
    Ok(ScanOutput {
        items,
        projects,
        entries_seen,
        warnings,
        cancelled: cancel.is_cancelled(),
        locations,
    })
}
