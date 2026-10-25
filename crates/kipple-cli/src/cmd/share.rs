//! `kipple share` (SPEC §14).

use std::process::ExitCode;

use anyhow::{Context as _, Result};
use kipple_core::model::{Os, Outcome, Timestamp};
use kipple_share::CardData;
use serde::Serialize;

use crate::args::ShareArgs;
use crate::context::Context;

#[derive(Serialize)]
struct ShareOutput {
    path: String,
    total: u64,
    freed: Option<u64>,
}

pub fn share(ctx: &mut Context, args: &ShareArgs) -> Result<ExitCode> {
    let k = ctx.open()?;
    let report = k.report()?;
    // Before/after variant when something was freed in the last 24 hours.
    let day_ago = Timestamp::now().as_second() - 86_400;
    let freed: u64 = k
        .journal()
        .iter()
        .filter(|j| j.ts.as_second() >= day_ago)
        .filter(|j| matches!(j.outcome, Outcome::Done { .. }))
        .map(|j| {
            if j.quarantine_id.is_some() {
                0
            } else {
                j.freed
            }
        })
        .sum::<u64>()
        + k.quarantine()?
            .iter()
            .filter(|e| e.created_at.as_second() >= day_ago)
            .map(|e| e.size.reclaimable)
            .sum::<u64>();
    let projects = if args.with_projects {
        report
            .top_projects
            .iter()
            .filter(|p| p.exists)
            .filter_map(|p| p.root.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect()
    } else {
        Vec::new()
    };
    let data = CardData {
        total: report.summary.reclaimable,
        freed: (freed > 0).then_some(freed),
        categories: report
            .summary
            .categories
            .iter()
            .map(|c| (c.category, c.reclaimable))
            .collect(),
        device: match k.platform().os {
            Os::Macos => "Mac",
            Os::Windows => "PC",
            Os::Linux | Os::Wsl => "machine",
        }
        .to_owned(),
        projects,
        units: k.config().ui.units,
        url: "github.com/Paras-ydv/kipple".to_owned(),
    };
    let png = kipple_share::render_png(&data)?;
    std::fs::write(&args.out, png).with_context(|| format!("writing {}", args.out.display()))?;
    if ctx.global.json {
        ctx.print_json(ShareOutput {
            path: args.out.display().to_string(),
            total: data.total,
            freed: data.freed,
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
