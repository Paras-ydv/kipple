//! `kipple clean`, `undo`, `quarantine`, `history` (SPEC §8, §11.2).

use std::io::{BufRead, Write};
use std::process::ExitCode;

use anyhow::{Result, bail};
use kipple_core::model::*;
use kipple_core::{Kipple, PLAN_TTL_SECS};

use crate::args::{CleanArgs, HistoryArgs, QuarantineCommand};
use crate::context::Context;
use crate::exit;
use crate::output::{Align, Style, Table, short_path, truncate};

/// Usage errors carry their own exit code.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct UsageError(pub String);

fn usage(msg: impl Into<String>) -> anyhow::Error {
    UsageError(msg.into()).into()
}

/// Reads one answer from stdin after printing `prompt` to stderr.
fn ask(prompt: &str) -> String {
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
    line.trim().to_owned()
}

/// Prints progress lines and lets Ctrl-C finish the current entry.
struct Progress {
    style: Style,
    cancel: CancelToken,
    quiet: bool,
}

impl ExecSink for Progress {
    fn event(&self, event: ExecEvent) {
        if self.quiet {
            return;
        }
        if let ExecEvent::EntryFinished { entry, .. } = event {
            let (word, detail) = match &entry.outcome {
                Outcome::Done { .. } => (
                    self.style.safety(Safety::Safe).replace("safe", "done"),
                    String::new(),
                ),
                Outcome::Skipped { reason } => (
                    "skipped".to_owned(),
                    format!("  {}", self.style.dim(reason)),
                ),
                Outcome::Failed { error } => (
                    self.style.safety(Safety::Keep).replace("keep", "failed"),
                    format!("  {}", self.style.dim(error)),
                ),
            };
            eprintln!("{word:<8} {}{detail}", truncate(&entry.title, 50));
        }
    }

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}

pub fn print_plan(ctx: &Context, plan: &Plan, keep_days: u32) {
    let style = ctx.style();
    let home = ctx.home();
    println!(
        "{}",
        style.bold(&format!(
            "Plan · {} items · reclaims {}",
            plan.totals.entries,
            style.size(plan.totals.reclaimable)
        ))
    );
    for level in [Safety::Safe, Safety::Review] {
        let entries: Vec<&PlanEntry> = plan
            .entries
            .iter()
            .filter(|e| e.item.safety == level)
            .collect();
        if entries.is_empty() {
            continue;
        }
        println!(
            "\n{} {}",
            style.safety(level),
            style.dim(&style.size(plan.totals.by_safety.get(level)))
        );
        let mut t = Table::new(&[
            ("Id", Align::Left),
            ("Size", Align::Right),
            ("Action", Align::Left),
            ("Item", Align::Left),
            ("Path", Align::Left),
        ]);
        for e in entries {
            let path = e
                .item
                .locator
                .primary_path()
                .map(|p| short_path(p, home.as_deref()))
                .unwrap_or_default();
            t.row(vec![
                e.item.id.as_str()[..8].to_owned(),
                style.size(e.item.size.reclaimable),
                e.action.to_string(),
                truncate(&e.item.title, 36),
                style.dim(&path),
            ]);
        }
        print!("{}", t.render(&style));
    }
    println!();
    if plan.totals.freed_now > 0 {
        println!(
            "{} is freed right away by the tools that made it or the trash. It can't be undone with kipple.",
            style.size(plan.totals.freed_now)
        );
    }
    if plan.totals.held_in_quarantine > 0 {
        println!(
            "{} waits in quarantine so you can undo it, then it is deleted after {keep_days} days.",
            style.size(plan.totals.held_in_quarantine)
        );
    }
    for w in &plan.warnings {
        println!("{}", style.dim(w));
    }
}

