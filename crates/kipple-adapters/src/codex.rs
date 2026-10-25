//! `codex`: OpenAI Codex sessions (INTEGRATIONS.md §4.11,
//! docs/integrations/codex.md).
//!
//! Sessions are removed only with `codex delete <uuid> --force`. kipple never
//! deletes rollout files itself: that would leave threads behind in Codex's
//! state database.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink};
use kipple_core::model::{
    ActionKind, Category, Check, Detection, IntegrationInfo, IntegrationStatus, Item, Locator,
    Outcome, ReasonCode, Safety, Timestamp,
};
use serde_json::{Value, json};

use crate::ai;
use crate::util::{children, file_name, info, mtime_used};

pub const ID: &str = "codex";
pub const DELETE: &str = "codex.delete";

/// Every Codex surface (CLI, desktop app, IDE extension) runs a process
/// named `codex` / `Codex`; names compare case-insensitively.
const PROCESS: &str = "codex";

/// Sessions above this size get a note.
const LARGE: u64 = 100 * 1000 * 1000;

/// Only rollouts modified this recently are checked for growth.
const GROWTH_WINDOW: Duration = Duration::from_secs(3600);

/// One row of the `threads` table.
#[derive(Debug, Default, Clone)]
struct Thread {
    title: Option<String>,
    cwd: Option<PathBuf>,
    source: Option<String>,
    archived: bool,
    created_ms: Option<i64>,
    updated_ms: Option<i64>,
}

/// What the state database told us.
#[derive(Debug, Default)]
struct State {
    threads: HashMap<String, Thread>,
    /// The database exists but isn't in a shape we know.
    unknown: bool,
    db: Option<PathBuf>,
}

pub struct Codex {
    /// Delay between the two size checks that detect an active writer
    /// (SPEC §7.2). Injectable so tests stay fast.
    pub growth_delay: Duration,
    /// Memoized `codex delete --help` support, per binary path.
    delete_support: Mutex<HashMap<PathBuf, bool>>,
}

impl Default for Codex {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}

/// `true` for a canonical lowercase/uppercase hex UUID.
pub fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

/// Thread id from `rollout-<timestamp>-<uuid>.jsonl[.zst]`: the last 36
/// characters of the stem (as Codex itself parses it).
pub fn thread_id(path: &Path) -> Option<String> {
    let name = file_name(path);
    if !name.starts_with("rollout-") {
        return None;
    }
    let stem = name
        .strip_suffix(".jsonl.zst")
        .or_else(|| name.strip_suffix(".jsonl"))?;
    let id = stem.get(stem.len().checked_sub(36)?..)?;
    is_uuid(id).then(|| id.to_ascii_lowercase())
}

fn ms_to_ts(ms: i64) -> Option<Timestamp> {
    // Older rows store seconds.
    let ms = if ms < 1_577_836_800_000 {
        ms * 1000
    } else {
        ms
    };
    Timestamp::from_millisecond(ms).ok()
}

/// Collects rollout files under `dir`, any depth (`YYYY/MM/DD/`).
fn rollouts(dir: &Path, out: &mut Vec<(PathBuf, bool)>, archived: bool) {
    for p in children(dir) {
        let Ok(meta) = p.symlink_metadata() else {
            continue;
        };
        if meta.is_dir() {
            rollouts(&p, out, archived);
        } else if meta.is_file() && thread_id(&p).is_some() {
            out.push((p, archived));
        }
    }
}

/// Facts from the head of a plain JSONL rollout.
#[derive(Debug, Default)]
struct RolloutMeta {
    recognised: bool,
    cwd: Option<PathBuf>,
    source: Option<String>,
    cli_version: Option<String>,
    first_prompt: Option<String>,
}

fn read_rollout(path: &Path) -> RolloutMeta {
    let mut m = RolloutMeta::default();
    let mut first = true;
    ai::scan_head(path, |v| {
        let kind = v.get("type").and_then(Value::as_str);
        let payload = v.get("payload");
        if first {
            first = false;
            m.recognised = kind == Some("session_meta");
        }
        if kind == Some("session_meta")
            && let Some(p) = payload
        {
            m.cwd = p.get("cwd").and_then(Value::as_str).map(PathBuf::from);
            m.cli_version = p
                .get("cli_version")
                .and_then(Value::as_str)
                .map(str::to_owned);
            m.source = match p.get("source") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Object(o)) => o.keys().next().cloned(),
                _ => None,
            };
        }
        if m.first_prompt.is_none()
            && kind == Some("event_msg")
            && payload.and_then(|p| p.get("type")).and_then(Value::as_str) == Some("user_message")
        {
            m.first_prompt = payload
                .and_then(|p| p.get("message"))
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        m.recognised && m.first_prompt.is_none()
    });
    m
}

