//! Planner and executor (SPEC §8). The planner is pure; the executor is the
//! only code path that changes anything, and it re-validates every entry
//! immediately before acting.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::integration::{Adapter, Ctx};
use crate::journal::Journal;
use crate::kipple::{Error, Kipple, Result, filter_items};
use crate::model::*;
use crate::platform::{TrashBin, fsx, paths};
use crate::quarantine::Quarantine;
use crate::safety::{ProtectedSet, is_actionable, reasons};

/// Plans stay executable this long after they're made.
pub const PLAN_TTL_SECS: i64 = 3600;

/// Builds a plan from items (pure).
pub fn build_plan(
    scan_id: ScanId,
    items: Vec<Item>,
    overrides: &BTreeMap<ItemId, ActionKind>,
    quarantine: &Quarantine,
    trash: &TrashBin,
) -> Plan {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for item in items {
        if item.safety == Safety::Keep || item.actions.is_empty() {
            continue;
        }
        let mut action = overrides
            .get(&item.id)
            .cloned()
            .unwrap_or_else(|| item.actions[0].clone());
        if !item.actions.contains(&action) && !matches!(action, ActionKind::Trash) {
            warnings.push(format!(
                "{}: `{action}` isn't allowed for this item; skipped.",
                item.title
            ));
            continue;
        }
        if action == ActionKind::Quarantine
            && !item
                .locator
                .paths()
                .iter()
                .all(|p| quarantine.same_volume(p))
        {
            if trash.available() {
                action = ActionKind::Trash;
            } else {
                warnings.push(format!(
                    "{} is on another drive and there's no trash here, so it was left out.",
                    item.title
                ));
                continue;
            }
        }
        entries.push(PlanEntry {
            item,
            action,
            preflight: Vec::new(),
        });
    }
    entries.sort_by(|a, b| {
        a.item
            .safety
            .cmp(&b.item.safety)
            .then(b.item.size.reclaimable.cmp(&a.item.size.reclaimable))
            .then(a.item.id.cmp(&b.item.id))
    });
    let totals = totals(&entries);
    Plan {
        id: PlanId::generate(),
        created_at: Timestamp::now(),
        scan_id,
        entries,
        totals,
        warnings,
    }
}

pub fn totals(entries: &[PlanEntry]) -> PlanTotals {
    let mut t = PlanTotals::default();
    for e in entries {
        let bytes = e.item.size.reclaimable;
        t.entries += 1;
        t.reclaimable += bytes;
        t.by_safety.add(e.item.safety, bytes);
        *t.by_action.entry(e.action.name().to_owned()).or_default() += bytes;
        if e.action.frees_immediately() {
            t.freed_now += bytes;
        } else {
            t.held_in_quarantine += bytes;
        }
    }
    t
}

/// Shared state for one execution.
struct Run<'a> {
    k: &'a Kipple,
    plan: &'a Plan,
    protected: ProtectedSet,
    allowed: Vec<PathBuf>,
    quarantine: Quarantine,
    journal: Journal,
    journal_lock: Mutex<()>,
    sink: &'a dyn ExecSink,
    cancel: CancelToken,
}

