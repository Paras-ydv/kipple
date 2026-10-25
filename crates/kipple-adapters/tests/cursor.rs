//! `cursor`: read-only stats, the guarded VACUUM, the backup copy and
//! workspace data for deleted folders.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kipple_adapters::cursor::Cursor;
use kipple_core::integration::Adapter;
use kipple_core::model::*;
use kipple_core::platform::FixedProcesses;
use kipple_core::testing::{APP_CONFIG_DIR, TestHome};

const ID: &str = "cursor";

fn global(home: &TestHome) -> PathBuf {
    home.join(APP_CONFIG_DIR).join("Cursor/User/globalStorage")
}

fn db(home: &TestHome) -> PathBuf {
    global(home).join("state.vscdb")
}

/// A Cursor adapter that compacts from 1 MB and sees `free` bytes free.
fn adapter(free: Option<u64>) -> Arc<dyn Adapter> {
    fn plenty(_: &Path) -> Option<u64> {
        Some(u64::MAX)
    }
    fn none(_: &Path) -> Option<u64> {
        Some(0)
    }
    fn unknown(_: &Path) -> Option<u64> {
        None
    }
    Arc::new(Cursor {
        min_free_pages: 1_000_000,
        free_space: match free {
            Some(0) => none,
            Some(_) => plenty,
            None => unknown,
        },
    })
}

fn db_item(items: &[Item]) -> Item {
    common::find(items, |i| i.kind == "cursor_state_db").clone()
}

fn reclaim(k: &kipple_core::Kipple, item: &Item) -> ExecReport {
    let plan = k
        .plan(PlanRequest {
            filter: ItemFilter {
                ids: vec![item.id.to_string()],
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    assert_eq!(plan.entries.len(), 1, "{plan:#?}");
    k.execute(
        &plan,
        Confirmation::Typed {
            word: "reclaim".into(),
        },
        &NoopSink::default(),
    )
    .unwrap()
}

fn backups(k: &kipple_core::Kipple) -> Vec<PathBuf> {
    std::fs::read_dir(k.dirs().backups_dir())
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

fn row_count(path: &Path) -> i64 {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.query_row("SELECT count(*) FROM cursorDiskKV", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn scan_opens_the_database_read_only() {
    let home = common::home_with(&[ID]);
    let root = home.join(APP_CONFIG_DIR).join("Cursor");
    let before = common::tree_state(&root);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    assert!(!items.is_empty());
    assert_eq!(
        common::tree_state(&root),
        before,
        "scan changed Cursor's files"
    );
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut p = db(&home).into_os_string();
        p.push(suffix);
        assert!(!PathBuf::from(p).exists(), "{suffix} file appeared");
    }
}

#[test]
fn below_the_threshold_the_database_is_report_only() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let item = db_item(&common::scan_only(&k, ID));
    assert_eq!(item.safety, Safety::Keep);
    assert!(item.actions.is_empty());
    assert!(item.size.reclaimable >= 2_900_000, "{:?}", item.size);
    assert_eq!(item.details["chat_bytes_by_key_prefix"]["bubbleId"], 60_000);
}

#[test]
fn vacuum_backs_up_compacts_and_checks_integrity() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, vec![adapter(Some(1))], &FixedProcesses::default());
    let item = db_item(&common::scan_only(&k, ID));
    assert_eq!(item.safety, Safety::Review);
    assert_eq!(
        item.actions,
        vec![ActionKind::Compact {
            method: "sqlite_vacuum".into()
        }]
    );
    let size_before = std::fs::metadata(db(&home)).unwrap().len();
    let rows_before = row_count(&db(&home));

    let report = reclaim(&k, &item);
    assert_eq!(report.done, 1, "{report:#?}");
    assert!(report.freed_now >= 2_900_000, "{report:#?}");

    let size_after = std::fs::metadata(db(&home)).unwrap().len();
    assert!(size_after < size_before);
    assert_eq!(row_count(&db(&home)), rows_before, "no rows removed");
    let conn = rusqlite::Connection::open(db(&home)).unwrap();
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok");

    let copies = backups(&k);
    assert_eq!(copies.len(), 1, "{copies:?}");
    assert_eq!(std::fs::metadata(&copies[0]).unwrap().len(), size_before);
    assert_eq!(row_count(&copies[0]), rows_before);
}

#[test]
fn vacuum_refuses_while_cursor_runs() {
    let home = common::home_with(&[ID]);
    let procs = FixedProcesses::default();
    let k = common::open(&home, vec![adapter(Some(1))], &procs);
    let item = db_item(&common::scan_only(&k, ID));
    let before = std::fs::read(db(&home)).unwrap();
    procs.set(vec![common::proc("Cursor")]);
    let report = reclaim(&k, &item);
    assert_eq!(report.skipped, 1, "{report:#?}");
    assert_eq!(std::fs::read(db(&home)).unwrap(), before);
    assert!(backups(&k).is_empty());
}

#[test]
fn vacuum_refuses_without_free_space() {
    for free in [Some(0), None] {
        let home = common::home_with(&[ID]);
        let k = common::open(&home, vec![adapter(free)], &FixedProcesses::default());
        let item = db_item(&common::scan_only(&k, ID));
        let before = std::fs::read(db(&home)).unwrap();
        let report = reclaim(&k, &item);
        assert_eq!(report.skipped, 1, "{report:#?}");
        assert_eq!(std::fs::read(db(&home)).unwrap(), before);
        assert!(backups(&k).is_empty());
    }
}

#[test]
fn running_cursor_keeps_everything() {
    let home = common::home_with(&[ID]);
    let procs = FixedProcesses::new(vec![common::proc("Cursor")]);
    let k = common::open(&home, vec![adapter(Some(1))], &procs);
    let items = common::scan_only(&k, ID);
    assert!(items.len() >= 4);
    for i in &items {
        assert_eq!(i.safety, Safety::Keep, "{}", i.title);
        assert_eq!(i.reasons[0].code, ReasonCode::ToolRunning);
    }
}

#[test]
fn backup_copy_quarantine_and_undo() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let backup = common::find(&items, |i| i.kind == "cursor_state_backup");
    assert!(backup.has_reason(ReasonCode::BackupCopy));
    let report = reclaim(&k, backup);
    assert_eq!(report.done, 1, "{report:#?}");
    let path = global(&home).join("state.vscdb.backup");
    assert!(!path.exists());
    assert!(db(&home).exists(), "the live database is untouched");
    k.undo().unwrap();
    assert!(path.exists());
}

#[test]
fn deleted_workspaces_are_orphans() {
    let home = common::home_with(&[ID]);
    let k = common::open(&home, common::integrations(), &FixedProcesses::default());
    let items = common::scan_only(&k, ID);
    let ws: Vec<&Item> = items
        .iter()
        .filter(|i| i.kind == "cursor_workspace_storage")
        .collect();
    assert_eq!(ws.len(), 2, "deleted folder and deleted workspace file");
    let folder = common::find(&items, |i| i.title == "Cursor data for old site");
    let project = folder.project.as_ref().unwrap();
    assert_eq!(project.root, home.join("code/old site"));
    assert!(folder.has_reason(ReasonCode::OrphanedProject));
}
