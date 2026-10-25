//! `cursor`: Cursor's chat database, its backup copy, workspace data for
//! deleted folders and agent transcripts (INTEGRATIONS.md §4.12,
//! docs/integrations/cursor.md).
//!
//! The database is only read during scans (read-only, immutable). The one
//! write kipple ever does is the guarded `VACUUM`, after a backup copy, with
//! Cursor closed. Rows are never edited.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kipple_core::integration::{
    Adapter, Ctx, IntegrationError, ItemDraft, ItemSink, SizeSpec, open_readonly_immutable,
};
use kipple_core::model::{
    ActionKind, Category, Check, Detection, IntegrationInfo, IntegrationStatus, Item, Locator, Os,
    Outcome, Reason, ReasonCode, Safety, Size, Timestamp,
};
use kipple_core::platform::fsx;
use kipple_core::safety::reasons::{reason, reason_with};
use serde_json::{Value, json};

use crate::ai;
use crate::util::{children, file_name, info, mtime_used};

pub const ID: &str = "cursor";
pub const VACUUM: &str = "sqlite_vacuum";

/// The Cursor app's process name on every OS (`Cursor`, `cursor`,
/// `Cursor.exe`; names compare case-insensitively).
const PROCESS: &str = "Cursor";

/// Compaction is offered from this much free space (INTEGRATIONS.md §4.12).
pub const MIN_FREE_PAGES: u64 = 256 * 1000 * 1000;

/// Backups of the database are kept this long, then removed by the next
/// compaction.
const BACKUP_KEEP: Duration = Duration::from_secs(7 * 86_400);

/// Key prefixes of `cursorDiskKV` reported separately.
const PREFIXES: &[&str] = &["bubbleId:", "composerData:", "agentKv:", "checkpointId:"];

/// Free bytes on the volume holding a path.
pub type FreeSpace = fn(&Path) -> Option<u64>;

pub struct Cursor {
    /// Smallest amount of free pages worth compacting.
    pub min_free_pages: u64,
    /// Free space on a volume. Injectable for tests.
    pub free_space: FreeSpace,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            min_free_pages: MIN_FREE_PAGES,
            free_space: volume_free_space,
        }
    }
}

/// Free bytes available to the user on the volume of `path`.
#[cfg(unix)]
pub fn volume_free_space(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let dir = path.ancestors().find(|p| p.exists())?;
    let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is a valid NUL-terminated path and `st` a valid out
    // pointer; statvfs only writes into it.
    let rc = unsafe { libc::statvfs(c.as_ptr(), &mut st) };
    (rc == 0).then(|| (st.f_bavail as u64).saturating_mul(st.f_frsize as u64))
}

/// Not measured on this OS yet, so compaction is refused.
#[cfg(not(unix))]
pub fn volume_free_space(_path: &Path) -> Option<u64> {
    None
}

/// Read-only facts about `state.vscdb`.
#[derive(Debug, Default, Clone)]
pub struct DbStats {
    pub page_size: u64,
    pub page_count: u64,
    pub free_bytes: u64,
    pub has_item_table: bool,
    pub has_kv: bool,
    /// Bytes of values per key prefix, `other` last.
    pub by_prefix: Vec<(String, u64)>,
}

