# claude-code

**Status:** stable · **Category:** AI history · **Kind:** adapter (`crates/kipple-adapters/src/claude_code.rs`) · **Spec:** INTEGRATIONS.md §4.10

Claude Code keeps every conversation as a plain-text transcript, plus snapshots, logs and caches it writes while you work. kipple lists each session as its own item, attributes it to the project it ran in, and moves whole transcripts to quarantine when you ask. It never edits a transcript.

## Where it looks

| OS | Root |
|---|---|
| macOS, Linux | `$CLAUDE_CONFIG_DIR`, else `~/.claude` |
| Windows | `$CLAUDE_CONFIG_DIR`, else `%USERPROFILE%\.claude` |

Inside the root:

| Path | Item |
|---|---|
| `projects/<encoded-cwd>/<session-id>.jsonl` | One item per session. Includes the session's companion folder `projects/<encoded-cwd>/<session-id>/` (subagent transcripts, `tool-results/`, `mcp-tasks/`) when it exists, so the session moves as one unit. |
| `projects/<encoded-cwd>/<session-id>.jsonl.superseded-<ts>`, `<session-id>.orphaned-<ts>-<suffix>.jsonl` | One item each ("set-aside transcript"): older copies Claude Code put aside. |
| `file-history/`, `plans/`, `paste-cache/`, `debug/`, `shell-snapshots/`, `session-env/`, `image-cache/`, `uploads/`, `usage-data/`, `todos/`, `statsig/`, `logs/` | One item per non-empty folder, `Review`. |

kipple reads the session's project from the `cwd` field inside the transcript. It never decodes the folder name. A session whose `cwd` no longer exists shows up under a deleted project (`ORPHANED_PROJECT`).

Titles come from the first prompt you typed (slash commands, tool results and injected `<…>` blocks are skipped), flattened to one line, secret-masked and cut to 60 characters. Nothing else from a transcript is stored. kipple reads at most the first 400 lines / 4 MB of a transcript for metadata and the last 64 KB for the newest `timestamp`.

## Never touched

`settings.json`, `settings.local.json`, `CLAUDE.md`, `agents/`, `commands/`, `skills/`, `plugins/`, `rules/`, `output-styles/`, `workflows/`, `themes/`, `keybindings.json`, `agent-memory/`, `.credentials.json`, `history.jsonl`, `backups/` (copies of `~/.claude.json`), `sessions/`, `jobs/`, `daemon/`, `tasks/`, `feedback*/`, and every project's auto memory `projects/<p>/memory/`. `~/.claude.json` lives outside the root and is never looked at. A test asserts none of these become items.

## Safety

| Item | Safety | Reasons |
|---|---|---|
| Session | `Review` | `CHAT_HISTORY`, `MAY_CONTAIN_SECRETS` (+ `ORPHANED_PROJECT`) |
| Data folder | `Review` | `CONTAINS_USER_DATA` (with what the folder holds), `MAY_CONTAIN_SECRETS` |
| Transcript kipple can't read | `Keep` | `UNKNOWN_FORMAT` |

In use:

- While a `claude` process runs, sessions and folders modified in the last 24 hours are `Keep` (`TOOL_RUNNING`). Older ones are unaffected, so you can clean old sessions from inside a running Claude Code. (The integration declares no blanket in-use process for this reason.)
- Anything written in the last 10 minutes is `Keep` (`RECENTLY_WRITTEN`), at scan time and again right before acting.

## Actions

- `quarantine` — moves the transcript and its companion folder into kipple's quarantine; `kipple undo` puts them back byte for byte.
- `archive` and `set_retention` (`cleanupPeriodDays`) are v1.

Claude Code deletes transcripts and most of the data folders above itself once they are older than `cleanupPeriodDays` (default 30, minimum 1; `0` is rejected by Claude Code's settings validation). `explain` mentions this.

## Gotchas

- `claude purge <path>` (Claude Code's own command) deletes a project's transcripts, auto memory and history lines for good. kipple's quarantine is the undoable alternative.
- The 24-hour in-use rule is evaluated at scan time. Plans are valid for an hour after a scan; right before acting kipple re-checks the 10-minute rule.
- Transcripts are append-only JSONL. Lines whose `type` field kipple doesn't know are ignored; a transcript with no recognisable line is `Keep`.

## Verified

| Fact | Source | Checked |
|---|---|---|
| Root, `CLAUDE_CONFIG_DIR`, Windows `%USERPROFILE%\.claude` | [code.claude.com/docs/en/settings](https://code.claude.com/docs/en/settings), [claude-directory](https://code.claude.com/docs/en/claude-directory) | 2026-10-25 |
| Transcript paths, companion `<session>/subagents|tool-results|mcp-tasks`, `.superseded-` / `.orphaned-` copies, data folders and what they hold, files to keep, auto memory location | [code.claude.com/docs/en/claude-directory § Application data](https://code.claude.com/docs/en/claude-directory#application-data) | 2026-10-25 |
| `cleanupPeriodDays` default 30, minimum 1, `0` fails validation | same page | 2026-10-25 |
| Line fields `type`, `uuid`, `parentUuid`, `timestamp` (ISO 8601), `sessionId`, `cwd`, `gitBranch`, `version`; `message.content` is a string or an array of blocks; tool results are `tool_result` blocks | [claude-dev.tools JSONL format](https://claude-dev.tools/docs/jsonl-format) (community, undated) and the fixture format | 2026-10-25 |
| `isMeta` / `isSidechain` flags on lines | not in official docs (verify); kipple only uses them to skip lines | — |