impl Run<'_> {
    fn adapter(&self, id: &IntegrationId) -> Option<&Arc<dyn Adapter>> {
        self.k.adapters().iter().find(|a| &a.id() == id)
    }

    /// SPEC §8.3 steps 1–3. `Err` is the reason to skip.
    fn revalidate(&self, entry: &PlanEntry) -> std::result::Result<(), String> {
        let item = &entry.item;
        for (i, p) in item.locator.paths().iter().enumerate() {
            let Ok(meta) = p.symlink_metadata() else {
                return Err(format!("{} no longer exists.", p.display()));
            };
            if i == 0
                && let Some(expected) = item.identity
                && fsx::identity_of(&meta) != expected
            {
                return Err(format!("{} was replaced since the scan.", p.display()));
            }
            if let Err(refusal) = is_actionable(&self.protected, p, &self.allowed) {
                return Err(format!("Refused: {refusal}."));
            }
        }
        let procs = self.k.platform().processes.snapshot();
        if let Some(adapter) = self.adapter(&item.integration) {
            for pattern in adapter.in_use_processes() {
                if procs.any_running(&pattern) {
                    let name = pattern.split('|').next().unwrap_or(&pattern).to_owned();
                    return Err(
                        reasons::reason_with(ReasonCode::ToolRunning, &[("tool", name)]).text,
                    );
                }
            }
        }
        // Cheap recent-write check: the item and its direct children.
        if let Some(p) = item.locator.primary_path()
            && recently_written(p)
        {
            return Err(reasons::reason(ReasonCode::RecentlyWritten).text);
        }
        Ok(())
    }

    fn act(&self, entry: &PlanEntry) -> (Outcome, Option<QuarantineId>) {
        let item = &entry.item;
        // Covers actions that never reach a path guard (native commands,
        // Docker removals): development builds don't change real data.
        if let Err(e) = fsx::change_guard(&self.k.platform().env) {
            return (
                Outcome::Skipped {
                    reason: e.to_string(),
                },
                None,
            );
        }
        match &entry.action {
            ActionKind::Quarantine => {
                match self
                    .quarantine
                    .put(&self.k.platform().env, &self.plan.id, item)
                {
                    Ok(e) => (Outcome::Done { freed: 0 }, Some(e.id)),
                    Err(e) => (
                        Outcome::Failed {
                            error: e.to_string(),
                        },
                        None,
                    ),
                }
            }
            ActionKind::Trash => {
                for p in item.locator.paths() {
                    if let Err(e) = fsx::sandbox_guard(&self.k.platform().env, p) {
                        return (
                            Outcome::Failed {
                                error: e.to_string(),
                            },
                            None,
                        );
                    }
                    if let Err(e) = self.k.platform().trash.trash(p) {
                        return (
                            Outcome::Failed {
                                error: e.to_string(),
                            },
                            None,
                        );
                    }
                }
                (
                    Outcome::Done {
                        freed: item.size.reclaimable,
                    },
                    None,
                )
            }
            action => {
                let Some(adapter) = self.adapter(&item.integration) else {
                    return (
                        Outcome::Failed {
                            error: "integration not loaded".into(),
                        },
                        None,
                    );
                };
                let procs = self.k.platform().processes.snapshot();
                let ctx = Ctx::new(
                    self.k.platform(),
                    self.k.config(),
                    self.k.dirs(),
                    &self.cancel,
                    &procs,
                );
                let failed: Vec<String> = adapter
                    .preflight(&ctx, item, action)
                    .into_iter()
                    .filter(|c| !c.ok)
                    .map(|c| c.message)
                    .collect();
                if !failed.is_empty() {
                    return (
                        Outcome::Skipped {
                            reason: failed.join(" "),
                        },
                        None,
                    );
                }
                match adapter.apply(&ctx, item, action) {
                    Ok(o) => (o, None),
                    Err(e) => (
                        Outcome::Failed {
                            error: e.to_string(),
                        },
                        None,
                    ),
                }
            }
        }
    }

    fn run_entry(&self, index: u64, entry: &PlanEntry) -> ExecEntry {
        self.sink.event(ExecEvent::EntryStarted {
            index,
            title: entry.item.title.clone(),
        });
        let (outcome, qid) = match self.revalidate(entry) {
            Err(reason) => (Outcome::Skipped { reason }, None),
            Ok(()) => self.act(entry),
        };
        let held = matches!(outcome, Outcome::Done { .. }) && qid.is_some();
        let freed = if held { 0 } else { outcome.freed() };
        let record = JournalEntry {
            ts: Timestamp::now(),
            plan_id: self.plan.id.clone(),
            item_id: entry.item.id.clone(),
            integration: entry.item.integration.clone(),
            title: entry.item.title.clone(),
            action: entry.action.clone(),
            outcome: outcome.clone(),
            freed,
            original_path: entry.item.locator.primary_path().cloned(),
            quarantine_id: qid.clone(),
        };
        // Journal before moving on (SPEC §8.3 step 4).
        {
            let _guard = self.journal_lock.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = self.journal.append(&record) {
                self.sink.event(ExecEvent::Warning {
                    message: format!("Couldn't write the journal: {e}"),
                });
            }
        }
        let result = ExecEntry {
            item_id: entry.item.id.clone(),
            title: entry.item.title.clone(),
            action: entry.action.clone(),
            outcome,
            quarantine_id: qid,
        };
        self.sink.event(ExecEvent::EntryFinished {
            index,
            entry: Box::new(result.clone()),
        });
        result
    }
}

fn recently_written(p: &std::path::Path) -> bool {
    let fresh = |m: std::fs::Metadata| {
        fsx::modified_age_secs(&m)
            .is_some_and(|age| age < crate::safety::engine::RECENT_WRITE.as_secs())
    };
    if p.symlink_metadata().is_ok_and(fresh) {
        return true;
    }
    std::fs::read_dir(p)
        .map(|rd| {
            rd.flatten()
                .take(500)
                .any(|e| e.path().symlink_metadata().is_ok_and(fresh))
        })
        .unwrap_or(false)
}

