//! `kipple ai`: AI history per tool, per project and per session
//! (SPEC §11.2). Uses only the public `Kipple` API.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use kipple_core::Kipple;
use kipple_core::model::{Category, Item, ItemQuery, ItemSort, ReasonCode, Safety};
use serde::Serialize;

use crate::args::{AiArgs, AiSessionsArgs, AiView, SortKey};
use crate::context::Context;
use crate::output::{Align, Table, age_of, short_path, truncate};

#[derive(Debug, Default, Serialize)]
struct ToolRow {
    integration: String,
    name: String,
    items: u64,
    reclaimable: u64,
    review: u64,
    keep_items: u64,
}

#[derive(Debug, Serialize)]
struct ProjectRow {
    name: String,
    root: Option<PathBuf>,
    deleted: bool,
    items: u64,
    reclaimable: u64,
    tools: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Overview {
    tools: Vec<ToolRow>,
    projects: Vec<ProjectRow>,
}

/// AI history items of the latest scan, Keep included.
fn ai_items(ctx: &Context, k: &Kipple, tool: Option<&str>, sort: SortKey) -> Result<Vec<Item>> {
    let mut filter = ctx.filter()?;
    filter.only = match tool {
        Some(t) => vec![t.to_owned()],
        None if !filter.only.is_empty() => filter.only,
        None => vec![Category::AiHistory.id().to_owned()],
    };
    let items = k.items(ItemQuery {
        filter,
        sort: match sort {
            SortKey::Size => ItemSort::Size,
            SortKey::Age => ItemSort::Age,
            SortKey::Name => ItemSort::Name,
        },
        limit: None,
        include_keep: true,
    })?;
    Ok(items
        .into_iter()
        .filter(|i| i.category == Category::AiHistory)
        .collect())
}

fn tool_names(k: &Kipple) -> BTreeMap<String, String> {
    k.adapters()
        .iter()
        .map(|a| {
            let info = a.info();
            (info.id.to_string(), info.name)
        })
        .collect()
}

fn countable(i: &Item) -> u64 {
    if i.safety == Safety::Keep {
        0
    } else {
        i.size.reclaimable
    }
}

fn by_tool(items: &[Item], names: &BTreeMap<String, String>) -> Vec<ToolRow> {
    let mut rows: BTreeMap<String, ToolRow> = BTreeMap::new();
    for i in items {
        let id = i.integration.to_string();
        let row = rows.entry(id.clone()).or_insert_with(|| ToolRow {
            name: names.get(&id).cloned().unwrap_or_else(|| id.clone()),
            integration: id,
            ..Default::default()
        });
        row.items += 1;
        row.reclaimable += countable(i);
        if i.safety == Safety::Review {
            row.review += i.size.reclaimable;
        }
        if i.safety == Safety::Keep {
            row.keep_items += 1;
        }
    }
    let mut out: Vec<ToolRow> = rows.into_values().collect();
    out.sort_by(|a, b| b.reclaimable.cmp(&a.reclaimable).then(a.name.cmp(&b.name)));
    out
}

fn by_project(items: &[Item], names: &BTreeMap<String, String>) -> Vec<ProjectRow> {
    let mut rows: BTreeMap<Option<PathBuf>, ProjectRow> = BTreeMap::new();
    for i in items {
        let key = i.project.as_ref().map(|p| p.root.clone());
        let row = rows.entry(key.clone()).or_insert_with(|| ProjectRow {
            name: i
                .project
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "No project".to_owned()),
            root: key,
            deleted: false,
            items: 0,
            reclaimable: 0,
            tools: Vec::new(),
        });
        row.items += 1;
        row.reclaimable += countable(i);
        row.deleted |= i.project.is_some() && i.has_reason(ReasonCode::OrphanedProject);
        let tool = names
            .get(i.integration.as_str())
            .cloned()
            .unwrap_or_else(|| i.integration.to_string());
        if !row.tools.contains(&tool) {
            row.tools.push(tool);
        }
    }
    let mut out: Vec<ProjectRow> = rows.into_values().collect();
    // Items without a project go last.
    out.sort_by(|a, b| {
        a.root
            .is_none()
            .cmp(&b.root.is_none())
            .then(b.reclaimable.cmp(&a.reclaimable))
            .then(a.name.cmp(&b.name))
    });
    out
}

fn project_table(ctx: &Context, rows: &[ProjectRow]) -> String {
    let style = ctx.style();
    let home = ctx.home();
    let mut t = Table::new(&[
        ("Project", Align::Left),
        ("Path", Align::Left),
        ("Items", Align::Right),
        ("Reclaimable", Align::Right),
        ("Tools", Align::Left),
    ]);
    for p in rows {
        let name = if p.deleted {
            format!("{} (deleted)", p.name)
        } else {
            p.name.clone()
        };
        t.row(vec![
            truncate(&name, 32),
            style.dim(
                &p.root
                    .as_deref()
                    .map(|r| short_path(r, home.as_deref()))
                    .unwrap_or_else(|| "—".to_owned()),
            ),
            p.items.to_string(),
            style.size_or_dash(p.reclaimable),
            style.dim(&p.tools.join(", ")),
        ]);
    }
    t.render(&style)
}