pub fn clean(ctx: &mut Context, args: &CleanArgs) -> Result<ExitCode> {
    if args.yes && !ctx.global.safe_only {
        return Err(usage(
            "--yes only works together with --safe-only. Review items always need you to type `reclaim`.",
        ));
    }
    let k = ctx.open()?;
    let plan = match &args.plan {
        Some(id) => k.load_plan(id)?,
        None => {
            let mut filter = ctx.filter()?;
            filter.ids = args.items.clone();
            if !filter.is_selective() && ctx.interactive() {
                // No selection in a terminal: pick in the TUI (decision 0003).
                drop(k);
                return crate::cmd::tui(ctx, false);
            }
            if !filter.is_selective() {
                return Err(usage(
                    "Say what to clean: item ids, or a filter such as --safe-only, --only <integration>, --older-than 30d or --min-size 1GB.\nTry `kipple clean --safe-only` to see a plan of everything marked safe.",
                ));
            }
            k.plan(PlanRequest {
                filter,
                exclude_ids: args.skip.clone(),
                ..Default::default()
            })?
        }
    };
    if plan.entries.is_empty() {
        if ctx.global.json {
            ctx.print_json(&plan)?;
        } else {
            println!("Nothing to clean matches.");
            for w in &plan.warnings {
                println!("{}", ctx.style().dim(w));
            }
        }
        return Ok(ExitCode::SUCCESS);
    }
    if args.plan_only {
        k.save_plan(&plan)?;
        if ctx.global.json {
            ctx.print_json(&plan)?;
        } else {
            print_plan(ctx, &plan, k.config().quarantine.keep_days);
            println!(
                "\nSaved plan {}. Run it within {} minutes with `kipple clean --plan {}`.",
                plan.id,
                PLAN_TTL_SECS / 60,
                plan.id
            );
        }
        return Ok(ExitCode::SUCCESS);
    }
    if ctx.global.json && !args.yes {
        return Err(usage(
            "--json can't ask for confirmation. Use --plan-only, or --safe-only --yes.",
        ));
    }
    if !ctx.global.json {
        print_plan(ctx, &plan, k.config().quarantine.keep_days);
        println!();
    }
    let confirmation = if args.yes {
        if plan.has_review() {
            return Err(usage(
                "This plan has items to review, so it needs you to type `reclaim`.",
            ));
        }
        Confirmation::Yes
    } else if plan.has_review() {
        let word = ask("This plan has items to review. Type reclaim to continue: ");
        if word != Confirmation::REVIEW_WORD {
            eprintln!("Nothing was changed.");
            return Ok(ExitCode::from(exit::ABORTED));
        }
        Confirmation::Typed { word }
    } else {
        let word = ask("Continue? [y/N] ");
        if !word.eq_ignore_ascii_case("y") && !word.eq_ignore_ascii_case("yes") {
            eprintln!("Nothing was changed.");
            return Ok(ExitCode::from(exit::ABORTED));
        }
        Confirmation::Typed { word: "y".into() }
    };

    let progress = Progress {
        style: ctx.style(),
        cancel: CancelToken::new(),
        quiet: ctx.global.json || ctx.global.quiet,
    };
    let interrupted = progress.cancel.clone();
    let _ = ctrlc::set_handler(move || {
        if interrupted.is_cancelled() {
            std::process::exit(i32::from(exit::INTERRUPTED));
        }
        eprintln!("\nStopping after the current item. Press Ctrl-C again to quit now.");
        interrupted.cancel();
    });
    let report = k.execute(&plan, confirmation, &progress)?;

    let mut purged = None;
    if args.purge_now && report.held_in_quarantine > 0 {
        let ids: Vec<QuarantineId> = report
            .entries
            .iter()
            .filter_map(|e| e.quarantine_id.clone())
            .collect();
        let go = args.yes
            || ask(&format!(
                "Type purge to delete the {} in quarantine now. This can't be undone: ",
                ctx.style().size(report.held_in_quarantine)
            )) == "purge";
        if go {
            purged = Some(k.purge(PurgeRequest {
                ids,
                ..Default::default()
            })?);
        }
    }

    if ctx.global.json {
        ctx.print_json(&report)?;
    } else {
        print_outcome(ctx, &k, &report, purged.as_ref());
    }
    Ok(if progress.cancel.is_cancelled() {
        ExitCode::from(exit::INTERRUPTED)
    } else if report.is_partial() {
        ExitCode::from(exit::PARTIAL)
    } else {
        ExitCode::SUCCESS
    })
}

