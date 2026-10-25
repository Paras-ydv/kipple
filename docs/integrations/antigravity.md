# antigravity

**Status:** experimental · **Category:** AI history · **Kind:** adapter (`crates/kipple-adapters/src/antigravity.rs`) · **Spec:** INTEGRATIONS.md §4.13

Google Antigravity saves each agent conversation as a file plus a "brain" folder of working files (plans, task lists, artifacts). Since May 2026 there are two apps — the standalone **Antigravity 2.0** app and the **Antigravity IDE** — and each keeps its own data. kipple lists each conversation, flags copies left behind by migrations, and moves conversations to quarantine. It never opens or edits Antigravity's databases.

Experimental: Google doesn't document this storage. Everything below comes from community reports.

## Where it looks

All OSes, under `~/.gemini` (`%USERPROFILE%\.gemini` on Windows):

| Root | Used by |
|---|---|
| `antigravity-ide/` | Antigravity IDE (current) |
| `antigravity/` | Antigravity 2.0 app; before May 2026, the IDE |
| `antigravity-backup/` | Snapshot the 2.0 migration made |

In each root: `conversations/<id>.pb` (and `<id>.db`, verify) and `brain/<id>/`. One item per conversation per root, with the conversation file and its brain folder summed. A brain folder without a conversation file is still listed, with a note. Other folders (`code_tracker/`, `implicit/`, `annotations/`, `context_state/`), `mcp_config.json` and the apps' `state.vscdb` files (the sidebar index, under `Antigravity/User/` and `Antigravity IDE/User/` in the app settings folder) are not offered and not opened.

Titles are "Conversation <first 8 of id> · <date>": the files are binary protobuf, so kipple doesn't read them.

## Migrated copies

When the same conversation id exists in more than one root, a copy is `MIGRATED_COPY` if a more authoritative root (order: `antigravity-ide`, `antigravity`, `antigravity-backup`) has the same conversation with at least as many bytes. It's still `Review`: the 2.0 app reads `antigravity/`, so a flagged copy there may still show up in that app.

## Safety

| Case | Safety | Reasons |
|---|---|---|
| Conversation | `Review` | `CHAT_HISTORY`, `MAY_CONTAIN_SECRETS` (+ `MIGRATED_COPY`) |
| Any `Antigravity*` process running (both apps and their helpers) | `Keep` | `TOOL_RUNNING` |
| Written in the last 10 minutes | `Keep` | `RECENTLY_WRITTEN` |

## Actions

- `quarantine` only — the conversation file and its brain folder move together; `kipple undo` restores them. No compaction, no database edits.
- Antigravity's sidebar index isn't updated, so the app may list a quarantined conversation until you remove it there (or restore it).

## Verified

| Fact | Source | Checked |
|---|---|---|
| `~/.gemini/antigravity` = 2.0 app, `antigravity-ide` = IDE, `antigravity-backup` = migration snapshot (May 18, 2026 rebrand) | [DEV: "Antigravity is Dead. Long Live Antigravity."](https://dev.to/turingsoracle/antigravity-is-dead-long-live-antigravity-186m) (May 20, 2026), [DEV: migrate to the new Antigravity IDE](https://dev.to/mohamadawwaad/how-to-migrate-your-conversation-history-to-the-new-antigravity-ide-macos-a85) (May 21, 2026) | 2026-10-25 |
| `conversations/` and `brain/` in both roots; `%APPDATA%\Antigravity\User\…` (2.0) and `%APPDATA%\Antigravity IDE\User\…` (IDE) | [Google AI forum, backup/restore for AG 2.0 and AG IDE](https://discuss.ai.google.dev/t/sync-ag2-0-and-ag-ide-chat-history-between-different-computers-backup-and-restore-method/175279) (Jul 18, 2026) | 2026-10-25 |
| `conversations/*.pb`, sidebar index in `state.vscdb`, `brain/` holds per-conversation artifacts | [Google AI forum 124337](https://discuss.ai.google.dev/t/i-have-lost-conversation-history-in-the-with-early-update-app/124337?page=5), [134342](https://discuss.ai.google.dev/t/how-can-we-read-conversations-that-we-ve-had-with-our-antigravity-assistants-from-other-programs/134342) | 2026-10-25 |
| `.db` conversation files | (verify) — accepted, not confirmed | — |
| Process names `Antigravity`, `Antigravity IDE` | app names from the sources above (verify on a real machine) | — |