impl Codex {
    pub fn new(growth_delay: Duration) -> Self {
        Self {
            growth_delay,
            delete_support: Mutex::new(HashMap::new()),
        }
    }

    /// `CODEX_HOME` → `~/.codex`.
    pub fn root(ctx: &Ctx) -> Option<PathBuf> {
        crate::util::env_or_home(&ctx.platform.env, &["CODEX_HOME"], ".codex")
    }

    /// `CODEX_SQLITE_HOME` → the Codex root.
    fn sqlite_home(ctx: &Ctx, root: &Path) -> PathBuf {
        ctx.platform
            .env
            .get_path("CODEX_SQLITE_HOME")
            .unwrap_or_else(|| root.to_path_buf())
    }

    /// The newest `state_<n>.sqlite`.
    fn state_db(dir: &Path) -> Option<PathBuf> {
        children(dir)
            .into_iter()
            .filter_map(|p| {
                let name = file_name(&p);
                let n: u32 = name
                    .strip_prefix("state_")?
                    .strip_suffix(".sqlite")?
                    .parse()
                    .ok()?;
                Some((n, p))
            })
            .max_by_key(|(n, _)| *n)
            .map(|(_, p)| p)
    }

    /// Reads thread metadata strictly read-only.
    fn read_state(ctx: &Ctx, db: Option<PathBuf>) -> State {
        let mut state = State {
            db: db.clone(),
            ..Default::default()
        };
        let Some(db) = db else {
            return state;
        };
        let result = (|| -> rusqlite::Result<bool> {
            let conn = ctx.open_readonly_immutable(&db)?;
            let mut cols: Vec<String> = Vec::new();
            let mut stmt = conn.prepare("SELECT name FROM pragma_table_info('threads')")?;
            for name in stmt.query_map([], |r| r.get::<_, String>(0))? {
                cols.push(name?);
            }
            let has = |c: &str| cols.iter().any(|x| x == c);
            if !(has("id") && has("rollout_path") && has("cwd")) {
                return Ok(false);
            }
            let pick = |names: &[&str]| -> String {
                names
                    .iter()
                    .find(|n| has(n))
                    .map(|n| (*n).to_owned())
                    .unwrap_or_else(|| "NULL".to_owned())
            };
            // Columns vary by Codex version (state migrations 0001–0062).
            let sql = format!(
                "SELECT id, cwd, {name}, {title}, {preview}, {first}, {source}, {archived}, {created}, {updated} FROM threads",
                name = pick(&["name"]),
                title = pick(&["title"]),
                preview = pick(&["preview"]),
                first = pick(&["first_user_message"]),
                source = pick(&["source"]),
                archived = pick(&["archived"]),
                created = pick(&["created_at_ms", "created_at"]),
                updated = pick(&["updated_at_ms", "updated_at"]),
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], |r| {
                let text = |i: usize| -> Option<String> {
                    r.get::<_, Option<String>>(i)
                        .ok()
                        .flatten()
                        .filter(|s| !s.trim().is_empty())
                };
                let id: String = r.get(0)?;
                Ok((
                    id.to_ascii_lowercase(),
                    Thread {
                        cwd: text(1).map(PathBuf::from),
                        title: text(2).or(text(3)).or(text(4)).or(text(5)),
                        source: text(6),
                        archived: r.get::<_, Option<i64>>(7).ok().flatten().unwrap_or(0) != 0,
                        created_ms: r.get::<_, Option<i64>>(8).ok().flatten(),
                        updated_ms: r.get::<_, Option<i64>>(9).ok().flatten(),
                    },
                ))
            })?;
            for row in rows {
                let (id, t) = row?;
                state.threads.insert(id, t);
            }
            Ok(true)
        })();
        match result {
            Ok(true) => {}
            Ok(false) => state.unknown = true,
            Err(e) => {
                tracing::warn!("codex: can't read {}: {e}", db.display());
                state.unknown = true;
            }
        }
        state
    }

    /// Whether the installed `codex` has a non-interactive `delete`.
    fn delete_supported(&self, ctx: &Ctx) -> bool {
        let Some(bin) = ctx.platform.runner.which("codex") else {
            return false;
        };
        if let Ok(memo) = self.delete_support.lock()
            && let Some(hit) = memo.get(&bin)
        {
            return *hit;
        }
        let ok = ctx
            .platform
            .runner
            .run(
                &["codex", "delete", "--help"],
                None,
                Duration::from_secs(10),
            )
            .is_ok_and(|out| {
                out.success() && format!("{}{}", out.stdout, out.stderr).contains("--force")
            });
        if let Ok(mut memo) = self.delete_support.lock() {
            memo.insert(bin, ok);
        }
        ok
    }

    fn draft(
        &self,
        ctx: &Ctx,
        path: &Path,
        id: &str,
        archived_dir: bool,
        state: &State,
        can_delete: bool,
    ) -> ItemDraft {
        let thread = state.threads.get(id);
        let compressed = file_name(path).ends_with(".zst");
        let meta = if compressed {
            RolloutMeta::default()
        } else {
            read_rollout(path)
        };
        let title = thread
            .and_then(|t| t.title.as_deref())
            .or(meta.first_prompt.as_deref())
            .and_then(ai::title)
            // Thread ids are UUIDv7: the first characters are a timestamp
            // shared by nearby sessions, so the tail tells them apart.
            .unwrap_or_else(|| format!("Codex session …{}", &id[id.len().saturating_sub(8)..]));
        let mut d = ItemDraft::new("codex_session", title, Locator::path(path));
        d.category = Some(Category::AiHistory);
        d.safety = Safety::Review;
        d.reasons = ai::chat_reasons();
        d.actions = vec![ActionKind::native(DELETE)];
        let cwd = thread.and_then(|t| t.cwd.clone()).or(meta.cwd.clone());
        let home = ctx.home();
        d.project_hint = cwd
            .clone()
            .filter(|c| c.is_absolute() && home.as_deref() != Some(c.as_path()));
        d.last_used = thread
            .and_then(|t| t.updated_ms)
            .and_then(ms_to_ts)
            .map(ai::tool_used)
            .or_else(|| mtime_used(path));
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        if size > LARGE {
            d.notes
                .push("Most of this is probably saved tool output, not conversation.".to_owned());
        }
        if compressed {
            d.notes.push(
                "Codex stores this session compressed and partly in its database.".to_owned(),
            );
        }
        // A plain rollout must start with `session_meta`; a compressed one
        // must at least be known to Codex's database.
        let recognised = if compressed {
            thread.is_some()
        } else {
            meta.recognised || size == 0
        };
        if !recognised || state.unknown {
            d.unknown_format = true;
        }
        d.details = json!({
            "thread_id": id,
            "source": thread.and_then(|t| t.source.clone()).or(meta.source),
            "archived": archived_dir || thread.is_some_and(|t| t.archived),
            "created_at": thread.and_then(|t| t.created_ms).and_then(ms_to_ts),
            "cli_version": meta.cli_version,
            "cwd": cwd,
            "in_state_db": thread.is_some(),
        });
        if !can_delete {
            d.safety = Safety::Keep;
            d.actions.clear();
            d.reasons.insert(0, ai::tool_missing("Codex"));
            d.notes.push(
                "kipple removes Codex sessions only with `codex delete`. Install or update Codex, then scan again."
                    .to_owned(),
            );
        }
        d
    }
}

