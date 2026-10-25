//! `claude-code`: Claude Code transcripts and session data
//! (INTEGRATIONS.md §4.10, docs/integrations/claude-code.md).
//!
//! Transcripts are moved whole into quarantine, never edited. Settings,
//! `CLAUDE.md`, agents, commands, skills, plugins, auto memory and
//! credentials are never offered.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kipple_core::integration::{Adapter, Ctx, IntegrationError, ItemDraft, ItemSink};
use kipple_core::model::{
    ActionKind, Category, Detection, IntegrationInfo, IntegrationStatus, Locator, Reason,
    ReasonCode, Safety,
};
use serde_json::{Value, json};

use crate::ai;
use crate::util::{children, file_name, info, mtime_used};

pub const ID: &str = "claude-code";

/// While `claude` runs, sessions changed this recently are kept.
const ACTIVE_WINDOW: Duration = Duration::from_secs(24 * 3600);

/// Process name of the Claude Code CLI.
const PROCESS: &str = "claude";

/// Folders under the root that Claude Code writes as it works and that
/// kipple reports as separate Review items. Each with a short description.
/// Source: code.claude.com/docs/en/claude-directory ("Application data"),
/// checked 2026-10-25.
const DATA_DIRS: &[(&str, &str)] = &[
    (
        "file-history",
        "Snapshots of files Claude changed, used to restore checkpoints of past sessions.",
    ),
    ("plans", "Plan files written in plan mode."),
    ("paste-cache", "Contents of large pastes."),
    ("debug", "Per-session debug logs."),
    (
        "shell-snapshots",
        "Shell settings captured for each session.",
    ),
    ("session-env", "Per-session environment details."),
    (
        "image-cache",
        "Images attached in older Claude Code versions.",
    ),
    ("uploads", "Files attached to Remote Control sessions."),
    ("usage-data", "Reports and cached analysis from /insights."),
    ("todos", "Left over from older Claude Code versions."),
    ("statsig", "Left over from older Claude Code versions."),
    ("logs", "Left over from older Claude Code versions."),
];

/// Never offered, whatever their size. `projects/<p>/memory/` is handled
/// separately.
pub const NEVER_TOUCH: &[&str] = &[
    "settings.json",
    "settings.local.json",
    "CLAUDE.md",
    "agents",
    "commands",
    "skills",
    "plugins",
    "rules",
    "output-styles",
    "workflows",
    "themes",
    "keybindings.json",
    "agent-memory",
    ".credentials.json",
    "history.jsonl",
    "backups",
    "sessions",
    "jobs",
    "daemon",
    "tasks",
    "feedback",
    "feedback-bundles",
];

#[derive(Debug, Default)]
pub struct ClaudeCode;

/// What we learn from the head and tail of a transcript.
#[derive(Debug, Default)]
struct SessionMeta {
    session_id: Option<String>,
    cwd: Option<PathBuf>,
    first_prompt: Option<String>,
    started: Option<String>,
    version: Option<String>,
    recognised: bool,
    parsed: usize,
}

/// Text of a user message, or `None` for tool results and injected text.
fn prompt_text(v: &Value) -> Option<String> {
    if v.get("type")?.as_str()? != "user" {
        return None;
    }
    if v.get("isMeta").and_then(Value::as_bool) == Some(true)
        || v.get("isSidechain").and_then(Value::as_bool) == Some(true)
    {
        return None;
    }
    let content = v.get("message")?.get("content")?;
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => return None,
    };
    let trimmed = text.trim();
    // Slash commands, command output and reminders are wrapped in tags.
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return None;
    }
    Some(trimmed.to_owned())
}

