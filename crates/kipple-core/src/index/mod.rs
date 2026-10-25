//! SQLite index of scans, items, projects and size history (SPEC §10.2).

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::model::{
    HistoryRequest, Item, ProjectSummary, ScanId, ScanResult, ScanStatus, SizePoint, SizeScope,
    Timestamp,
};

/// Scans whose items are kept; older scans keep only their size points.
pub const KEEP_SCANS: usize = 10;

const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_init.sql")];

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("index database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("index data is corrupt: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot create {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "the index was written by a newer kipple (schema {found}, this build knows {known}); update kipple"
    )]
    TooNew { found: u32, known: u32 },
}

pub type Result<T> = std::result::Result<T, IndexError>;

pub struct Index {
    conn: Connection,
}

fn ts_secs(ts: Timestamp) -> i64 {
    ts.as_second()
}

fn from_secs(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).unwrap_or(Timestamp::UNIX_EPOCH)
}

impl Index {
    /// Opens (creating if needed) the index at `path` and migrates it.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| IndexError::CreateDir {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Self::with_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::with_connection(Connection::open_in_memory()?)
    }

    fn with_connection(mut conn: Connection) -> Result<Self> {
        migrate(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn schema_version(&self) -> Result<u32> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    /// Inserts or updates a scan row.
    pub fn put_scan(&self, scan: &ScanResult) -> Result<()> {
        let roots = serde_json::to_string(&scan.scan.roots)?;
        let json = serde_json::to_string(scan)?;
        self.conn.execute(
            "INSERT INTO scans (id, started_at, finished_at, roots, kipple_version, status, json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET finished_at = ?3, status = ?6, json = ?7",
            params![
                scan.scan.id.as_str(),
                ts_secs(scan.scan.started_at),
                scan.scan.finished_at.map(ts_secs),
                roots,
                scan.scan.kipple_version,
                status_str(scan.scan.status),
                json,
            ],
        )?;
        Ok(())
    }

    /// Stores all items and projects of a scan in one transaction.
    pub fn put_scan_contents(
        &mut self,
        scan_id: &ScanId,
        items: &[Item],
        projects: &[ProjectSummary],
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO items (scan_id, id, integration, category, kind, title,
                    locator, apparent, on_disk, reclaimable, project_id, last_used, safety, json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            )?;
            for item in items {
                stmt.execute(params![
                    scan_id.as_str(),
                    item.id.as_str(),
                    item.integration.as_str(),
                    item.category.id(),
                    item.kind,
                    item.title,
                    item.locator.canonical_key(),
                    item.size.apparent as i64,
                    item.size.on_disk as i64,
                    item.size.reclaimable as i64,
                    item.project.as_ref().map(|p| p.id.as_str()),
                    item.last_used.map(|l| ts_secs(l.at)),
                    item.safety.as_str(),
                    serde_json::to_string(item)?,
                ])?;
            }
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO projects (scan_id, id, root, name, last_used, exists_, json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for project in projects {
                stmt.execute(params![
                    scan_id.as_str(),
                    project.id.as_str(),
                    project.root.to_string_lossy(),
                    project.name,
                    project.last_used.map(|l| ts_secs(l.at)),
                    project.exists,
                    serde_json::to_string(project)?,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn put_size_points(&mut self, points: &[SizePoint]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO size_points (ts, scope, key, reclaimable) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for p in points {
                stmt.execute(params![
                    ts_secs(p.ts),
                    p.scope.as_str(),
                    p.key,
                    p.reclaimable as i64
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// The most recent complete scan.
    pub fn latest_scan(&self) -> Result<Option<ScanResult>> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT json FROM scans WHERE status = 'complete'
                 ORDER BY started_at DESC, id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;
        json.map(|j| serde_json::from_str(&j).map_err(Into::into))
            .transpose()
    }

    pub fn scan(&self, id: &ScanId) -> Result<Option<ScanResult>> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT json FROM scans WHERE id = ?1", [id.as_str()], |r| {
                r.get(0)
            })
            .optional()?;
        json.map(|j| serde_json::from_str(&j).map_err(Into::into))
            .transpose()
    }

    pub fn items(&self, scan_id: &ScanId) -> Result<Vec<Item>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM items WHERE scan_id = ?1 ORDER BY reclaimable DESC, id")?;
        let rows = stmt.query_map([scan_id.as_str()], |r| r.get::<_, String>(0))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(serde_json::from_str(&row?)?);
        }
        Ok(items)
    }

    pub fn projects(&self, scan_id: &ScanId) -> Result<Vec<ProjectSummary>> {
        let mut stmt = self
            .conn
            .prepare("SELECT json FROM projects WHERE scan_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map([scan_id.as_str()], |r| r.get::<_, String>(0))?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(serde_json::from_str(&row?)?);
        }
        Ok(projects)
    }

    pub fn size_points(&self, req: &HistoryRequest) -> Result<Vec<SizePoint>> {
        let since = req.since.map(ts_secs).unwrap_or(i64::MIN);
        let key = match req.scope {
            SizeScope::Total => Some("total".to_owned()),
            _ => req.key.clone(),
        };
        let mut stmt = self.conn.prepare(
            "SELECT ts, key, reclaimable FROM size_points
             WHERE scope = ?1 AND (?2 IS NULL OR key = ?2) AND ts >= ?3
             ORDER BY ts, key",
        )?;
        let rows = stmt.query_map(params![req.scope.as_str(), key, since], |r| {
            Ok(SizePoint {
                ts: from_secs(r.get(0)?),
                scope: req.scope,
                key: r.get(1)?,
                reclaimable: r.get::<_, i64>(2)?.max(0) as u64,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Deletes items and projects of all but the newest `keep` scans.
    pub fn prune(&mut self, keep: usize) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let old: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT id FROM scans ORDER BY started_at DESC, id DESC LIMIT -1 OFFSET ?1",
            )?;
            stmt.query_map([keep as i64], |r| r.get(0))?
                .collect::<std::result::Result<_, _>>()?
        };
        for id in &old {
            tx.execute("DELETE FROM items WHERE scan_id = ?1", [id])?;
            tx.execute("DELETE FROM projects WHERE scan_id = ?1", [id])?;
            tx.execute("DELETE FROM scans WHERE id = ?1", [id])?;
        }
        tx.commit()?;
        Ok(old.len())
    }
}

fn status_str(status: ScanStatus) -> &'static str {
    match status {
        ScanStatus::Running => "running",
        ScanStatus::Complete => "complete",
        ScanStatus::Cancelled => "cancelled",
        ScanStatus::Failed => "failed",
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let known = MIGRATIONS.len() as u32;
    if current > known {
        return Err(IndexError::TooNew {
            found: current,
            known,
        });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as u32)?;
        tx.commit()?;
        tracing::debug!(version = i + 1, "migrated index");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn scan(id: &str, started: i64) -> ScanResult {
        ScanResult {
            scan: ScanInfo {
                id: ScanId::from(id),
                started_at: from_secs(started),
                finished_at: Some(from_secs(started + 5)),
                roots: vec![PathBuf::from("/h")],
                kipple_version: "test".into(),
                status: ScanStatus::Complete,
                entries_seen: 10,
                duration_ms: 5000,
            },
            summary: Summary {
                items: 1,
                reclaimable: 100,
                by_safety: SafetyMix::default(),
                categories: vec![],
                quarantine_bytes: 0,
            },
            warnings: vec![],
        }
    }

    fn item(n: u64) -> Item {
        let integration = IntegrationId::from("npm-cache");
        let locator = Locator::path(format!("/h/item{n}"));
        Item {
            id: ItemId::new(&integration, &locator),
            integration,
            category: Category::PackageCache,
            kind: "npm_cache".into(),
            title: format!("item {n}"),
            locator,
            size: Size::simple(n, n),
            project: None,
            last_used: None,
            safety: Safety::Safe,
            reasons: vec![Reason {
                code: ReasonCode::Redownloadable,
                text: "x".into(),
            }],
            actions: vec![ActionKind::Quarantine],
            details: serde_json::Value::Null,
            notes: vec![],
            restore_hint: None,
            identity: None,
        }
    }

    #[test]
    fn migrates_from_empty() {
        let index = Index::open_in_memory().unwrap();
        assert_eq!(index.schema_version().unwrap(), MIGRATIONS.len() as u32);
        assert!(index.latest_scan().unwrap().is_none());
    }

    #[test]
    fn migrates_file_db_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/index.sqlite");
        drop(Index::open(&path).unwrap());
        let index = Index::open(&path).unwrap();
        assert_eq!(index.schema_version().unwrap(), 1);
    }

    #[test]
    fn refuses_newer_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        assert!(matches!(
            Index::with_connection(conn),
            Err(IndexError::TooNew { found: 99, .. })
        ));
    }

    #[test]
    fn stores_and_reads_scan_contents() {
        let mut index = Index::open_in_memory().unwrap();
        let s = scan("01A", 1000);
        index.put_scan(&s).unwrap();
        let items = vec![item(1), item(5)];
        let project = ProjectSummary {
            id: ProjectId::from("p"),
            root: PathBuf::from("/h/p"),
            name: "p".into(),
            markers: vec!["package.json".into()],
            last_used: None,
            total: Size::default(),
            by_category: BTreeMap::new(),
            safety_mix: SafetyMix::default(),
            item_count: 0,
            exists: true,
        };
        index
            .put_scan_contents(&s.scan.id, &items, std::slice::from_ref(&project))
            .unwrap();
        assert_eq!(index.latest_scan().unwrap().unwrap(), s);
        let read = index.items(&s.scan.id).unwrap();
        assert_eq!(read, vec![item(5), item(1)]);
        assert_eq!(index.projects(&s.scan.id).unwrap(), vec![project]);
    }

    #[test]
    fn prune_keeps_newest_scans() {
        let mut index = Index::open_in_memory().unwrap();
        for i in 0..12 {
            let s = scan(&format!("{i:02}"), 1000 + i);
            index.put_scan(&s).unwrap();
            index
                .put_scan_contents(&s.scan.id, &[item(1)], &[])
                .unwrap();
        }
        assert_eq!(index.prune(KEEP_SCANS).unwrap(), 2);
        assert!(index.scan(&ScanId::from("00")).unwrap().is_none());
        assert!(index.items(&ScanId::from("01")).unwrap().is_empty());
        assert_eq!(index.items(&ScanId::from("11")).unwrap().len(), 1);
        assert_eq!(index.latest_scan().unwrap().unwrap().scan.id.as_str(), "11");
    }

    #[test]
    fn size_points_filter_by_scope() {
        let mut index = Index::open_in_memory().unwrap();
        let p = |ts, scope, key: &str, n| SizePoint {
            ts: from_secs(ts),
            scope,
            key: key.into(),
            reclaimable: n,
        };
        index
            .put_size_points(&[
                p(1, SizeScope::Total, "total", 10),
                p(1, SizeScope::Category, "package-cache", 4),
                p(2, SizeScope::Total, "total", 12),
            ])
            .unwrap();
        let total = index.size_points(&HistoryRequest::default()).unwrap();
        assert_eq!(total.len(), 2);
        let cat = index
            .size_points(&HistoryRequest {
                scope: SizeScope::Category,
                key: None,
                since: None,
            })
            .unwrap();
        assert_eq!(cat, vec![p(1, SizeScope::Category, "package-cache", 4)]);
    }
}
