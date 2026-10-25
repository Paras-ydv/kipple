//! Planner and executor on real fixtures (SPEC §8, §21.3).

use std::collections::BTreeMap;
use std::path::Path;

use kipple_core::model::*;
use kipple_core::testing::{Fixture, TestHome};
use kipple_core::{Kipple, OpenOptions};

/// Hash of every path, type, mode and content under `root`, plus file
/// mtimes. Folder mtimes are left out: moving a child out and back updates
/// its parent folder's mtime, as any rename does.
fn tree_hash(root: &Path) -> String {
    let mut entries: BTreeMap<String, String> = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let m = p.symlink_metadata().unwrap();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            let mtime = if m.is_dir() { None } else { m.modified().ok() };
            let mut desc = format!("{:?}|{:?}", m.file_type(), mtime);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                desc.push_str(&format!("|{:o}", m.permissions().mode()));
            }
            if m.is_file() {
                desc.push_str(&blake3::hash(&std::fs::read(&p).unwrap()).to_hex());
            } else if m.is_dir() {
                stack.push(p);
            }
            entries.insert(rel, desc);
        }
    }
    blake3::hash(format!("{entries:?}").as_bytes())
        .to_hex()
        .to_string()
}

fn setup(fixtures: &[&str]) -> (TestHome, Kipple) {
    let mut home = TestHome::new().unwrap();
    for f in fixtures {
        Fixture::named(f).unwrap().generate(&mut home).unwrap();
    }
    let k = Kipple::open(OpenOptions::new(
        home.env().clone(),
        kipple_packs::builtin().unwrap(),
    ))
    .unwrap();
    k.scan(ScanRequest::default(), &NoopSink::default())
        .unwrap();
    (home, k)
}