fn print_outcome(ctx: &Context, k: &Kipple, report: &ExecReport, purged: Option<&PurgeReport>) {
    let style = ctx.style();
    println!();
    let mut freed = report.freed_now;
    let mut held = report.held_in_quarantine;
    if let Some(p) = purged {
        freed += p.freed.min(held);
        held = held.saturating_sub(p.freed);
    }
    if freed > 0 {
        println!("Freed {}.", style.bold(&style.size(freed)));
    }
    if held > 0 {
        println!(
            "{} is in quarantine for {} days. Undo with `kipple undo`, or free it now with `kipple quarantine purge --all`.",
            style.size(held),
            k.config().quarantine.keep_days
        );
    }
    if report.skipped > 0 || report.failed > 0 {
        println!(
            "{} done, {} skipped, {} failed.",
            report.done, report.skipped, report.failed
        );
    }
    if report.cancelled {
        println!("Stopped early. `kipple history` shows what happened.");
    }
}

pub fn undo(ctx: &mut Context) -> Result<ExitCode> {
    let k = ctx.open()?;
    let report = k.undo()?;
    if ctx.global.json {
        ctx.print_json(&report)?;
    } else {
        print_restore(ctx, &report);
    }
    Ok(if report.skipped.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(exit::PARTIAL)
    })
}

fn print_restore(ctx: &Context, report: &RestoreReport) {
    let home = ctx.home();
    if report.restored.is_empty() && report.skipped.is_empty() {
        println!("Quarantine is empty; there's nothing to undo.");
        return;
    }
    for r in &report.restored {
        let path = r
            .paths
            .first()
            .map(|p| short_path(p, home.as_deref()))
            .unwrap_or_default();
        println!("restored  {}  {}", r.title, ctx.style().dim(&path));
    }
    for s in &report.skipped {
        println!("skipped   {}  {}", s.id, ctx.style().dim(&s.reason));
    }
}