/// Reads stats from a database opened read-only.
pub fn db_stats(conn: &rusqlite::Connection) -> rusqlite::Result<DbStats> {
    let pragma = |name: &str| -> rusqlite::Result<u64> {
        conn.query_row(&format!("PRAGMA {name}"), [], |r| r.get::<_, i64>(0))
            .map(|v| v.max(0) as u64)
    };
    let table = |name: &str| -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
    };
    let mut s = DbStats {
        page_size: pragma("page_size")?,
        page_count: pragma("page_count")?,
        ..Default::default()
    };
    s.free_bytes = pragma("freelist_count")? * s.page_size;
    s.has_item_table = table("ItemTable")?;
    s.has_kv = table("cursorDiskKV")?;
    if s.has_kv {
        let mut case = String::from("CASE");
        for p in PREFIXES {
            case.push_str(&format!(
                " WHEN substr(key, 1, {}) = '{p}' THEN '{p}'",
                p.len()
            ));
        }
        case.push_str(" ELSE 'other' END");
        let sql = format!(
            "SELECT {case} AS prefix, COALESCE(SUM(length(value)), 0) FROM cursorDiskKV GROUP BY prefix"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?.max(0) as u64))
        })?;
        let mut found: Vec<(String, u64)> = Vec::new();
        for row in rows {
            found.push(row?);
        }
        for p in PREFIXES
            .iter()
            .map(|p| (*p).to_owned())
            .chain(["other".to_owned()])
        {
            let bytes = found
                .iter()
                .find(|(k, _)| *k == p)
                .map(|(_, b)| *b)
                .unwrap_or(0);
            s.by_prefix
                .push((p.trim_end_matches(':').to_owned(), bytes));
        }
    }
    Ok(s)
}

/// Decodes `%XX` escapes.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let hex = |b: u8| (b as char).to_digit(16);
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A local path from a `file://` URI, as VS Code writes it in
/// `workspace.json`. Other schemes (remote, WSL) give `None`.
pub fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let decoded = percent_decode(rest);
    // `file:///c:/Users/…` on Windows.
    let b = decoded.as_bytes();
    if b.len() >= 3 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b':' {
        return Some(PathBuf::from(&decoded[1..]));
    }
    // `file://C:\…`, as some tools write it.
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return Some(PathBuf::from(decoded));
    }
    decoded.starts_with('/').then(|| PathBuf::from(decoded))
}

impl Cursor {
    /// Cursor's `User` folder for this OS.
    pub fn user_dir(ctx: &Ctx) -> Option<PathBuf> {
        let env = &ctx.platform.env;
        let home = env.home();
        match ctx.platform.os {
            Os::Macos => home.map(|h| h.join("Library/Application Support/Cursor/User")),
            Os::Windows => env.get_path("APPDATA").map(|a| a.join("Cursor/User")),
            Os::Linux | Os::Wsl => env
                .get_path("XDG_CONFIG_HOME")
                .or_else(|| home.map(|h| h.join(".config")))
                .map(|c| c.join("Cursor/User")),
        }
    }

    /// `~/.cursor/projects`.
    fn projects_dir(ctx: &Ctx) -> Option<PathBuf> {
        ctx.home().map(|h| h.join(".cursor/projects"))
    }

    fn running(ctx: &Ctx) -> bool {
        ctx.platform.processes.snapshot().any_running(PROCESS)
    }

    fn db_item(&self, db: &Path) -> Option<ItemDraft> {
        let meta = db.metadata().ok()?;
        let stats = open_readonly_immutable(db).and_then(|c| db_stats(&c));
        let mut d = ItemDraft::new("cursor_state_db", "Cursor chat database", Locator::path(db));
        d.category = Some(Category::AiHistory);
        d.last_used = mtime_used(db);
        d.safety = Safety::Review;
        d.notes.push(
            "To remove old chats, use Cursor's command palette: \"Developer: Delete Old Chats\", then \"Developer: GC Agent KV Blobs\". Quit Cursor and scan again afterwards."
                .to_owned(),
        );
        let stats = match stats {
            Ok(s) if s.has_item_table => s,
            Ok(_) | Err(_) => {
                d.unknown_format = true;
                d.reasons.push(reason(ReasonCode::UnknownFormat));
                d.size = SizeSpec::Known(Size::simple(meta.len(), meta.len()));
                return Some(d);
            }
        };
        let size = kipple_core::format::size(stats.free_bytes, Default::default());
        d.size = SizeSpec::Known(Size {
            apparent: meta.len(),
            on_disk: meta.len(),
            reclaimable: stats.free_bytes,
            file_count: Some(1),
        });
        if stats.free_bytes >= self.min_free_pages {
            d.reasons
                .push(reason_with(ReasonCode::FreePages, &[("size", size)]));
            d.actions = vec![ActionKind::Compact {
                method: VACUUM.to_owned(),
            }];
        } else {
            d.reasons.push(Reason {
                code: ReasonCode::FreePages,
                text: format!(
                    "Only {size} of unused space; kipple compacts from {}.",
                    kipple_core::format::size(self.min_free_pages, Default::default())
                ),
            });
        }
        d.reasons.push(reason(ReasonCode::ChatHistory));
        if ai::modified_within(db, kipple_core::safety::engine::RECENT_WRITE) {
            d.reasons.insert(0, reason(ReasonCode::RecentlyWritten));
            d.safety = Safety::Keep;
            d.actions.clear();
        }
        let by_prefix: serde_json::Map<String, Value> = stats
            .by_prefix
            .iter()
            .map(|(k, v)| (k.clone(), json!(v)))
            .collect();
        d.details = json!({
            "page_size": stats.page_size,
            "pages": stats.page_count,
            "free_bytes": stats.free_bytes,
            "chat_bytes_by_key_prefix": by_prefix,
            "has_chat_table": stats.has_kv,
        });
        Some(d)
    }