impl Adapter for Codex {
    fn info(&self) -> IntegrationInfo {
        let mut i = info(
            ID,
            "Codex",
            Category::AiHistory,
            "OpenAI Codex's saved sessions. kipple removes them only with Codex's own `codex delete`.",
            IntegrationStatus::Stable,
            &[],
            &[PROCESS],
        );
        i.docs_url = Some("https://developers.openai.com/codex/cli/reference".to_owned());
        i
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let root = Self::root(ctx).filter(|r| r.join("sessions").is_dir());
        if root.is_none() && ctx.platform.runner.which("codex").is_none() {
            return Detection::NotPresent;
        }
        let version = ctx
            .platform
            .runner
            .run(&["codex", "--version"], None, Duration::from_secs(10))
            .ok()
            .filter(|o| o.success())
            .and_then(|o| o.stdout.split_whitespace().last().map(str::to_owned));
        Detection::Present { version }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        Self::root(ctx).filter(|r| r.is_dir()).into_iter().collect()
    }

    fn tools(&self) -> Vec<String> {
        vec!["codex".to_owned()]
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(root) = Self::root(ctx) else {
            return Ok(());
        };
        let mut files = Vec::new();
        rollouts(&root.join("sessions"), &mut files, false);
        rollouts(&root.join("archived_sessions"), &mut files, true);
        if files.is_empty() {
            return Ok(());
        }
        let state = Self::read_state(ctx, Self::state_db(&Self::sqlite_home(ctx, &root)));
        if state.unknown {
            sink.warning(format!(
                "{} isn't in a format kipple knows; Codex sessions are listed but kept.",
                state.db.as_deref().unwrap_or(&root).display()
            ));
        }
        let can_delete = self.delete_supported(ctx);

        // Rollouts written recently are checked for growth once, together.
        let fresh: Vec<&Path> = files
            .iter()
            .map(|(p, _)| p.as_path())
            .filter(|p| ai::modified_within(p, GROWTH_WINDOW))
            .collect();
        let grew: Vec<PathBuf> = fresh
            .iter()
            .zip(ai::growing(&fresh, self.growth_delay))
            .filter(|(_, g)| *g)
            .map(|(p, _)| p.to_path_buf())
            .collect();

        for (path, archived) in &files {
            if ctx.cancel.is_cancelled() {
                return Ok(());
            }
            let Some(id) = thread_id(path) else { continue };
            let mut d = self.draft(ctx, path, &id, *archived, &state, can_delete);
            if grew.contains(path) {
                d.safety = Safety::Keep;
                d.actions.clear();
                d.reasons.insert(
                    0,
                    kipple_core::safety::reasons::reason(ReasonCode::ActiveWriter),
                );
            }
            sink.item(d);
        }
        Ok(())
    }