fn safe_plan(k: &Kipple) -> Plan {
    k.plan(PlanRequest {
        filter: ItemFilter {
            safe_only: true,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap()
}

#[test]
fn clean_then_undo_restores_byte_identical_trees() {
    let (home, k) = setup(&["node-modules", "python-caches", "rust-target", "npm-cache"]);
    let code = home.join("code");
    let caches = home.join("caches");
    let before = (tree_hash(&code), tree_hash(&caches));

    let plan = safe_plan(&k);
    assert!(plan.entries.len() >= 4, "{:#?}", plan.entries.len());
    assert!(plan.entries.iter().all(|e| e.item.safety == Safety::Safe));
    let report = k
        .execute(&plan, Confirmation::Yes, &NoopSink::default())
        .unwrap();
    assert_eq!(report.failed, 0, "{report:#?}");
    assert_eq!(report.skipped, 0, "{report:#?}");
    assert!(report.held_in_quarantine > 0);
    assert!(!home.join("code/old-app/node_modules").exists());
    assert!(!home.join("code/tool/target").exists());
    assert!(
        home.join("code/old-app/package.json").exists(),
        "sources untouched"
    );
    assert_ne!(tree_hash(&code), before.0);

    let journal = k.journal();
    assert_eq!(journal.len(), plan.entries.len());
    assert!(journal.iter().all(|j| j.plan_id == plan.id));

    let restored = k.undo().unwrap();
    assert!(restored.skipped.is_empty(), "{restored:#?}");
    assert_eq!(restored.restored.len() as u64, report.done);
    assert_eq!((tree_hash(&code), tree_hash(&caches)), before);
    assert!(k.quarantine().unwrap().is_empty());
}

#[test]
fn review_plans_need_the_word_reclaim() {
    let (_home, k) = setup(&["node-modules"]);
    let plan = k.plan(PlanRequest::default()).unwrap();
    assert!(plan.has_review());
    assert!(
        k.execute(&plan, Confirmation::Yes, &NoopSink::default())
            .is_err()
    );
    assert!(
        k.execute(
            &plan,
            Confirmation::Typed { word: "y".into() },
            &NoopSink::default()
        )
        .is_err()
    );
    let report = k
        .execute(
            &plan,
            Confirmation::Typed {
                word: "reclaim".into(),
            },
            &NoopSink::default(),
        )
        .unwrap();
    assert!(report.done > 0);
}

#[test]
fn keep_items_never_enter_a_plan() {
    let (_home, k) = setup(&["gradle-cache", "pnpm-store"]);
    let plan = k.plan(PlanRequest::default()).unwrap();
    assert!(!plan.entries.is_empty());
    assert!(plan.entries.iter().all(|e| e.item.safety != Safety::Keep));
    assert!(!plan.entries.iter().any(|e| e.item.title.contains("8.10")));
}

#[test]
fn deleted_or_replaced_items_are_skipped() {
    let (home, k) = setup(&["rust-target", "maven-project"]);
    let plan = safe_plan(&k);
    // Delete one item and replace the other with a new folder.
    std::fs::remove_dir_all(home.join("code/tool/target")).unwrap();
    std::fs::rename(
        home.join("code/java-svc/target"),
        home.join("code/java-svc/old"),
    )
    .unwrap();
    home.write("code/java-svc/target/x", "new").unwrap();
    let report = k
        .execute(&plan, Confirmation::Yes, &NoopSink::default())
        .unwrap();
    assert_eq!(report.done, 0, "{report:#?}");
    assert_eq!(report.skipped, 2);
    assert!(home.join("code/java-svc/target/x").exists());
}

#[test]
fn freshly_written_items_are_skipped() {
    let (home, k) = setup(&["rust-target"]);
    let plan = safe_plan(&k);
    home.write("code/tool/target/new-build-output", "x")
        .unwrap();
    let report = k
        .execute(&plan, Confirmation::Yes, &NoopSink::default())
        .unwrap();
    assert_eq!(report.skipped, 1);
    assert!(home.join("code/tool/target").exists());
}

#[test]
fn cancel_stops_before_the_next_entry() {
    let (_home, k) = setup(&["node-modules", "rust-target", "npm-cache", "pip-cache"]);
    let plan = safe_plan(&k);
    let sink = NoopSink::default();
    sink.cancel.cancel();
    let report = k.execute(&plan, Confirmation::Yes, &sink).unwrap();
    assert!(report.cancelled);
    assert_eq!(report.done, 0);
}

#[test]
fn restore_refuses_when_path_is_back_and_purge_frees() {
    let (home, k) = setup(&["rust-target"]);
    let plan = safe_plan(&k);
    k.execute(&plan, Confirmation::Yes, &NoopSink::default())
        .unwrap();
    home.write("code/tool/target/rebuilt", "x").unwrap();
    let undo = k.undo().unwrap();
    assert_eq!(undo.restored.len(), 0);
    assert!(undo.skipped[0].reason.contains("exists again"));
    let purged = k
        .purge(PurgeRequest {
            all: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(purged.purged.len(), 1);
    assert!(k.quarantine().unwrap().is_empty());
}

#[test]
fn saved_plans_round_trip_and_expire_with_new_scans() {
    let (_home, k) = setup(&["rust-target"]);
    let plan = safe_plan(&k);
    k.save_plan(&plan).unwrap();
    assert_eq!(k.load_plan(plan.id.as_str()).unwrap(), plan);
    k.scan(ScanRequest::default(), &NoopSink::default())
        .unwrap();
    assert!(k.load_plan(plan.id.as_str()).is_err());
    assert!(k.load_plan("../etc/passwd").is_err());
}

#[cfg(unix)]
#[test]
fn native_command_runs_and_journals_freed_bytes() {
    let mut home = TestHome::new().unwrap();
    Fixture::named("go-build-cache")
        .unwrap()
        .generate(&mut home)
        .unwrap();
    let cache = home.join("caches/go-build");
    home.mock_command(
        "go",
        &format!(
            "case \"$1 $2\" in\n  'env GOCACHE') echo '{c}' ;;\n  'clean -cache') rm -rf '{c}/00' ;;\n  *) exit 1 ;;\nesac",
            c = cache.display()
        ),
    )
    .unwrap();
    let k = Kipple::open(OpenOptions::new(
        home.env().clone(),
        kipple_packs::builtin().unwrap(),
    ))
    .unwrap();
    k.scan(
        ScanRequest {
            only: vec!["go-build-cache".into()],
            ..Default::default()
        },
        &NoopSink::default(),
    )
    .unwrap();
    let plan = safe_plan(&k);
    assert_eq!(plan.entries[0].action, ActionKind::native("go-clean-cache"));
    assert!(plan.totals.freed_now > 0);
    let report = k
        .execute(&plan, Confirmation::Yes, &NoopSink::default())
        .unwrap();
    assert_eq!(report.done, 1, "{report:#?}");
    assert!(report.freed_now >= 11_000);
    assert!(!cache.join("00").exists());
    assert!(cache.join("README").exists());
    assert_eq!(k.journal()[0].freed, report.freed_now);
}