    fn backup_item(path: &Path) -> ItemDraft {
        let mut d = ItemDraft::new(
            "cursor_state_backup",
            "Cursor chat database backup",
            Locator::path(path),
        );
        d.category = Some(Category::AiHistory);
        d.safety = Safety::Review;
        d.reasons = vec![
            reason(ReasonCode::BackupCopy),
            reason(ReasonCode::ChatHistory),
        ];
        d.actions = vec![ActionKind::Quarantine];
        d.notes.push(
            "Cursor uses this copy to recover a damaged database and writes a new one when it closes."
                .to_owned(),
        );
        d
    }

    fn workspace_items(ctx: &Ctx, user: &Path, sink: &mut dyn ItemSink) {
        for dir in children(&user.join("workspaceStorage")) {
            let Ok(text) = std::fs::read_to_string(dir.join("workspace.json")) else {
                continue;
            };
            let Ok(v) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let (target, is_folder) = match (
                v.get("folder").and_then(Value::as_str),
                v.get("workspace").and_then(Value::as_str),
            ) {
                (Some(f), _) => (file_uri_path(f), true),
                (None, Some(w)) => (file_uri_path(w), false),
                _ => (None, false),
            };
            let Some(target) = target else { continue };
            if target.exists() {
                continue;
            }
            let name = file_name(&target);
            let mut d = ItemDraft::new(
                "cursor_workspace_storage",
                format!("Cursor data for {name}"),
                Locator::path(&dir),
            );
            d.category = Some(Category::AiHistory);
            d.safety = Safety::Review;
            d.reasons = vec![
                reason(ReasonCode::OrphanedProject),
                reason(ReasonCode::ChatHistory),
            ];
            d.actions = vec![ActionKind::Quarantine];
            let home = ctx.home();
            if is_folder && home.as_deref() != Some(target.as_path()) {
                d.project_hint = Some(target.clone());
            }
            d.details = json!({
                "workspace": target,
                "kind": if is_folder { "folder" } else { "workspace file" },
            });
            sink.item(d);
        }
    }

    fn transcript_items(ctx: &Ctx, sink: &mut dyn ItemSink) {
        let Some(projects) = Self::projects_dir(ctx) else {
            return;
        };
        for p in children(&projects) {
            let dir = p.join("agent-transcripts");
            if !dir.is_dir() {
                continue;
            }
            let mut d = ItemDraft::new(
                "cursor_agent_transcripts",
                format!("Cursor agent transcripts · {}", file_name(&p)),
                Locator::path(&dir),
            );
            d.category = Some(Category::AiHistory);
            d.safety = Safety::Review;
            d.reasons = ai::chat_reasons();
            d.actions = vec![ActionKind::Quarantine];
            d.details = json!({ "project_folder": file_name(&p) });
            sink.item(d);
        }
    }

