//! `claude-code` behaviour and actions on the generated fixture.

mod common;

use std::time::{Duration, SystemTime};

use kipple_core::model::*;
use kipple_core::platform::FixedProcesses;

const ID: &str = "claude-code";
const WEBAPP: &str = ".claude/projects/-home-code-webapp";
const MAIN: &str = "0b5a7c1e-1111-4222-8333-944455556666";

fn set_mtime(path: &std::path::Path, ago: Duration) {
    let t = filetime::FileTime::from_system_time(SystemTime::now() - ago);
    filetime::set_file_mtime(path, t).unwrap();
}

#[test]
fn scan_changes_nothing() {
    let home = common::home_with(&[ID]);
    let before = common::tree_state(&home.join(".claude"));
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    common::scan_only(&k, ID);
    assert_eq!(common::tree_state(&home.join(".claude")), before);
}

#[test]
fn never_offers_config_memory_or_credentials() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let root = home.join(".claude");
    for item in &items {
        for p in item.locator.paths() {
            let rel = p.strip_prefix(&root).unwrap();
            let first = rel.components().next().unwrap().as_os_str();
            assert!(
                !kipple_adapters::claude_code::NEVER_TOUCH
                    .iter()
                    .any(|n| first == *n),
                "{} must never be offered",
                rel.display()
            );
            assert!(
                !rel.components().any(|c| c.as_os_str() == "memory"),
                "auto memory offered: {}",
                rel.display()
            );
        }
    }
}

#[test]
fn sessions_belong_to_projects_and_flag_deleted_ones() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    common::scan_only(&k, ID);
    let orphaned = k
        .projects(ProjectQuery {
            orphaned: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(orphaned.len(), 1);
    assert_eq!(orphaned[0].root, home.join("code/deleted-app"));
    assert!(!orphaned[0].exists);
}

#[test]
fn sessions_changed_today_are_kept_while_claude_runs() {
    let home = common::home_with(&[ID]);
    let recent = home
        .join(WEBAPP)
        .join("2c3d4e5f-aaaa-4bbb-8ccc-ddddeeeeffff.jsonl");
    set_mtime(&recent, Duration::from_secs(2 * 3600));
    let title_safety = |procs: &FixedProcesses| {
        let k = common::open(&home, common::integrations(), procs);
        let items = common::scan_only(&k, ID);
        let recent = common::find(&items, |i| i.title == "Add a dark mode toggle").clone();
        let old = common::find(&items, |i| i.title.starts_with("Write a README")).clone();
        (recent, old)
    };

    let (recent, old) = title_safety(&FixedProcesses::default());
    assert_eq!(recent.safety, Safety::Review);
    assert_eq!(old.safety, Safety::Review);

    let (recent, old) = title_safety(&FixedProcesses::new(vec![common::proc("claude")]));
    assert_eq!(recent.safety, Safety::Keep);
    assert_eq!(recent.reasons[0].code, ReasonCode::ToolRunning);
    assert!(recent.actions.is_empty());
    assert_eq!(old.safety, Safety::Review, "older sessions are unaffected");
}

#[test]
fn sessions_written_in_the_last_minutes_are_kept() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let live = common::find(&items, |i| i.title == "Still running");
    assert_eq!(live.safety, Safety::Keep);
    assert_eq!(live.reasons[0].code, ReasonCode::RecentlyWritten);
}

#[test]
fn quarantine_moves_whole_transcripts_and_undo_restores_them() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let main = common::find(&items, |i| i.title.starts_with("Deploy the staging"));
    let transcript = home.join(WEBAPP).join(format!("{MAIN}.jsonl"));
    let companion = home.join(WEBAPP).join(MAIN);
    let original = std::fs::read(&transcript).unwrap();
    let before = common::tree_state(&home.join(".claude"));

    let plan = k
        .plan(PlanRequest {
            filter: ItemFilter {
                ids: vec![main.id.to_string()],
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].action, ActionKind::Quarantine);
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
    assert!(!transcript.exists());
    assert!(!companion.exists());
    assert!(home.join(WEBAPP).join("memory/MEMORY.md").exists());
    assert!(home.join(".claude/settings.json").exists());

    let restored = k.undo().unwrap();
    assert_eq!(restored.restored.len(), 1, "{restored:#?}");
    assert_eq!(std::fs::read(&transcript).unwrap(), original);
    assert!(companion.join("subagents/agent-a1.jsonl").exists());
    assert_eq!(common::tree_state(&home.join(".claude")), before);
}
