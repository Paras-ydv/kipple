//! Golden tests: scanning each fixture with only its pack enabled produces a
//! stable list of items (SPEC §18).

use kipple_core::model::{Item, NoopSink, ScanRequest};
use kipple_core::testing::{Fixture, TestHome};
use kipple_core::{Kipple, OpenOptions};
use serde_json::{Value, json};

fn rel(home: &TestHome, p: &std::path::Path) -> String {
    let s = p
        .strip_prefix(home.path())
        .map(|r| format!("~/{}", r.display()))
        .unwrap_or_else(|_| p.display().to_string());
    s.replace('\\', "/")
}

/// OS- and filesystem-independent view of an item.
fn project(home: &TestHome, item: &Item) -> Value {
    let paths: Vec<String> = item.locator.paths().iter().map(|p| rel(home, p)).collect();
    let mut details = item.details.clone();
    if let Some(sub) = details.get_mut("subproject") {
        if let Some(s) = sub.as_str() {
            *sub = json!(rel(home, std::path::Path::new(s)));
        }
    }
    json!({
        "integration": item.integration,
        "category": item.category,
        "kind": item.kind,
        "title": item.title,
        "paths": paths,
        "project": item.project.as_ref().map(|p| p.name.clone()),
        "safety": item.safety,
        "reasons": item.reasons.iter().map(|r| r.code).collect::<Vec<_>>(),
        "actions": item.actions.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "apparent": item.size.apparent,
        "files": item.size.file_count,
        "shared_hardlinks": item.size.reclaimable < item.size.on_disk,
        "notes": item.notes.len(),
        "restore_hint": item.restore_hint,
        "details": details,
    })
}

fn scan_fixture(id: &str) -> Value {
    let mut home = TestHome::new().unwrap();
    Fixture::named(id).unwrap().generate(&mut home).unwrap();
    let k = Kipple::open(OpenOptions::new(
        home.env().clone(),
        kipple_packs::builtin().unwrap(),
    ))
    .unwrap();
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
    Value::Array(items.iter().map(|i| project(&home, i)).collect())
}

macro_rules! fixture_tests {
    ($($name:ident => $id:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                insta::assert_json_snapshot!($id, scan_fixture($id));
            }
        )*
    };
}

fixture_tests! {
    node_modules => "node-modules",
    js_build => "js-build",
    python_venv => "python-venv",
    python_caches => "python-caches",
    rust_target => "rust-target",
    gradle_project => "gradle-project",
    maven_project => "maven-project",
    xcode_project => "xcode-project",
    cachedir_tag => "cachedir-tag",
    npm_cache => "npm-cache",
    pnpm_store => "pnpm-store",
    yarn_cache => "yarn-cache",
    bun_cache => "bun-cache",
    pip_cache => "pip-cache",
    uv_cache => "uv-cache",
    cargo_registry => "cargo-registry",
    go_build_cache => "go-build-cache",
    go_mod_cache => "go-mod-cache",
    gradle_cache => "gradle-cache",
    puppeteer => "puppeteer",
}

#[cfg(unix)]
#[test]
fn homebrew_cache() {
    insta::assert_json_snapshot!("homebrew-cache", scan_fixture("homebrew-cache"));
}

#[test]
fn every_pack_has_a_fixture() {
    for pack in kipple_packs::builtin_packs().unwrap() {
        let path = kipple_core::testing::fixtures_dir()
            .join(&pack.id)
            .join("fixture.toml");
        assert!(path.is_file(), "missing fixture for {}", pack.id);
    }
}