    /// Removes old database backups from kipple's backups folder.
    fn prune_backups(ctx: &Ctx, dir: &Path) {
        for p in children(dir) {
            let name = file_name(&p);
            if !name.starts_with("cursor-") || !name.contains(".vscdb") {
                continue;
            }
            let old = ai::mtime(&p).is_some_and(|t| ai::age_secs(t) > BACKUP_KEEP.as_secs());
            if old
                && fsx::sandbox_guard(&ctx.platform.env, &p).is_ok()
                && let Err(e) = std::fs::remove_file(&p)
            {
                tracing::warn!("couldn't remove old backup {}: {e}", p.display());
            }
        }
    }

    fn vacuum(&self, ctx: &Ctx, item: &Item, db: &Path) -> Result<Outcome, IntegrationError> {
        fsx::sandbox_guard(&ctx.platform.env, db)
            .map_err(|e| IntegrationError::other(e.to_string()))?;
        let wal = sidecar(db, "-wal");
        let size_of = |p: &Path| p.metadata().map(|m| m.len()).unwrap_or(0);
        let before = size_of(db) + size_of(&wal);

        let backups = ctx.dirs.backups_dir();
        fsx::sandbox_guard(&ctx.platform.env, &backups)
            .map_err(|e| IntegrationError::other(e.to_string()))?;
        std::fs::create_dir_all(&backups)?;
        Self::prune_backups(ctx, &backups);
        let stamp = Timestamp::now().strftime("%Y%m%dT%H%M%SZ").to_string();
        let backup = backups.join(format!("cursor-{stamp}.vscdb"));
        std::fs::copy(db, &backup)?;
        if wal.exists() {
            std::fs::copy(&wal, sidecar(&backup, "-wal"))?;
        }

        let conn = rusqlite::Connection::open(db)?;
        conn.execute_batch("VACUUM")?;
        let check: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        drop(conn);
        if check != "ok" {
            return Ok(Outcome::Failed {
                error: format!(
                    "The database failed its integrity check after compaction. A copy from before is at {}.",
                    backup.display()
                ),
            });
        }
        let after = size_of(db) + size_of(&wal);
        tracing::info!(
            item = %item.id,
            backup = %backup.display(),
            "compacted Cursor database"
        );
        Ok(Outcome::Done {
            freed: before.saturating_sub(after),
        })
    }
}

fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut s = db.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

