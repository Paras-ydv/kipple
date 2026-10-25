//! `antigravity`: migrated copies, in-use and quarantine.

mod common;

use kipple_core::model::*;
use kipple_core::platform::FixedProcesses;
use kipple_core::testing::APP_CONFIG_DIR;

const ID: &str = "antigravity";
const A: &str = "aaaa1111-0000-4000-8000-000000000001";

#[test]
fn scan_changes_nothing() {
    let home = common::home_with(&[ID]);
    let before = (
        common::tree_state(&home.join(".gemini")),
        common::tree_state(&home.join(APP_CONFIG_DIR)),
    );
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    common::scan_only(&k, ID);
    let after = (
        common::tree_state(&home.join(".gemini")),
        common::tree_state(&home.join(APP_CONFIG_DIR)),
    );
    assert_eq!(after, before);
}

#[test]
fn every_item_is_review_with_quarantine_only() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    assert_eq!(items.len(), 8);
    for i in &items {
        assert_eq!(i.safety, Safety::Review, "{}", i.title);
        assert_eq!(i.actions, vec![ActionKind::Quarantine]);
    }
    let mut migrated: Vec<String> = items
        .iter()
        .filter(|i| i.has_reason(ReasonCode::MigratedCopy))
        .map(|i| i.details["data_root"].as_str().unwrap().to_owned())
        .collect();
    migrated.sort();
    assert_eq!(
        migrated,
        vec!["~/.gemini/antigravity", "~/.gemini/antigravity-backup"]
    );
}

#[test]
fn running_antigravity_keeps_everything() {
    for name in [
        "Antigravity",
        "Antigravity IDE",
        "Antigravity Helper (Renderer)",
    ] {
        let home = common::home_with(&[ID]);
        let procs = FixedProcesses::new(vec![common::proc(name)]);
        let k = common::open(&home, common::integrations(), &procs);
        for i in common::scan_only(&k, ID) {
            assert_eq!(i.safety, Safety::Keep, "{name}: {}", i.title);
            assert_eq!(i.reasons[0].code, ReasonCode::ToolRunning);
        }
    }
}

#[test]
fn quarantine_moves_conversation_and_brain_together() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let copy = common::find(&items, |i| {
        i.has_reason(ReasonCode::MigratedCopy) && i.details["conversation_id"] == A
    });
    let pb = home.join(format!(".gemini/antigravity/conversations/{A}.pb"));
    let brain = home.join(format!(".gemini/antigravity/brain/{A}"));
    let plan = k
        .plan(PlanRequest {
            filter: ItemFilter {
                ids: vec![copy.id.to_string()],
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
    assert!(!pb.exists() && !brain.exists());
    assert!(
        home.join(format!(".gemini/antigravity-ide/conversations/{A}.pb"))
            .exists(),
        "the current copy is untouched"
    );
    k.undo().unwrap();
    assert!(pb.exists() && brain.join("task.md").exists());
}
