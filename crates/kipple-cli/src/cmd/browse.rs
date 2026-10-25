//! `kipple items`, `kipple projects`, `kipple explain`.

use std::process::ExitCode;

use anyhow::Result;
use kipple_core::model::{ItemQuery, ItemSort, ProjectQuery, ProjectSort, Safety};

use crate::args::{ItemsArgs, ProjectsArgs, SortKey};
use crate::context::Context;
use crate::output::{Align, Table, age_of, category_short, compact_gb, short_path, truncate};

pub fn items(ctx: &mut Context, args: &ItemsArgs) -> Result<ExitCode> {
    let k = ctx.open()?;
    let query = ItemQuery {
        filter: ctx.filter()?,
        sort: match args.sort {
            SortKey::Size => ItemSort::Size,
            SortKey::Age => ItemSort::Age,
            SortKey::Name => ItemSort::Name,
        },
        limit: args.limit,
        include_keep: args.all,
    };
    let items = k.items(query)?;
    if ctx.global.json {
        ctx.print_json(&items)?;
        return Ok(ExitCode::SUCCESS);
    }
    let style = ctx.style();
    let home = ctx.home();
    if items.is_empty() {
        println!("No items match.");
        return Ok(ExitCode::SUCCESS);
    }
    let mut t = Table::new(&[
        ("Id", Align::Left),
        ("Size", Align::Right),
        ("Safety", Align::Left),
        ("Last used", Align::Left),
        ("Item", Align::Left),
        ("Path", Align::Left),
    ]);
    let mut total = 0;
    for i in &items {
        total += i.size.reclaimable;
        let path = i
            .locator
            .primary_path()
            .map(|p| short_path(p, home.as_deref()))
            .unwrap_or_default();
        t.row(vec![
            i.id.as_str()[..8].to_owned(),
            style.size(i.size.reclaimable),
            style.safety(i.safety),
            age_of(i.last_used.as_ref()),
            truncate(&i.title, 40),
            style.dim(&path),
        ]);
    }
    print!("{}", t.render(&style));
    println!("\n{} items, {}", items.len(), style.size(total));
    Ok(ExitCode::SUCCESS)
}

pub fn projects(ctx: &mut Context, args: &ProjectsArgs) -> Result<ExitCode> {
    let k = ctx.open()?;
    let projects = k.projects(ProjectQuery {
        orphaned: args.orphaned,
        sort: match args.sort {
            SortKey::Size => ProjectSort::Size,
            SortKey::Age => ProjectSort::Age,
            SortKey::Name => ProjectSort::Name,
        },
        limit: args.limit,
        min_size: ctx.filter()?.min_size,
    })?;
    if ctx.global.json {
        ctx.print_json(&projects)?;
        return Ok(ExitCode::SUCCESS);
    }
    let style = ctx.style();
    let home = ctx.home();
    if projects.is_empty() {
        println!(
            "{}",
            if args.orphaned {
                "No items belong to deleted projects."
            } else {
                "No projects hold reclaimable items."
            }
        );
        return Ok(ExitCode::SUCCESS);
    }
    let mut t = Table::new(&[
        ("Project", Align::Left),
        ("Path", Align::Left),
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
        let name = if p.exists {
            p.name.clone()
        } else {
            format!("{} (deleted)", p.name)
        };
        t.row(vec![
            truncate(&name, 32),
            style.dim(&short_path(&p.root, home.as_deref())),
            age_of(p.last_used.as_ref()),
            style.size(p.total.reclaimable),
            style.dim(
                &parts
                    .into_iter()
                    .take(3)
                    .map(|(_, s)| s)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ]);
    }
    print!("{}", t.render(&style));
    Ok(ExitCode::SUCCESS)
}

pub fn explain(ctx: &mut Context, key: &str) -> Result<ExitCode> {
    let k = ctx.open()?;
    let e = k.explain(key)?;
    if ctx.global.json {
        ctx.print_json(&e)?;
        return Ok(ExitCode::SUCCESS);
    }
    let style = ctx.style();
    let home = ctx.home();
    let i = &e.item;
    println!("{}", style.bold(&i.title));
    println!(
        "{}",
        style.dim(&format!("{} · {}", e.integration.name, i.id))
    );
    println!();
    if !e.integration.description.is_empty() {
        println!("{}\n", e.integration.description);
    }
    for p in i.locator.paths() {
        println!("Path         {}", short_path(p, home.as_deref()));
    }
    if let Some(p) = &i.project {
        let gone = if p.root.exists() {
            ""
        } else {
            " (folder no longer exists)"
        };
        println!(
            "Project      {} — {}{gone}",
            p.name,
            short_path(&p.root, home.as_deref())
        );
    }
    println!(
        "Last used    {}",
        match &i.last_used {
            Some(_) => format!("{} ago", age_of(i.last_used.as_ref())),
            None => "unknown".into(),
        }
    );
    println!(
        "Size         {} reclaimable{}",
        style.size(i.size.reclaimable),
        if e.size_differs {
            format!(
                " ({} on disk, {} apparent)",
                style.size(i.size.on_disk),
                style.size(i.size.apparent)
            )
        } else {
            String::new()
        }
    );
    if let Some(n) = i.size.file_count {
        println!("Files        {n}");
    }
    println!("\nSafety       {}", style.safety(i.safety));
    for r in &i.reasons {
        println!(
            "             {} {}",
            r.text,
            style.dim(&format!("[{}]", r.code))
        );
    }
    for n in &i.notes {
        println!("Note         {n}");
    }
    if e.actions.is_empty() {
        println!("\nkipple won't act on this item.");
    } else {
        println!("\nWhat kipple would do");
        for (idx, a) in e.actions.iter().enumerate() {
            let mark = if idx == 0 { "→" } else { " " };
            println!(
                "  {mark} {}  {}",
                style.bold(&a.action.to_string()),
                a.description
            );
        }
    }
    if let Some(h) = &i.restore_hint {
        println!("\nTo get it back, run: {h}");
    }
    if i.safety == Safety::Review {
        println!(
            "\n{}",
            style.dim("Review items are only removed when you confirm by typing `reclaim`.")
        );
    }
    Ok(ExitCode::SUCCESS)
}
