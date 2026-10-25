//! `codex` behaviour and the native delete action, with a mock `codex`.

mod common;

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use kipple_core::integration::Adapter;
use kipple_core::model::*;
use kipple_core::platform::FixedProcesses;

const ID: &str = "codex";
const S1: &str = "019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a01";

fn rollout1(home: &kipple_core::testing::TestHome) -> std::path::PathBuf {
    home.join(format!(
        ".codex/sessions/2026/01/15/rollout-2026-01-15T10-00-00-{S1}.jsonl"
    ))
}

/// A `codex` that supports `delete --force`, records its arguments and
/// removes the matching rollout like the real command would.
#[cfg(unix)]
fn mock_codex(home: &kipple_core::testing::TestHome, removes: bool) {
    let remove = if removes {
        "find \"$CODEX_HOME\" -name \"*$2*\" -type f -exec rm {} \\;"
    } else {
        ":"
    };
    home.mock_command(
        "codex",
        &format!(
            r#"if [ "$1" = "--version" ]; then echo "codex-cli 0.164.0"; exit 0; fi
if [ "$1" = "delete" ] && [ "$2" = "--help" ]; then
  echo "Permanently delete a saved session by id or session name."
  echo "Usage: codex delete [OPTIONS] <SESSION>"
  echo "      --force  Delete without prompting. SESSION must be a UUID."
  exit 0
fi
if [ "$1" = "delete" ]; then
  echo "$@" >> "$HOME/codex-calls.txt"
  {remove}
  exit 0
fi
exit 2"#
        ),
    )
    .unwrap();
}

#[test]
fn without_codex_sessions_are_report_only() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    assert_eq!(items.len(), 6);
    for i in &items {
        assert_eq!(i.safety, Safety::Keep, "{}", i.title);
        assert!(i.actions.is_empty());
    }
    let s1 = common::find(&items, |i| {
        i.locator.primary_path() == Some(&rollout1(&home))
    });
    assert_eq!(s1.reasons[0].code, ReasonCode::NativeToolMissing);
}

#[test]
fn scan_opens_the_state_database_read_only() {
    let home = common::home_with(&[ID]);
    let before = common::tree_state(&home.join(".codex"));
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    common::scan_only(&k, ID);
    let after = common::tree_state(&home.join(".codex"));
    assert_eq!(after, before, "scan changed files under ~/.codex");
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(!home.join(format!(".codex/state_5.sqlite{suffix}")).exists());
    }
}

#[cfg(unix)]
#[test]
fn with_codex_sessions_are_reviewable_and_never_quarantined() {
    let home = common::home_with(&[ID]);
    mock_codex(&home, true);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let s1 = common::find(&items, |i| {
        i.locator.primary_path() == Some(&rollout1(&home))
    });
    assert_eq!(s1.safety, Safety::Review);
    assert_eq!(s1.title, "Payments client refactor");
    assert_eq!(s1.actions, vec![ActionKind::native("codex.delete")]);
    for i in &items {
        assert!(
            !i.actions
                .iter()
                .any(|a| matches!(a, ActionKind::Quarantine | ActionKind::Trash)),
            "Codex rollouts must never be moved by kipple"
        );
    }
    let secret = common::find(&items, |i| i.title.starts_with("Rotate token"));
    assert_eq!(secret.title, "Rotate token ghp_**** in CI");
    let unknown: Vec<&Item> = items
        .iter()
        .filter(|i| i.has_reason(ReasonCode::UnknownFormat))
        .collect();
    assert_eq!(unknown.len(), 2, "unknown zst and unknown jsonl");
}

#[cfg(unix)]
#[test]
fn delete_runs_codex_delete_with_force() {
    let home = common::home_with(&[ID]);
    mock_codex(&home, true);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let s1 = common::find(&items, |i| {
        i.locator.primary_path() == Some(&rollout1(&home))
    });
    let plan = k
        .plan(PlanRequest {
            filter: ItemFilter {
                ids: vec![s1.id.to_string()],
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let report = k
        .execute(
            &plan,
            Confirmation::Typed {
                word: "reclaim".into(),
            },
            &NoopSink::default(),
        )
        .unwrap();
    assert_eq!(report.done, 1, "{report:#?}");
    assert!(report.freed_now > 0);
    let calls = std::fs::read_to_string(home.join("codex-calls.txt")).unwrap();
    assert_eq!(calls.trim(), format!("delete {S1} --force"));
    assert!(!rollout1(&home).exists());
    assert!(k.quarantine().unwrap().is_empty(), "nothing quarantined");
}

#[cfg(unix)]
#[test]
fn delete_that_leaves_the_file_is_reported_as_failed() {
    let home = common::home_with(&[ID]);
    mock_codex(&home, false);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let s1 = common::find(&items, |i| {
        i.locator.primary_path() == Some(&rollout1(&home))
    });
    let plan = k
        .plan(PlanRequest {
            filter: ItemFilter {
                ids: vec![s1.id.to_string()],
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let report = k
        .execute(
            &plan,
            Confirmation::Typed {
                word: "reclaim".into(),
            },
            &NoopSink::default(),
        )
        .unwrap();
    assert_eq!(report.failed, 1, "{report:#?}");
    assert!(
        rollout1(&home).exists(),
        "kipple never removes rollouts itself"
    );
}

#[cfg(unix)]
#[test]
fn old_codex_without_delete_is_report_only() {
    let home = common::home_with(&[ID]);
    home.mock_command(
        "codex",
        "echo \"error: unrecognized subcommand 'delete'\" >&2; exit 2",
    )
    .unwrap();
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let s1 = common::find(&items, |i| {
        i.locator.primary_path() == Some(&rollout1(&home))
    });
    assert_eq!(s1.safety, Safety::Keep);
    assert_eq!(s1.reasons[0].code, ReasonCode::NativeToolMissing);
}

#[cfg(unix)]
#[test]
fn running_codex_keeps_everything() {
    let home = common::home_with(&[ID]);
    mock_codex(&home, true);
    let procs = FixedProcesses::new(vec![common::proc("Codex")]);
    let k = common::open(&home, common::integrations(), &procs);
    let items = common::scan_only(&k, ID);
    for i in &items {
        assert_eq!(i.safety, Safety::Keep);
    }
    let s1 = common::find(&items, |i| {
        i.locator.primary_path() == Some(&rollout1(&home))
    });
    assert_eq!(s1.reasons[0].code, ReasonCode::ToolRunning);
}

#[test]
fn growing_rollouts_are_active_writers() {
    let home = common::home_with(&[ID]);
    let path = rollout1(&home);
    // Touch it now so it qualifies for the growth check.
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    let adapter: Arc<dyn Adapter> = Arc::new(kipple_adapters::codex::Codex::new(
        Duration::from_millis(400),
    ));
    let k = common::open(&home, vec![adapter], &FixedProcesses::default());
    let writer = {
        let path = path.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            f.write_all(b"{\"timestamp\":\"x\",\"type\":\"event_msg\"}\n")
                .unwrap();
        })
    };
    let items = common::scan_only(&k, ID);
    writer.join().unwrap();
    let s1 = common::find(&items, |i| i.locator.primary_path() == Some(&path));
    assert_eq!(s1.safety, Safety::Keep);
    assert!(s1.has_reason(ReasonCode::ActiveWriter), "{:?}", s1.reasons);
}