pub fn run(ctx: &mut Context, args: &AiArgs) -> Result<ExitCode> {
    match &args.view {
        None => overview(ctx),
        Some(AiView::Projects) => projects(ctx),
        Some(AiView::Sessions(a)) => sessions(ctx, a),
    }
}

fn overview(ctx: &mut Context) -> Result<ExitCode> {
    let k = ctx.open()?;
    let items = ai_items(ctx, &k, None, SortKey::Size)?;
    let names = tool_names(&k);
    let data = Overview {
        tools: by_tool(&items, &names),
        projects: by_project(&items, &names),
    };
    if ctx.global.json {
        ctx.print_json(&data)?;
        return Ok(ExitCode::SUCCESS);
    }
    if items.is_empty() {
        println!("No AI history found.");
        return Ok(ExitCode::SUCCESS);
    }
    let style = ctx.style();
    let mut t = Table::new(&[
        ("Tool", Align::Left),
        ("Items", Align::Right),
        ("Reclaimable", Align::Right),
        ("Review", Align::Right),
        ("Kept", Align::Right),
    ]);
    for r in &data.tools {
        t.row(vec![
            r.name.clone(),
            r.items.to_string(),
            style.size_or_dash(r.reclaimable),
            style.size_or_dash(r.review),
            r.keep_items.to_string(),
        ]);
    }
    print!("{}", t.render(&style));
    println!();
    let top: Vec<ProjectRow> = data.projects.into_iter().take(5).collect();
    print!("{}", project_table(ctx, &top));
    println!(
        "\n{}",
        style.dim(
            "AI history is never marked safe: chats can't be recovered and may hold pasted secrets."
        )
    );
    println!(
        "{}",
        style.dim("Next: kipple ai sessions --tool <id> --older-than 30d")
    );
    Ok(ExitCode::SUCCESS)
}

fn projects(ctx: &mut Context) -> Result<ExitCode> {
    let k = ctx.open()?;
    let items = ai_items(ctx, &k, None, SortKey::Size)?;
    let rows = by_project(&items, &tool_names(&k));
    if ctx.global.json {
        ctx.print_json(&rows)?;
        return Ok(ExitCode::SUCCESS);
    }
    if rows.is_empty() {
        println!("No AI history found.");
        return Ok(ExitCode::SUCCESS);
    }
    print!("{}", project_table(ctx, &rows));
    let deleted = rows.iter().filter(|r| r.deleted).count();
    if deleted > 0 {
        println!(
            "\n{}",
            ctx.style().dim(&format!(
                "{deleted} of these projects no longer exist on disk."
            ))
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn sessions(ctx: &mut Context, args: &AiSessionsArgs) -> Result<ExitCode> {
    let k = ctx.open()?;
    let mut items = ai_items(ctx, &k, args.tool.as_deref(), args.sort)?;
    if !args.all {
        items.retain(|i| i.safety != Safety::Keep);
    }
    if let Some(n) = args.limit {
        items.truncate(n);
    }
    if ctx.global.json {
        ctx.print_json(&items)?;
        return Ok(ExitCode::SUCCESS);
    }
    if items.is_empty() {
        println!("No sessions match.");
        return Ok(ExitCode::SUCCESS);
    }
    let names = tool_names(&k);
    let style = ctx.style();
    let mut t = Table::new(&[
        ("Id", Align::Left),
        ("Size", Align::Right),
        ("Safety", Align::Left),
        ("Last used", Align::Left),
        ("Tool", Align::Left),
        ("Title", Align::Left),
        ("Project", Align::Left),
    ]);
    let mut total = 0;
    for i in &items {
        total += countable(i);
        let project = match &i.project {
            Some(p) if i.has_reason(ReasonCode::OrphanedProject) => format!("{} (deleted)", p.name),
            Some(p) => p.name.clone(),
            None => "—".to_owned(),
        };
        t.row(vec![
            i.id.as_str()[..8].to_owned(),
            style.size(i.size.reclaimable),
            style.safety(i.safety),
            age_of(i.last_used.as_ref()),
            names
                .get(i.integration.as_str())
                .cloned()
                .unwrap_or_else(|| i.integration.to_string()),
            // Titles are already secret-masked and at most 60 characters.
            truncate(&i.title, 60),
            style.dim(&truncate(&project, 28)),
        ]);
    }
    print!("{}", t.render(&style));
    println!("\n{} items, {}", items.len(), style.size(total));
    Ok(ExitCode::SUCCESS)
}
