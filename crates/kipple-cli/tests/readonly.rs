//! Read-only commands never change anything outside kipple's data dir
//! (SPEC §21.5), and `--json` output matches the published schema (§21.6).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use assert_cmd::Command;
use kipple_core::testing::{Fixture, TestHome, fixtures_dir};
use serde_json::Value;

fn home_with_all_fixtures() -> TestHome {
    let mut home = TestHome::new().unwrap();
    let mut ids: Vec<String> = std::fs::read_dir(fixtures_dir())
        .unwrap()
        .flatten()
        .filter(|e| e.path().join("fixture.toml").is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    for id in ids {
        Fixture::named(&id).unwrap().generate(&mut home).unwrap();
    }
    home
}

fn kipple(home: &TestHome) -> Command {
    let mut cmd = Command::cargo_bin("kipple").unwrap();
    cmd.env_clear().envs(home.env_pairs());
    cmd
}

fn json(home: &TestHome, args: &[&str]) -> Value {
    let out = kipple(home).args(args).arg("--json").assert().success();
    let v: Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["schema_version"], 1, "{args:?}");
    v["data"].clone()
}

/// Every path under `root` (except `skip`) with its mtime.
fn mtimes(root: &Path, skip: &[PathBuf]) -> BTreeMap<PathBuf, SystemTime> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if skip.iter().any(|s| p.starts_with(s)) {
                continue;
            }
            let m = p.symlink_metadata().unwrap();
            out.insert(p.clone(), m.modified().unwrap());
            if m.is_dir() {
                stack.push(p);
            }
        }
    }
    out
}

#[test]
fn read_only_commands_touch_nothing() {
    let home = home_with_all_fixtures();
    let data = home.join(".local/share/kipple");
    let skip = vec![data.clone(), home.join(".config/kipple"), home.join("tmp")];
    let before = mtimes(home.path(), &skip);

    kipple(&home).arg("scan").assert().success();
    let items = json(&home, &["items", "--all"]);
    let first = items[0]["id"].as_str().unwrap().to_owned();
    for args in [
        vec!["report"],
        vec!["report", "--by", "project"],
        vec!["report", "--by", "integration"],
        vec!["projects"],
        vec!["projects", "--orphaned"],
        vec!["items"],
        vec!["items", "--all", "--sort", "age"],
        vec!["explain", first.as_str()],
        vec!["config", "show"],
        vec!["config", "path"],
        vec!["config", "validate"],
        vec!["packs", "list"],
        vec!["packs", "test", "npm-cache"],
        vec!["scan", "--json"],
    ] {
        kipple(&home).args(&args).assert().success();
    }

    let after = mtimes(home.path(), &skip);
    // Creating kipple's own data dir may create its missing parents.
    let appeared: Vec<_> = after
        .keys()
        .filter(|p| !before.contains_key(*p) && !data.starts_with(p))
        .collect();
    let vanished: Vec<_> = before.keys().filter(|p| !after.contains_key(*p)).collect();
    assert!(
        appeared.is_empty() && vanished.is_empty(),
        "appeared {appeared:?}, vanished {vanished:?}"
    );
    for (path, t) in before.iter().filter(|(p, _)| !data.starts_with(p)) {
        assert_eq!(after.get(path), Some(t), "{} changed", path.display());
    }
    assert!(data.join("index.sqlite").is_file());
}

fn validator(name: &str) -> jsonschema::Validator {
    let path = fixtures_dir()
        .join("../docs/schema")
        .join(format!("{name}.json"));
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    jsonschema::validator_for(&schema).unwrap()
}

fn assert_valid(name: &str, v: &Value) {
    let validator = validator(name);
    let errors: Vec<String> = validator.iter_errors(v).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "{name}: {errors:#?}");
}

#[test]
fn json_output_matches_schema() {
    let home = home_with_all_fixtures();
    assert_valid("scan-result", &json(&home, &["scan"]));
    assert_valid("report", &json(&home, &["report"]));
    let items = json(&home, &["items", "--all"]);
    assert!(items.as_array().unwrap().len() > 20);
    for item in items.as_array().unwrap() {
        assert_valid("item", item);
    }
    for p in json(&home, &["projects"]).as_array().unwrap() {
        assert_valid("project", p);
    }
    let id = items[0]["id"].as_str().unwrap();
    assert_valid("explanation", &json(&home, &["explain", id]));
}

#[test]
fn every_item_has_a_reason_and_keep_has_no_actions() {
    let home = home_with_all_fixtures();
    kipple(&home).arg("scan").assert().success();
    for item in json(&home, &["items", "--all"]).as_array().unwrap() {
        assert!(!item["reasons"].as_array().unwrap().is_empty(), "{item}");
        if item["safety"] == "keep" {
            assert!(item["actions"].as_array().unwrap().is_empty(), "{item}");
        }
    }
}

#[test]
fn report_without_scan_explains_what_to_do() {
    let home = TestHome::new().unwrap();
    kipple(&home)
        .arg("report")
        .assert()
        .code(1)
        .stderr(predicates::str::contains("run `kipple scan` first"));
}

#[test]
fn unknown_only_selector_is_an_error() {
    let home = TestHome::new().unwrap();
    kipple(&home)
        .args(["scan", "--only", "npm-cahce"])
        .assert()
        .code(1)
        .stderr(predicates::str::contains(
            "unknown integration or category `npm-cahce`",
        ));
}