pub fn quarantine(ctx: &mut Context, cmd: &QuarantineCommand) -> Result<ExitCode> {
    let k = ctx.open()?;
    let style = ctx.style();
    let home = ctx.home();
    match cmd {
        QuarantineCommand::List => {
            let entries = k.quarantine()?;
            if ctx.global.json {
                ctx.print_json(&entries)?;
                return Ok(ExitCode::SUCCESS);
            }
            if entries.is_empty() {
                println!("Quarantine is empty.");
                return Ok(ExitCode::SUCCESS);
            }
            let mut t = Table::new(&[
                ("Id", Align::Left),
                ("Size", Align::Right),
                ("Deleted in", Align::Left),
                ("Item", Align::Left),
                ("From", Align::Left),
            ]);
            let now = Timestamp::now().as_second();
            let mut total = 0;
            for e in &entries {
                total += e.size.on_disk;
                let left = e.expires_at.as_second() - now;
                let left = if left <= 0 {
                    "next run".to_owned()
                } else {
                    kipple_core::format::age(left)
                };
                let from = e
                    .original_paths
                    .first()
                    .map(|p| short_path(p, home.as_deref()))
                    .unwrap_or_default();
                t.row(vec![
                    e.id.to_string(),
                    style.size(e.size.on_disk),
                    left,
                    truncate(&e.item.title, 36),
                    style.dim(&from),
                ]);
            }
            print!("{}", t.render(&style));
            println!("\n{} items, {} held.", entries.len(), style.size(total));
        }
        QuarantineCommand::Restore { id, to } => {
            let id = resolve_qid(&k, id)?;
            let report = k.restore(&id, to.as_deref())?;
            if ctx.global.json {
                ctx.print_json(&report)?;
            } else {
                print_restore(ctx, &report);
            }
            if !report.skipped.is_empty() {
                return Ok(ExitCode::from(exit::ERROR));
            }
        }
        QuarantineCommand::Purge { all, yes } => {
            let older = ctx
                .global
                .older_than
                .as_deref()
                .map(kipple_core::format::parse_age)
                .transpose()?;
            let req = PurgeRequest {
                older_than_secs: older,
                all: *all,
                ids: Vec::new(),
            };
            let targets = if *all { k.quarantine()?.len() } else { 0 };
            if *all && !*yes && targets > 0 {
                if ctx.global.json {
                    return Err(usage("--json can't ask for confirmation; add --yes."));
                }
                let word = ask(&format!(
                    "Type purge to delete all {targets} quarantined items for good: "
                ));
                if word != "purge" {
                    eprintln!("Nothing was changed.");
                    return Ok(ExitCode::from(exit::ABORTED));
                }
            }
            let report = k.purge(req)?;
            if ctx.global.json {
                ctx.print_json(&report)?;
            } else if report.purged.is_empty() {
                println!("Nothing to purge.");
            } else {
                println!(
                    "Deleted {} items, freed {}.",
                    report.purged.len(),
                    style.size(report.freed)
                );
            }
            for e in &report.errors {
                eprintln!("{e}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_qid(k: &Kipple, key: &str) -> Result<QuarantineId> {
    let entries = k.quarantine()?;
    let hits: Vec<&QuarantineEntry> = entries
        .iter()
        .filter(|e| {
            e.id.as_str().eq_ignore_ascii_case(key)
                || e.id
                    .as_str()
                    .to_ascii_lowercase()
                    .starts_with(&key.to_ascii_lowercase())
        })
        .collect();
    match hits.as_slice() {
        [one] => Ok(one.id.clone()),
        [] => bail!("nothing in quarantine matches `{key}`"),
        _ => bail!("`{key}` matches more than one quarantine entry; use more characters"),
    }
}

pub fn history(ctx: &mut Context, args: &HistoryArgs) -> Result<ExitCode> {
    let k = ctx.open()?;
    let style = ctx.style();
    if args.growth {
        let points = k.history(HistoryRequest {
            scope: SizeScope::Category,
            key: None,
            since: None,
        })?;
        if ctx.global.json {
            ctx.print_json(&points)?;
            return Ok(ExitCode::SUCCESS);
        }
        if points.is_empty() {
            println!("No scans yet.");
            return Ok(ExitCode::SUCCESS);
        }
        // One column per scan date, one row per category.
        let mut dates: Vec<i64> = points.iter().map(|p| p.ts.as_second()).collect();
        dates.sort_unstable();
        dates.dedup();
        let dates: Vec<i64> = dates.into_iter().rev().take(6).rev().collect();
        let labels: Vec<String> = dates
            .iter()
            .map(|d| {
                Timestamp::from_second(*d)
                    .map(|t| t.strftime("%b %d").to_string())
                    .unwrap_or_default()
            })
            .collect();
        let mut headers: Vec<(&str, Align)> = vec![("Category", Align::Left)];
        for l in &labels {
            headers.push((l.as_str(), Align::Right));
        }
        let mut t = Table::new(&headers);
        let mut cats: Vec<String> = points.iter().map(|p| p.key.clone()).collect();
        cats.sort();
        cats.dedup();
        for c in cats {
            let mut row = vec![
                c.parse::<Category>()
                    .map(|c| c.label().to_owned())
                    .unwrap_or(c.clone()),
            ];
            for d in &dates {
                let v = points
                    .iter()
                    .find(|p| p.key == c && p.ts.as_second() == *d)
                    .map(|p| p.reclaimable);
                row.push(v.map(|v| style.size(v)).unwrap_or_else(|| "—".into()));
            }
            t.row(row);
        }
        print!("{}", t.render(&style));
        return Ok(ExitCode::SUCCESS);
    }
    let mut journal = k.journal();
    let skip = journal.len().saturating_sub(args.limit);
    journal.drain(..skip);
    if ctx.global.json {
        ctx.print_json(&journal)?;
        return Ok(ExitCode::SUCCESS);
    }
    if journal.is_empty() {
        println!("kipple hasn't changed anything yet.");
        return Ok(ExitCode::SUCCESS);
    }
    let home = ctx.home();
    let mut t = Table::new(&[
        ("When", Align::Left),
        ("Action", Align::Left),
        ("Result", Align::Left),
        ("Freed", Align::Right),
        ("Item", Align::Left),
        ("Path", Align::Left),
    ]);
    let now = Timestamp::now().as_second();
    for j in &journal {
        let result = match &j.outcome {
            Outcome::Done { .. } if j.quarantine_id.is_some() => "quarantined",
            Outcome::Done { .. } => "done",
            Outcome::Skipped { .. } => "skipped",
            Outcome::Failed { .. } => "failed",
        };
        t.row(vec![
            format!("{} ago", kipple_core::format::age(now - j.ts.as_second())),
            j.action.to_string(),
            result.to_owned(),
            style.size_or_dash(j.freed),
            truncate(&j.title, 36),
            style.dim(
                &j.original_path
                    .as_ref()
                    .map(|p| short_path(p, home.as_deref()))
                    .unwrap_or_default(),
            ),
        ]);
    }
    print!("{}", t.render(&style));
    Ok(ExitCode::SUCCESS)
}