fn read_meta(path: &Path) -> SessionMeta {
    let mut m = SessionMeta::default();
    m.parsed = ai::scan_head(path, |v| {
        if v.get("type").and_then(Value::as_str).is_some()
            && (v.get("sessionId").is_some() || v.get("uuid").is_some())
        {
            m.recognised = true;
        }
        if m.cwd.is_none() {
            m.cwd = v
                .get("cwd")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from);
        }
        if m.session_id.is_none() {
            m.session_id = v
                .get("sessionId")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        if m.version.is_none() {
            m.version = v.get("version").and_then(Value::as_str).map(str::to_owned);
        }
        if m.started.is_none() && v.get("type").and_then(Value::as_str) == Some("user") {
            m.started = v
                .get("timestamp")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        if m.first_prompt.is_none() {
            m.first_prompt = prompt_text(v);
        }
        !(m.cwd.is_some() && m.first_prompt.is_some() && m.version.is_some())
    });
    m
}

/// Session id from a transcript file name: `<uuid>.jsonl`, or a set-aside
/// copy `<uuid>.orphaned-….jsonl` / `<uuid>.jsonl.superseded-…`.
fn session_file(path: &Path) -> Option<(String, bool)> {
    let name = file_name(path);
    let primary = name.ends_with(".jsonl") && name.matches('.').count() == 1;
    let set_aside = name.contains(".jsonl.superseded-")
        || (name.ends_with(".jsonl") && name.contains(".orphaned-"));
    if !(primary || set_aside) {
        return None;
    }
    let id = name.split('.').next()?.to_owned();
    (!id.is_empty()).then_some((id, set_aside))
}

impl ClaudeCode {
    /// `CLAUDE_CONFIG_DIR` → `~/.claude`.
    pub fn root(ctx: &Ctx) -> Option<PathBuf> {
        crate::util::env_or_home(&ctx.platform.env, &["CLAUDE_CONFIG_DIR"], ".claude")
    }

    fn running(ctx: &Ctx) -> bool {
        ctx.processes.any_running(PROCESS)
    }

    /// Keep when Claude Code runs and the path changed in the last day.
    fn hold_if_active(ctx: &Ctx, d: &mut ItemDraft, path: &Path) {
        if Self::running(ctx) && ai::modified_within(path, ACTIVE_WINDOW) {
            d.safety = Safety::Keep;
            d.actions.clear();
            d.reasons.insert(
                0,
                ai::tool_running(
                    "Claude Code is running and this changed in the last day. Quit it first, then scan again.",
                ),
            );
        }
    }

    fn session(
        &self,
        ctx: &Ctx,
        project_dir: &Path,
        path: &Path,
        id: &str,
        set_aside: bool,
    ) -> ItemDraft {
        let meta = read_meta(path);
        let companion = project_dir.join(id);
        let mut paths = vec![path.to_path_buf()];
        if !set_aside && companion.is_dir() {
            paths.push(companion.clone());
        }
        let locator = if paths.len() == 1 {
            Locator::path(path)
        } else {
            Locator::Paths { paths }
        };
        let title = meta
            .first_prompt
            .as_deref()
            .and_then(ai::title)
            .unwrap_or_else(|| format!("Claude Code session {}", ai::short_id(id)));
        let mut d = ItemDraft::new(
            if set_aside {
                "claude_code_set_aside_transcript"
            } else {
                "claude_code_session"
            },
            title,
            locator,
        );
        d.category = Some(Category::AiHistory);
        d.safety = Safety::Review;
        d.reasons = ai::chat_reasons();
        d.actions = vec![ActionKind::Quarantine];
        let home = ctx.home();
        d.project_hint = meta
            .cwd
            .clone()
            .filter(|c| c.is_absolute() && home.as_deref() != Some(c.as_path()));
        d.last_used = ai::last_timestamp(path)
            .map(ai::tool_used)
            .or_else(|| mtime_used(path));
        if set_aside {
            d.notes.push(
                "Claude Code set this older copy of the transcript aside; it doesn't appear in the session picker."
                    .to_owned(),
            );
        }
        if (meta.parsed > 0 && !meta.recognised) || (meta.parsed == 0 && file_len(path) > 0) {
            d.unknown_format = true;
        }
        d.details = json!({
            "session_id": meta.session_id.unwrap_or_else(|| id.to_owned()),
            "cwd": meta.cwd,
            "started_at": meta.started,
            "claude_version": meta.version,
            "has_subagents": companion.join("subagents").is_dir(),
        });
        Self::hold_if_active(ctx, &mut d, path);
        d
    }

    fn data_dir(ctx: &Ctx, path: &Path, name: &str, about: &str) -> ItemDraft {
        let mut d = ItemDraft::new(
            "claude_code_data",
            format!("Claude Code {}", name.replace('-', " ")),
            Locator::path(path),
        );
        d.category = Some(Category::AiHistory);
        d.safety = Safety::Review;
        d.reasons = vec![
            Reason {
                code: ReasonCode::ContainsUserData,
                text: about.to_owned(),
            },
            kipple_core::safety::reasons::reason(ReasonCode::MayContainSecrets),
        ];
        d.actions = vec![ActionKind::Quarantine];
        d.details = json!({ "folder": name });
        Self::hold_if_active(ctx, &mut d, path);
        d
    }
}

fn file_len(p: &Path) -> u64 {
    p.metadata().map(|m| m.len()).unwrap_or(0)
}

impl Adapter for ClaudeCode {
    fn info(&self) -> IntegrationInfo {
        let mut i = info(
            ID,
            "Claude Code",
            Category::AiHistory,
            "Claude Code's saved conversations and session data. Removing a session means you can't resume or read it again.",
            IntegrationStatus::Stable,
            &[],
            // In-use is per item (sessions changed in the last day while
            // `claude` runs), so the integration declares no blanket process.
            &[],
        );
        i.docs_url = Some("https://code.claude.com/docs/en/claude-directory".to_owned());
        i
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        match Self::root(ctx) {
            Some(root) if root.join("projects").is_dir() => Detection::present(),
            _ => Detection::NotPresent,
        }
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        Self::root(ctx).filter(|r| r.is_dir()).into_iter().collect()
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        let Some(root) = Self::root(ctx) else {
            return Ok(());
        };
        if !root.is_dir() {
            return Ok(());
        }
        let projects = root.join("projects");
        for project_dir in children(&projects).into_iter().filter(|p| p.is_dir()) {
            if ctx.cancel.is_cancelled() {
                return Ok(());
            }
            let mut sessions: BTreeMap<String, Vec<(PathBuf, bool)>> = BTreeMap::new();
            for entry in children(&project_dir) {
                if !entry.is_file() {
                    continue;
                }
                // `memory/` is auto memory, never a session.
                if let Some((id, set_aside)) = session_file(&entry).filter(|(id, _)| id != "memory")
                {
                    sessions.entry(id).or_default().push((entry, set_aside));
                }
            }
            for (id, files) in sessions {
                for (path, set_aside) in files {
                    sink.item(self.session(ctx, &project_dir, &path, &id, set_aside));
                }
            }
        }
        for (name, about) in DATA_DIRS {
            let path = root.join(name);
            if path.is_dir() && std::fs::read_dir(&path).is_ok_and(|mut rd| rd.next().is_some()) {
                sink.item(Self::data_dir(ctx, &path, name, about));
            }
        }
        Ok(())
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        matches!(action, ActionKind::Quarantine).then(|| {
            "Move the whole transcript (and its subagent and tool-output folder) to kipple's quarantine. Claude Code itself deletes transcripts older than cleanupPeriodDays (30 days by default).".to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_session_file_names() {
        let id = "0b5a7c1e-1111-4222-8333-944455556666";
        assert_eq!(
            session_file(Path::new(&format!("/p/{id}.jsonl"))),
            Some((id.to_owned(), false))
        );
        assert_eq!(
            session_file(Path::new(&format!("/p/{id}.jsonl.superseded-1700000000"))),
            Some((id.to_owned(), true))
        );
        assert_eq!(
            session_file(Path::new(&format!("/p/{id}.orphaned-1700-abc.jsonl"))),
            Some((id.to_owned(), true))
        );
        assert_eq!(session_file(Path::new("/p/notes.txt")), None);
    }

    #[test]
    fn prompt_text_skips_tool_results_and_tags() {
        let v: Value = serde_json::from_str(
            r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"x"}]}}"#,
        )
        .unwrap();
        assert_eq!(prompt_text(&v), None);
        let v: Value = serde_json::from_str(
            r#"{"type":"user","message":{"role":"user","content":"<command-name>/clear</command-name>"}}"#,
        )
        .unwrap();
        assert_eq!(prompt_text(&v), None);
        let v: Value = serde_json::from_str(
            r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Fix the login"}]}}"#,
        )
        .unwrap();
        assert_eq!(prompt_text(&v).as_deref(), Some("Fix the login"));
    }
}