impl Adapter for Cursor {
    fn info(&self) -> IntegrationInfo {
        let mut i = info(
            ID,
            "Cursor",
            Category::AiHistory,
            "Cursor's chat database, its backup copy, data for workspaces you deleted, and agent transcripts.",
            // Storage is reverse-engineered, not documented by Cursor.
            IntegrationStatus::Experimental,
            &[],
            &[PROCESS],
        );
        i.docs_url = Some("https://forum.cursor.com/t/state-vscdb-grows-to-30gb-due-to-bubbleid-agentkv-entries/167641".to_owned());
        i
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let user = Self::user_dir(ctx).filter(|u| u.is_dir());
        let projects = Self::projects_dir(ctx).filter(|p| p.is_dir());
        if user.is_some() || projects.is_some() {
            Detection::present()
        } else {
            Detection::NotPresent
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        [Self::user_dir(ctx), Self::projects_dir(ctx)]
            .into_iter()
            .flatten()
            .filter(|p| p.is_dir())
            .collect()
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        if let Some(user) = Self::user_dir(ctx).filter(|u| u.is_dir()) {
            let global = user.join("globalStorage");
            let db = global.join("state.vscdb");
            if db.is_file()
                && let Some(d) = self.db_item(&db)
            {
                sink.item(d);
            }
            let backup = global.join("state.vscdb.backup");
            if backup.is_file() {
                sink.item(Self::backup_item(&backup));
            }
            Self::workspace_items(ctx, &user, sink);
        }
        Self::transcript_items(ctx, sink);
        Ok(())
    }

    fn preflight(&self, ctx: &Ctx, item: &Item, action: &ActionKind) -> Vec<Check> {
        let ActionKind::Compact { method } = action else {
            return vec![Check::fail(
                "action",
                format!("Cursor can't run `{action}`."),
            )];
        };
        if method != VACUUM {
            return vec![Check::fail(
                "action",
                format!("Unknown compaction `{method}`."),
            )];
        }
        let mut checks = Vec::new();
        let Some(db) = item.locator.primary_path() else {
            return vec![Check::fail("path", "This item has no database path.")];
        };
        checks.push(if Self::running(ctx) {
            Check::fail("closed", "Quit Cursor first.")
        } else {
            Check::pass("closed", "Cursor is closed")
        });
        match open_readonly_immutable(db).and_then(|c| db_stats(&c)) {
            Ok(s) if s.free_bytes >= self.min_free_pages => {
                checks.push(Check::pass("free_pages", "enough unused space to compact"));
            }
            Ok(_) => checks.push(Check::fail(
                "free_pages",
                "The database no longer has enough unused space to be worth compacting.",
            )),
            Err(e) => checks.push(Check::fail("read", format!("Can't read the database: {e}"))),
        }
        // A backup copy plus SQLite's own temporary copy during VACUUM.
        let db_size = db.metadata().map(|m| m.len()).unwrap_or(u64::MAX);
        let need = db_size.saturating_mul(2);
        let backups = ctx.dirs.backups_dir();
        let free_backup = (self.free_space)(&backups);
        let free_db = (self.free_space)(db);
        checks.push(match (free_backup, free_db) {
            (Some(b), Some(d)) if b.min(d) >= need => {
                Check::pass("space", "enough free space for a backup and compaction")
            }
            (Some(_), Some(_)) => Check::fail(
                "space",
                format!(
                    "Compacting needs {} free for a backup copy and SQLite's working copy.",
                    kipple_core::format::size(need, Default::default())
                ),
            ),
            _ => Check::fail(
                "space",
                "kipple can't check free space here, so it won't compact.",
            ),
        });
        checks
    }

    fn apply(
        &self,
        ctx: &Ctx,
        item: &Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        match action {
            ActionKind::Compact { method } if method == VACUUM => {
                let db = item
                    .locator
                    .primary_path()
                    .ok_or_else(|| IntegrationError::other("this item has no database path"))?;
                if Self::running(ctx) {
                    return Ok(Outcome::Skipped {
                        reason: "Quit Cursor first.".to_owned(),
                    });
                }
                self.vacuum(ctx, item, db)
            }
            _ => Err(IntegrationError::Unsupported(format!(
                "Cursor can't run `{action}`"
            ))),
        }
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        matches!(action, ActionKind::Compact { method } if method == VACUUM).then(|| {
            "With Cursor closed, copy the database to kipple's backups folder (kept 7 days), run SQLite VACUUM to return unused space, then check the database's integrity. No chats are removed.".to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_file_uris() {
        assert_eq!(
            file_uri_path("file:///Users/me/My%20App"),
            Some(PathBuf::from("/Users/me/My App"))
        );
        assert_eq!(
            file_uri_path("file:///c%3A/Users/me/app"),
            Some(PathBuf::from("c:/Users/me/app"))
        );
        assert_eq!(
            file_uri_path("vscode-remote://ssh-remote+box/home/me"),
            None
        );
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn stats_group_values_by_prefix() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
             CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
             INSERT INTO cursorDiskKV VALUES ('bubbleId:a:1', zeroblob(100)), ('bubbleId:a:2', zeroblob(50)),
               ('composerData:a', zeroblob(10)), ('agentKv:x', zeroblob(7)), ('misc', zeroblob(3));",
        )
        .unwrap();
        let s = db_stats(&conn).unwrap();
        assert!(s.has_item_table && s.has_kv);
        assert_eq!(
            s.by_prefix,
            vec![
                ("bubbleId".to_owned(), 150),
                ("composerData".to_owned(), 10),
                ("agentKv".to_owned(), 7),
                ("checkpointId".to_owned(), 0),
                ("other".to_owned(), 3),
            ]
        );
    }
}