impl Kipple {
    /// Builds a plan from the latest scan (SPEC §8.1). Pure: nothing is
    /// written; see [`Kipple::save_plan`].
    pub fn plan(&self, req: PlanRequest) -> Result<Plan> {
        let (scan, items) = self.all_items()?;
        let mut selected = filter_items(items, &req.filter, false, &self.platform().env)?;
        if !req.exclude_ids.is_empty() {
            let excluded: Vec<ItemId> = req
                .exclude_ids
                .iter()
                .map(|k| crate::kipple::find_in(&selected, k, &self.platform().env).map(|i| i.id))
                .collect::<Result<_>>()?;
            selected.retain(|i| !excluded.contains(&i.id));
        }
        Ok(build_plan(
            scan.scan.id,
            selected,
            &req.action_overrides,
            &Quarantine::new(self.dirs(), self.config().quarantine.keep_days),
            &self.platform().trash,
        ))
    }

    pub fn save_plan(&self, plan: &Plan) -> Result<PathBuf> {
        let dir = self.dirs().plans_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", plan.id));
        std::fs::write(&path, serde_json::to_vec_pretty(plan)?)?;
        Ok(path)
    }

    /// Loads a saved plan if it is still valid: under an hour old and made
    /// from the latest scan.
    pub fn load_plan(&self, id: &str) -> Result<Plan> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(Error::Invalid(format!("`{id}` isn't a plan id")));
        }
        let path = self.dirs().plans_dir().join(format!("{id}.json"));
        let text = std::fs::read_to_string(&path)
            .map_err(|_| Error::Invalid(format!("no saved plan `{id}`")))?;
        let plan: Plan = serde_json::from_str(&text)?;
        if Timestamp::now().as_second() - plan.created_at.as_second() > PLAN_TTL_SECS {
            return Err(Error::Invalid(
                "this plan is more than an hour old; make a new one".into(),
            ));
        }
        let latest = self.latest_scan()?.ok_or(Error::NoScan)?;
        if latest.scan.id != plan.scan_id {
            return Err(Error::Invalid(
                "there's a newer scan than this plan; make a new one".into(),
            ));
        }
        Ok(plan)
    }

    /// Integration-declared locations, recomputed for actionability checks.
    fn allowed_roots(&self, scan: &ScanResult) -> Vec<PathBuf> {
        let cancel = CancelToken::new();
        let procs = crate::platform::ProcessSnapshot::default();
        let ctx = Ctx::new(self.platform(), self.config(), self.dirs(), &cancel, &procs);
        let mut roots = scan.scan.roots.clone();
        for a in self.adapters() {
            roots.extend(a.locations(&ctx));
        }
        roots
    }

    /// Runs a plan (SPEC §8.3). Refuses without the right confirmation.
    pub fn execute(
        &self,
        plan: &Plan,
        confirm: Confirmation,
        sink: &dyn ExecSink,
    ) -> Result<ExecReport> {
        if !confirm.permits(plan) {
            return Err(Error::Invalid(if plan.has_review() {
                "this plan has items to review; confirm by typing `reclaim`".into()
            } else {
                "the plan wasn't confirmed".into()
            }));
        }
        if plan.entries.iter().any(|e| e.item.safety == Safety::Keep) {
            return Err(Error::Invalid(
                "a plan can't contain items marked keep".into(),
            ));
        }
        let scan = self.latest_scan()?.ok_or(Error::NoScan)?;
        // Expired quarantine entries go at the start of every run that acts.
        let _ = self.purge(PurgeRequest::default());

        let protect: Vec<PathBuf> = self
            .config()
            .safety
            .protect
            .iter()
            .filter_map(|p| paths::expand(p, &self.platform().env, &BTreeMap::new()).ok())
            .collect();
        let run = Run {
            k: self,
            plan,
            protected: ProtectedSet::new(
                &self.platform().env,
                self.platform().os,
                self.dirs(),
                &protect,
                &scan.scan.roots,
            ),
            allowed: self.allowed_roots(&scan),
            quarantine: Quarantine::new(self.dirs(), self.config().quarantine.keep_days),
            journal: Journal::new(self.dirs().journal()),
            journal_lock: Mutex::new(()),
            sink,
            cancel: sink.cancel_token().cloned().unwrap_or_default(),
        };
        sink.event(ExecEvent::Started {
            plan_id: plan.id.clone(),
            entries: plan.entries.len() as u64,
        });

        // Sequential per integration, parallel across integrations.
        let mut groups: BTreeMap<&IntegrationId, Vec<(u64, &PlanEntry)>> = BTreeMap::new();
        for (i, e) in plan.entries.iter().enumerate() {
            groups
                .entry(&e.item.integration)
                .or_default()
                .push((i as u64, e));
        }
        let queue: Mutex<VecDeque<Vec<(u64, &PlanEntry)>>> =
            Mutex::new(groups.into_values().collect());
        let results: Mutex<Vec<(u64, ExecEntry)>> = Mutex::new(Vec::new());
        let workers = self.config().exec.parallelism.max(1) as usize;
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| {
                    // The guard moves into the closure and drops there, so the
                    // queue isn't locked while an entry runs.
                    while let Some(group) = queue.lock().ok().and_then(|mut q| q.pop_front()) {
                        for (i, entry) in group {
                            // Cancel: finish the current entry, then stop.
                            if run.cancel.is_cancelled() {
                                break;
                            }
                            let r = run.run_entry(i, entry);
                            if let Ok(mut res) = results.lock() {
                                res.push((i, r));
                            }
                        }
                    }
                });
            }
        });
        let mut results = results.into_inner().unwrap_or_default();
        results.sort_by_key(|(i, _)| *i);
        let entries: Vec<ExecEntry> = results.into_iter().map(|(_, e)| e).collect();
        let mut report = ExecReport {
            plan_id: plan.id.clone(),
            freed_now: 0,
            held_in_quarantine: 0,
            done: 0,
            skipped: 0,
            failed: 0,
            cancelled: run.cancel.is_cancelled() || entries.len() < plan.entries.len(),
            entries: Vec::new(),
        };
        for e in &entries {
            match &e.outcome {
                Outcome::Done { freed } => {
                    report.done += 1;
                    if e.quarantine_id.is_some() {
                        let size = plan
                            .entries
                            .iter()
                            .find(|p| p.item.id == e.item_id)
                            .map(|p| p.item.size.reclaimable)
                            .unwrap_or(0);
                        report.held_in_quarantine += size;
                    } else {
                        report.freed_now += freed;
                    }
                }
                Outcome::Skipped { .. } => report.skipped += 1,
                Outcome::Failed { .. } => report.failed += 1,
            }
        }
        report.entries = entries;
        sink.event(ExecEvent::Finished);
        Ok(report)
    }

    /// Quarantined entries, oldest first.
    pub fn quarantine(&self) -> Result<Vec<QuarantineEntry>> {
        Ok(Quarantine::new(self.dirs(), self.config().quarantine.keep_days).entries())
    }

    /// Restores one entry, optionally into another folder.
    pub fn restore(
        &self,
        id: &QuarantineId,
        to: Option<&std::path::Path>,
    ) -> Result<RestoreReport> {
        let q = Quarantine::new(self.dirs(), self.config().quarantine.keep_days);
        let mut report = RestoreReport::default();
        match q.restore(&self.platform().env, id, to) {
            Ok(r) => report.restored.push(r),
            Err(e) => report.skipped.push(SkippedRestore {
                id: id.clone(),
                reason: e.to_string(),
            }),
        }
        Ok(report)
    }

    /// Restores everything the most recent plan quarantined (`kipple undo`).
    pub fn undo(&self) -> Result<RestoreReport> {
        let q = Quarantine::new(self.dirs(), self.config().quarantine.keep_days);
        let entries = q.entries();
        let Some(latest) = entries
            .iter()
            .max_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)))
        else {
            return Ok(RestoreReport::default());
        };
        let ids: Vec<QuarantineId> = entries
            .iter()
            .filter(|e| e.plan_id == latest.plan_id)
            .map(|e| e.id.clone())
            .collect();
        Ok(q.restore_many(&self.platform().env, &ids))
    }

    /// Deletes quarantine entries for good (SPEC §8.4).
    pub fn purge(&self, req: PurgeRequest) -> Result<PurgeReport> {
        let q = Quarantine::new(self.dirs(), self.config().quarantine.keep_days);
        let targets: Vec<QuarantineId> = if req.all {
            q.entries().into_iter().map(|e| e.id).collect()
        } else if !req.ids.is_empty() {
            req.ids.clone()
        } else {
            q.expired(req.older_than_secs)
                .into_iter()
                .map(|e| e.id)
                .collect()
        };
        let mut report = PurgeReport::default();
        for id in targets {
            match q.purge_one(&self.platform().env, &id) {
                Ok(bytes) => {
                    report.freed += bytes;
                    report.purged.push(id);
                }
                Err(e) => report.errors.push(format!("{id}: {e}")),
            }
        }
        Ok(report)
    }

    /// Executed actions, oldest first (SPEC §8.5).
    pub fn journal(&self) -> Vec<JournalEntry> {
        Journal::new(self.dirs().journal()).read()
    }
}
