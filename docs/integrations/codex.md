# codex

**Status:** stable · **Category:** AI history · **Kind:** adapter (`crates/kipple-adapters/src/codex.rs`) · **Spec:** INTEGRATIONS.md §4.11

OpenAI Codex (CLI, desktop app and IDE extension) saves every session as a "rollout" file and keeps thread metadata in a SQLite database. kipple lists each session, attributes it to the folder it ran in, and removes sessions **only** by running Codex's own `codex delete`. kipple never deletes or moves rollout files itself: that would leave threads behind in Codex's database.

## Where it looks

| What | Path (all OSes; `~` is `%USERPROFILE%` on Windows) |
|---|---|
| Root | `$CODEX_HOME`, else `~/.codex` |
| Sessions | `<root>/sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl` or `.jsonl.zst` (compressed) |
| Archived sessions | `<root>/archived_sessions/rollout-…` |
| Thread metadata | `$CODEX_SQLITE_HOME` (else the root) `/state_<n>.sqlite`, newest `n` (currently `state_5.sqlite`), table `threads` |

The database is opened read-only with `immutable=1`; a test asserts that a scan creates no `-wal`, `-shm` or journal files and changes no mtimes. `session_index.jsonl` (may be stale) is not used.

From `threads` kipple reads only `id`, `cwd`, `name`, `title`, `preview`, `first_user_message`, `source`, `archived`, `created_at(_ms)`, `updated_at(_ms)`, using whichever of those columns exist (they were added over many migrations). `id`, `rollout_path` and `cwd` must exist; otherwise the database counts as an unknown format and every session is `Keep`.

For a session missing from the database, kipple reads the head of a plain rollout: the first line must be `{"type":"session_meta","payload":{…"cwd"…}}`, and the first `event_msg` of type `user_message` gives the title. Compressed rollouts are only understood through the database.

Titles: thread `name`, else `title`, else `preview`, else the first prompt — secret-masked, one line, 60 characters. Fallback: "Codex session …<last 8 of id>" (ids are UUIDv7, so the start is a shared timestamp).

## Safety

| Case | Safety | Reasons |
|---|---|---|
| Session, `codex delete` available | `Review` | `CHAT_HISTORY`, `MAY_CONTAIN_SECRETS` (+ `ORPHANED_PROJECT`) |
| `codex` missing or too old to have `delete --force` | `Keep` (report-only) | `NATIVE_TOOL_MISSING` |
| Any process named `codex`/`Codex` running (CLI, app, IDE extension) | `Keep` | `TOOL_RUNNING` |
| Rollout grows between two size checks 1 s apart | `Keep` | `ACTIVE_WRITER` |
| Written in the last 10 minutes | `Keep` | `RECENTLY_WRITTEN` |
| Rollout or database in a shape kipple doesn't know | `Keep` | `UNKNOWN_FORMAT` |

Sessions over 100 MB get a note: most of that is usually saved tool output.

## Actions

- `native:codex.delete` → `codex delete <uuid> --force`. Support is detected at runtime by running `codex delete --help` and looking for `--force`. `--force` skips Codex's prompt and only accepts a UUID, which kipple validates first. Before running, kipple re-checks that Codex is closed and the rollout isn't growing. If the command succeeds but the rollout is still there, the entry is reported as failed.
- Deleting is permanent; there's no undo. `codex archive` only hides a session from Codex's lists and frees nothing, so kipple doesn't offer it.

## Gotchas

- Since 2026 Codex can migrate rollouts to a paginated, compressed form (`.jsonl.zst`) with part of the history in `thread_history_1.sqlite`. `codex delete` cleans up both; deleting the file by hand would not.
- `history.jsonl` (your prompt history), `config.toml`, `auth.json` and other files in the root are never offered.

## Verified

| Fact | Source | Checked |
|---|---|---|
| `codex delete <SESSION>`, `--force` (UUID only, no prompt), `codex archive` doesn't delete | [Codex CLI reference](https://learn.chatgpt.com/docs/developer-commands?surface=cli) (redirect from developers.openai.com/codex/cli/reference); `codex-rs/cli/src/main.rs` `DeleteCommand` at tag `rust-v0.164.0` | 2026-10-25 |
| Latest release `rust-v0.164.0` (2026-10-23) | GitHub releases, openai/codex | 2026-10-25 |
| `state_5.sqlite`, `CODEX_SQLITE_HOME` | `codex-rs/state/src/sqlite.rs`, `codex-rs/state/src/lib.rs` | 2026-10-25 |
| `threads` columns | `codex-rs/state/migrations/0001_threads.sql`, `0007`, `0025`, `0032`, `0041` | 2026-10-25 |
| Rollout file names, `sessions/` + `archived_sessions/`, `.jsonl.zst`, thread id = last 36 chars of the stem | `codex-rs/thread-store/src/local/rollout_migration.rs` at `rust-v0.164.0` | 2026-10-25 |
| Rollout line `{timestamp, type, payload}`, `session_meta` payload with `id`, `cwd`, `originator`, `cli_version`, `source`; `user_message` event | `codex-rs/history/src/rollout_payload.rs`, `codex-rs/protocol/src/protocol.rs` (`SessionMeta`, `UserMessageEvent`) at `rust-v0.164.0` | 2026-10-25 |
| Process names of the desktop app and IDE extension host | not checked on a real machine (verify); kipple matches `codex` case-insensitively | — |
