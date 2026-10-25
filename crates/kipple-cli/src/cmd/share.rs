//! `kipple share` (SPEC §14, UI.md §5).

use std::collections::BTreeMap;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use kipple_core::model::{Category, Os, Outcome, Timestamp};
use kipple_share::{CardData, Variant};
use serde::Serialize;

use crate::args::ShareArgs;
use crate::context::Context;

#[derive(Serialize)]
struct ShareOutput {
    path: String,
    variant: &'static str,
    total: u64,
}

pub fn share(ctx: &mut Context, args: &ShareArgs) -> Result<ExitCode> {
    let k = ctx.open()?;
    let report = k.report()?;
    let categories_of: BTreeMap<String, Category> = k
        .adapters()
        .iter()
        .map(|a| {
            let i = a.info();
            (i.id.to_string(), i.category)
        })
        .collect();

    // Freed in the last 24 hours, by category: native/trash actions from the
    // journal plus what went to quarantine.
    let day_ago = Timestamp::now().as_second() - 86_400;
    let mut freed: BTreeMap<Category, u64> = BTreeMap::new();
    for j in k.journal().iter().filter(|j| j.ts.as_second() >= day_ago) {
        if matches!(j.outcome, Outcome::Done { .. }) && j.quarantine_id.is_none()
            && let Some(c) = categories_of.get(j.integration.as_str()) {
                *freed.entry(*c).or_default() += j.freed;
            }
    }
    for e in k
        .quarantine()?
        .iter()
        .filter(|e| e.created_at.as_second() >= day_ago)
    {
        *freed.entry(e.item.category).or_default() += e.item.size.reclaimable;
    }
    let freed_total: u64 = freed.values().sum();

    let (variant, total, categories) = if freed_total > 0 {
        (Variant::Reclaimed, freed_total, freed.into_iter().collect())
    } else {
        (
            Variant::Found,
            report.summary.reclaimable,
            report
                .summary
                .categories
                .iter()
                .map(|c| (c.category, c.reclaimable))
                .collect(),
        )
    };
    let projects = if args.with_projects {
        report
            .top_projects
            .iter()
            .filter(|p| p.exists)
            .filter_map(|p| {
                let name = p.root.file_name()?.to_string_lossy().into_owned();
                Some((name, p.total.reclaimable))
            })
            .collect()
    } else {
        Vec::new()
    };
    let data = CardData {
        variant,
        total,
        device: match k.platform().os {
            Os::Macos => "Mac",
            Os::Windows => "PC",
            Os::Linux | Os::Wsl => "computer",
        }
        .to_owned(),
        categories,
        remaining: report.summary.reclaimable.saturating_sub(freed_total),
        projects,
    };
    let png = kipple_share::render_png(&data)?;
    std::fs::write(&args.out, png).with_context(|| format!("writing {}", args.out.display()))?;
    if ctx.global.json {
        ctx.print_json(ShareOutput {
            path: args.out.display().to_string(),
            variant: match variant {
                Variant::Found => "found",
                Variant::Reclaimed => "reclaimed",
            },
            total,
        })?;
    } else {
        println!("Wrote {}.", args.out.display());
    }
    if args.open {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(windows) {
            "explorer"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(opener).arg(&args.out).status();
    }
    Ok(ExitCode::SUCCESS)
}