    fn preflight(&self, ctx: &Ctx, item: &Item, action: &ActionKind) -> Vec<Check> {
        let mut checks = Vec::new();
        if !matches!(action, ActionKind::Native { command_id } if command_id == DELETE) {
            checks.push(Check::fail(
                "action",
                "Codex sessions are only removed with `codex delete`.",
            ));
            return checks;
        }
        let id = item.details.get("thread_id").and_then(Value::as_str);
        checks.push(match id {
            Some(id) if is_uuid(id) => Check::pass("session", "session id is valid"),
            _ => Check::fail("session", "This session has no valid id."),
        });
        checks.push(if self.delete_supported(ctx) {
            Check::pass("tool", "codex delete is available")
        } else {
            Check::fail(
                "tool",
                "Codex isn't installed or is too old to delete sessions.",
            )
        });
        let procs = ctx.platform.processes.snapshot();
        if procs.any_running(PROCESS) {
            checks.push(Check::fail(
                "closed",
                "Quit Codex first (CLI, app and editor extension).",
            ));
        }
        if let Some(path) = item.locator.primary_path()
            && ai::growing(&[path.as_path()], self.growth_delay)
                .first()
                .copied()
                .unwrap_or(false)
        {
            checks.push(Check::fail(
                "writer",
                kipple_core::safety::reasons::reason(ReasonCode::ActiveWriter).text,
            ));
        }
        checks
    }

    fn apply(
        &self,
        ctx: &Ctx,
        item: &Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        if !matches!(action, ActionKind::Native { command_id } if command_id == DELETE) {
            return Err(IntegrationError::Unsupported(format!(
                "Codex sessions can't be removed with `{action}`"
            )));
        }
        let id = item
            .details
            .get("thread_id")
            .and_then(Value::as_str)
            .filter(|id| is_uuid(id))
            .ok_or_else(|| IntegrationError::other("this session has no valid id"))?;
        let argv = ["codex", "delete", id, "--force"];
        let out = ctx
            .platform
            .runner
            .run(&argv, None, Duration::from_secs(120))?;
        if !out.success() {
            let msg = out
                .stderr
                .lines()
                .chain(out.stdout.lines())
                .find(|l| !l.trim().is_empty())
                .unwrap_or("no output")
                .trim()
                .to_owned();
            return Ok(Outcome::Failed {
                error: format!("`codex delete` failed: {msg}"),
            });
        }
        let still_there = item.locator.primary_path().is_some_and(|p| p.exists());
        if still_there {
            return Ok(Outcome::Failed {
                error: "`codex delete` finished but the session file is still there.".to_owned(),
            });
        }
        Ok(Outcome::Done {
            freed: item.size.reclaimable,
        })
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        matches!(action, ActionKind::Native { command_id } if command_id == DELETE).then(|| {
            "Run `codex delete <session-id> --force`, Codex's own command. This removes the session for good; it can't be undone. (`codex archive` only hides a session and frees nothing.)".to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_thread_ids_from_rollout_names() {
        let id = "019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b";
        for name in [
            format!("rollout-2026-01-15T10-00-00-{id}.jsonl"),
            format!("rollout-2026-01-15T10-00-00-{id}.jsonl.zst"),
        ] {
            assert_eq!(thread_id(Path::new(&name)).as_deref(), Some(id));
        }
        assert_eq!(thread_id(Path::new("rollout-x.jsonl")), None);
        assert_eq!(thread_id(Path::new(&format!("other-{id}.jsonl"))), None);
    }

    #[test]
    fn validates_uuids() {
        assert!(is_uuid("019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7b"));
        assert!(!is_uuid("019a1b2c-3d4e-7f80-9a1b-2c3d4e5f6a7"));
        assert!(!is_uuid("--force-3d4e-7f80-9a1b-2c3d4e5f6a7b"));
    }

    #[test]
    fn reads_seconds_and_milliseconds() {
        let a = ms_to_ts(1_768_471_200).unwrap();
        let b = ms_to_ts(1_768_471_200_000).unwrap();
        assert_eq!(a, b);
    }
}
