//! `kipple scan` and `kipple report`.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::process::ExitCode;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::Result;
use kipple_core::Kipple;
use kipple_core::model::{CancelToken, Report, ScanEvent, ScanRequest, ScanSink, Summary};

use crate::args::{ReportArgs, ReportBy};
use crate::context::Context;
use crate::output::{Align, Style, Table, age_of, category_short, compact_gb, short_path};

/// Shows a one-line progress counter on stderr.
struct Progress {
    enabled: bool,
    last: Mutex<Instant>,
    cancel: CancelToken,
}

impl ScanSink for Progress {
    fn event(&self, event: ScanEvent) {
        if !self.enabled {
            return;
        }
        match event {
            ScanEvent::Progress { done, .. } => {
                let Ok(mut last) = self.last.lock() else {
                    return;
                };
                if last.elapsed().as_millis() < 120 {
                    return;
                }
                *last = Instant::now();
                eprint!("\r\x1b[2KScanning… {} files", compact_count(done));
                let _ = std::io::stderr().flush();
            }
            ScanEvent::Finished { .. } => {
                eprint!("\r\x1b[2K");
                let _ = std::io::stderr().flush();
            }
            _ => {}
        }
    }

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}

/// `1.02M`, `532k`, `812`.
pub fn compact_count(n: u64) -> String {
    match n {
        n if n >= 1_000_000 => format!("{:.2}M", n as f64 / 1e6),
        n if n >= 10_000 => format!("{}k", n / 1000),
        n => n.to_string(),
    }
}

pub fn scan(ctx: &mut Context) -> Result<ExitCode> {
    let k = ctx.open()?;
    let req = ScanRequest {
        roots: ctx.global.root.clone(),
        only: ctx.global.only.clone(),
        exclude: ctx.global.exclude.clone(),
        follow_symlinks: None,
    };
    let progress = Progress {
        enabled: !ctx.global.json && !ctx.global.quiet && std::io::stderr().is_terminal(),
        last: Mutex::new(Instant::now()),
        cancel: CancelToken::new(),
    };
    let result = k.scan(req, &progress)?;
    if ctx.global.json {
        ctx.print_json(&result)?;
        return Ok(ExitCode::SUCCESS);
    }
    for w in &result.warnings {
        eprintln!("{}", ctx.style().dim(w));
    }
    print_report(ctx, &k, &k.report()?, ReportBy::Category);
    Ok(ExitCode::SUCCESS)
}

pub fn report(ctx: &mut Context, args: &ReportArgs) -> Result<ExitCode> {
    let k = ctx.open()?;
    let report = k.report()?;
    if ctx.global.json {
        ctx.print_json(&report)?;
    } else {
        print_report(ctx, &k, &report, args.by);
    }
    Ok(ExitCode::SUCCESS)
}

fn header(style: &Style, report: &Report) -> String {
    let left = format!(
        "kipple · scanned {} files in {:.1}s",
        compact_count(report.scan.entries_seen),
        report.scan.duration_ms as f64 / 1000.0
    );
    let right = format!("reclaimable {}", style.size(report.summary.reclaimable));
    let gap = 78usize
        .saturating_sub(left.chars().count() + right.chars().count())
        .max(3);
    format!("{}{}{}", left, " ".repeat(gap), style.bold(&right))
}

fn category_table(style: &Style, summary: &Summary) -> Table {
    let mut t = Table::new(&[
        ("Category", Align::Left),
        ("Items", Align::Right),
        ("Reclaimable", Align::Right),
        ("Safe", Align::Right),
        ("Review", Align::Right),
    ]);
    for c in &summary.categories {
        t.row(vec![
            c.category.label().to_owned(),
            c.items.to_string(),
            style.size(c.reclaimable),
            style.size_or_dash(c.by_safety.safe),
            style.size_or_dash(c.by_safety.review),
        ]);
    }
    t
}

fn print_report(ctx: &Context, k: &Kipple, report: &Report, by: ReportBy) {
    let style = ctx.style();
    let home = ctx.home();
    println!("{}\n", header(&style, report));
    if report.summary.items == 0 {
        println!("Nothing to reclaim was found.");
        return;
    }
    match by {
        ReportBy::Category => print!("{}", category_table(&style, &report.summary).render(&style)),
        ReportBy::Integration => {
            let mut rows: BTreeMap<String, (u64, u64, u64, u64)> = BTreeMap::new();
            if let Ok((_, items)) = k.all_items() {
                for i in items {
                    let r = rows.entry(i.integration.to_string()).or_default();
                    r.0 += 1;
                    r.1 += i.size.reclaimable;
                    match i.safety {
                        kipple_core::model::Safety::Safe => r.2 += i.size.reclaimable,
                        kipple_core::model::Safety::Review => r.3 += i.size.reclaimable,
                        kipple_core::model::Safety::Keep => {}
                    }
                }
            }
            let mut sorted: Vec<_> = rows.into_iter().collect();
            sorted.sort_by_key(|a| std::cmp::Reverse(a.1.1));
            let mut t = Table::new(&[
                ("Integration", Align::Left),
                ("Items", Align::Right),
                ("Reclaimable", Align::Right),
                ("Safe", Align::Right),
                ("Review", Align::Right),
            ]);
            for (id, (n, total, safe, review)) in sorted {
                t.row(vec![
                    id,
                    n.to_string(),
                    style.size(total),
                    style.size_or_dash(safe),
                    style.size_or_dash(review),
                ]);
            }
            print!("{}", t.render(&style));
        }
        ReportBy::Project => {}
    }
    let projects = if by == ReportBy::Project {
        k.projects(Default::default()).unwrap_or_default()
    } else {
        report.top_projects.clone()
    };
    if !projects.is_empty() {
        println!();
        let title = if by == ReportBy::Project {
            "Projects"
        } else {
            "Heaviest projects"
        };
        let mut t = Table::new(&[
            (title, Align::Left),
            ("Last used", Align::Left),
            ("Reclaimable", Align::Right),
            ("", Align::Left),
        ]);
        for p in &projects {
            let mut parts: Vec<(u64, String)> = p
                .by_category
                .iter()
                .filter(|(_, b)| **b > 0)
                .map(|(c, b)| {
                    (
                        *b,
                        format!("{} {}", category_short(*c), compact_gb(*b, &style)),
                    )
                })
                .collect();
            parts.sort_by_key(|a| std::cmp::Reverse(a.0));
            let breakdown: Vec<String> = parts.into_iter().take(3).map(|(_, s)| s).collect();
            t.row(vec![
                short_path(&p.root, home.as_deref()),
                age_of(p.last_used.as_ref()),
                style.size(p.total.reclaimable),
                style.dim(&breakdown.join(", ")),
            ]);
        }
        print!("{}", t.render(&style));
    }
    if report.summary.quarantine_bytes > 0 {
        println!(
            "\n{}",
            style.dim(&format!(
                "Quarantine holds {} that is freed after it expires, or now with `kipple quarantine purge`.",
                style.size(report.summary.quarantine_bytes)
            ))
        );
    }
    if let Some(next) = &report.next {
        let undo = if next.undoable > 0 {
            format!(
                " ({} of it can be undone for {} days)",
                style.size(next.undoable),
                next.keep_days
            )
        } else {
            String::new()
        };
        println!(
            "\nNext: {}   reclaims {}{}",
            style.bold(&next.command),
            style.size(next.reclaimable),
            undo
        );
    }
}
