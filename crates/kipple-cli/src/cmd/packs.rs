//! `kipple packs list|show|validate|test`.

use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use kipple_core::model::{ItemFilter, ItemQuery, NoopSink, ScanRequest};
use serde::Serialize;

use crate::args::PacksCommand;
use crate::context::Context;
use crate::output::{Align, Table, short_path};

#[derive(Serialize)]
struct ValidateResult {
    file: String,
    id: Option<String>,
    errors: Vec<String>,
}

pub fn run(ctx: &mut Context, cmd: &PacksCommand) -> Result<ExitCode> {
    match cmd {
        PacksCommand::List => {
            let packs = kipple_packs::builtin_packs()?;
            if ctx.global.json {
                ctx.print_json(&packs)?;
                return Ok(ExitCode::SUCCESS);
            }
            let style = ctx.style();
            let mut t = Table::new(&[
                ("Id", Align::Left),
                ("Category", Align::Left),
                ("Name", Align::Left),
            ]);
            for p in &packs {
                let status = if p.status == kipple_core::model::IntegrationStatus::Experimental {
                    " (experimental)"
                } else {
                    ""
                };
                t.row(vec![
                    p.id.clone(),
                    p.category.label().to_owned(),
                    format!("{}{status}", p.name),
                ]);
            }
            print!("{}", t.render(&style));
        }
        PacksCommand::Show { id } => {
            let file = format!("{id}.toml");
            let Some((_, text)) = kipple_packs::builtin_sources()
                .iter()
                .find(|(f, _)| *f == file)
            else {
                bail!("no built-in pack named `{id}`");
            };
            if ctx.global.json {
                ctx.print_json(
                    kipple_packs::parse_and_validate(text)
                        .map_err(|e| anyhow::anyhow!(e.join("; ")))?,
                )?;
            } else {
                print!("{text}");
            }
        }
        PacksCommand::Validate { files } => {
            let mut results = Vec::new();
            for f in files {
                let text = std::fs::read_to_string(f)
                    .with_context(|| format!("reading {}", f.display()))?;
                let (id, errors) = match kipple_packs::parse_and_validate(&text) {
                    Ok(p) => {
                        let mut errors = Vec::new();
                        let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
                        if stem != p.id {
                            errors.push(format!("file name should be {}.toml", p.id));
                        }
                        (Some(p.id), errors)
                    }
                    Err(e) => (None, e),
                };
                results.push(ValidateResult {
                    file: f.display().to_string(),
                    id,
                    errors,
                });
            }
            let failed = results.iter().any(|r| !r.errors.is_empty());
            if ctx.global.json {
                ctx.print_json(&results)?;
            } else {
                for r in &results {
                    if r.errors.is_empty() {
                        println!("ok     {}", r.file);
                    } else {
                        println!("error  {}", r.file);
                        for e in &r.errors {
                            println!("       {e}");
                        }
                    }
                }
            }
            if failed {
                return Ok(ExitCode::from(crate::exit::ERROR));
            }
        }
        PacksCommand::Test { id } => {
            let k = ctx.open()?;
            k.scan(
                ScanRequest {
                    roots: ctx.global.root.clone(),
                    only: vec![id.clone()],
                    ..Default::default()
                },
                &NoopSink::default(),
            )?;
            let items = k.items(ItemQuery {
                filter: ItemFilter {
                    only: vec![id.clone()],
                    ..Default::default()
                },
                include_keep: true,
                ..Default::default()
            })?;
            if ctx.global.json {
                ctx.print_json(&items)?;
                return Ok(ExitCode::SUCCESS);
            }
            let style = ctx.style();
            let home = ctx.home();
            println!("{} found {} items.", id, items.len());
            for i in &items {
                let path = i
                    .locator
                    .primary_path()
                    .map(|p| short_path(p, home.as_deref()))
                    .unwrap_or_default();
                println!(
                    "  {:>9}  {:<6}  {}  {}",
                    style.size(i.size.reclaimable),
                    style.safety(i.safety),
                    i.title,
                    style.dim(&path)
                );
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
