//! `kipple ai` views on the AI history fixtures.

use assert_cmd::Command;
use kipple_core::testing::{Fixture, TestHome};

fn kipple(home: &TestHome) -> Command {
    let mut cmd = Command::cargo_bin("kipple").unwrap();
    cmd.env_clear().envs(home.env_pairs());
    cmd
}

fn scanned() -> TestHome {
    let mut home = TestHome::new().unwrap();
    // Each fixture re-ages the whole home, so the one with fresh files goes last.
    for f in ["codex", "claude-code"] {
        Fixture::named(f).unwrap().generate(&mut home).unwrap();
    }
    kipple(&home).arg("scan").assert().success();
    home
}

fn stdout(home: &TestHome, args: &[&str]) -> String {
    let out = kipple(home).args(args).assert().success();
    String::from_utf8(out.get_output().stdout.clone()).unwrap()
}

#[test]
fn overview_lists_tools_and_projects() {
    let home = scanned();
    let out = stdout(&home, &["ai"]);
    assert!(out.contains("Claude Code"), "{out}");
    assert!(out.contains("Codex"), "{out}");
    assert!(out.contains("deleted-app (deleted)"), "{out}");
    assert!(out.contains("never marked safe"), "{out}");
}

#[test]
fn projects_view_counts_deleted_projects() {
    let home = scanned();
    let out = stdout(&home, &["ai", "projects"]);
    assert!(out.contains("webapp"), "{out}");
    assert!(out.contains("no longer exist on disk"), "{out}");
}

#[test]
fn sessions_filter_by_tool_and_age_and_mask_secrets() {
    let home = scanned();
    let out = stdout(
        &home,
        &[
            "ai",
            "sessions",
            "--tool",
            "claude-code",
            "--older-than",
            "30d",
        ],
    );
    assert!(out.contains("AKIA****"), "{out}");
    assert!(
        !out.contains("AKIAIOSFODNN7EXAMPLE"),
        "secret printed: {out}"
    );
    assert!(!out.contains("Codex"), "{out}");
    // The session written just now is kept and too new.
    assert!(!out.contains("Still running"), "{out}");
    assert!(out.contains("review"), "{out}");
}

#[test]
fn sessions_json_has_schema_version() {
    let home = scanned();
    let out = stdout(&home, &["ai", "sessions", "--all", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["schema_version"], 1);
    let items = v["data"].as_array().unwrap();
    assert!(items.iter().all(|i| i["category"] == "ai-history"));
    assert!(items.iter().any(|i| i["integration"] == "codex"));
    let json = serde_json::to_string(&v).unwrap();
    assert!(!json.contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn unknown_tool_is_an_error() {
    let home = scanned();
    kipple(&home)
        .args(["ai", "sessions", "--tool", "nope"])
        .assert()
        .failure();
}
