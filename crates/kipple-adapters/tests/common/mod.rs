//! Shared helpers for adapter tests. Everything runs in a `TestHome`.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use kipple_core::integration::Adapter;
use kipple_core::model::{Item, NoopSink, ScanRequest};
use kipple_core::platform::{FixedProcesses, ProcessInfo};
use kipple_core::testing::{Fixture, TestHome};
use kipple_core::{Kipple, OpenOptions};
use serde_json::{Value, json};

/// `~/…` with the OS's app settings folder shown as `{config}`, so
/// snapshots are the same on every OS.
pub fn rel(home: &TestHome, p: &Path) -> String {
    if let Ok(r) = p.strip_prefix(home.join(kipple_core::testing::APP_CONFIG_DIR)) {
        return format!("{{config}}/{}", r.display()).replace('\\', "/");
    }
    let s = p
        .strip_prefix(home.path())
        .map(|r| format!("~/{}", r.display()))
        .unwrap_or_else(|_| p.display().to_string());
    s.replace('\\', "/")
}

/// Replaces the home path in any string inside `v`.
fn relativize(home: &TestHome, v: &mut Value) {
    match v {
        Value::String(s) => {
            let h = home.path().to_string_lossy().into_owned();
            if s.contains(&h) {
                *s = s.replace(&h, "~").replace('\\', "/");
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| relativize(home, x)),
        Value::Object(o) => o.values_mut().for_each(|x| relativize(home, x)),
        _ => {}
    }
}

/// Titles ending in `· YYYY-MM-DD` (dates from fixture mtimes, which are
/// relative to today) become `· <date>`.
fn undated(title: &str) -> String {
    match title.rsplit_once(" · ") {
        Some((head, tail))
            if tail.len() == 10 && tail.chars().filter(|c| *c == '-').count() == 2 =>
        {
            format!("{head} · <date>")
        }
        _ => title.to_owned(),
    }
}

/// OS- and time-independent view of an item.
pub fn project(home: &TestHome, item: &Item) -> Value {
    let paths: Vec<String> = item.locator.paths().iter().map(|p| rel(home, p)).collect();
    let mut details = item.details.clone();
    relativize(home, &mut details);
    json!({
        "integration": item.integration,
        "category": item.category,
        "kind": item.kind,
        "title": undated(&item.title),
        "paths": paths,
        "project": item.project.as_ref().map(|p| json!({
            "name": p.name,
            "root": rel(home, &p.root),
            "exists": p.root.exists(),
        })),
        "safety": item.safety,
        "reasons": item.reasons.iter().map(|r| r.code).collect::<Vec<_>>(),
        "actions": item.actions.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "apparent": item.size.apparent,
        "reclaimable": item.size.reclaimable,
        "files": item.size.file_count,
        "last_used_source": item.last_used.map(|l| l.source),
        "notes": item.notes,
        "details": details,
    })
}

/// Every built-in integration, as the CLI registers them.
pub fn integrations() -> Vec<Arc<dyn Adapter>> {
    let mut all = kipple_packs::builtin().unwrap();
    all.extend(kipple_adapters::all());
    all
}

pub fn home_with(fixtures: &[&str]) -> TestHome {
    let mut home = TestHome::new().unwrap();
    for f in fixtures {
        Fixture::named(f).unwrap().generate(&mut home).unwrap();
    }
    home
}

pub fn open(home: &TestHome, adapters: Vec<Arc<dyn Adapter>>, procs: &FixedProcesses) -> Kipple {
    let mut opts = OpenOptions::new(home.env().clone(), adapters);
    opts.processes = Some(Arc::new(procs.clone()));
    Kipple::open(opts).unwrap()
}

pub fn proc(name: &str) -> ProcessInfo {
    ProcessInfo {
        pid: 42,
        name: name.to_owned(),
        args: vec![],
    }
}

/// Scans with only `id` enabled and returns its items, sorted by locator.
pub fn scan_only(k: &Kipple, id: &str) -> Vec<Item> {
    k.scan(
        ScanRequest {
            only: vec![id.to_owned()],
            ..Default::default()
        },
        &NoopSink::default(),
    )
    .unwrap();
    let (_, mut items) = k.all_items().unwrap();
    items.sort_by_key(|i| i.locator.canonical_key());
    items
}

/// Snapshot value for a fixture scanned with every integration registered
/// and only `id` enabled.
pub fn scan_fixture(id: &str) -> Value {
    let home = home_with(&[id]);
    let k = open(&home, integrations(), &FixedProcesses::default());
    let items = scan_only(&k, id);
    Value::Array(items.iter().map(|i| project(&home, i)).collect())
}

/// Modification time and existence of every file under `root`, for
/// asserting that a scan changed nothing.
pub fn tree_state(root: &Path) -> BTreeMap<PathBuf, Option<SystemTime>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(m) = p.symlink_metadata() else {
                continue;
            };
            if m.is_dir() {
                stack.push(p.clone());
                continue;
            }
            out.insert(p, m.modified().ok());
        }
    }
    out
}

pub fn find(items: &[Item], pred: impl Fn(&Item) -> bool) -> &Item {
    items
        .iter()
        .find(|i| pred(i))
        .unwrap_or_else(|| panic!("no matching item in {items:#?}"))
}
