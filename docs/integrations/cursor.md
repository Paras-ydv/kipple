# cursor

**Status:** experimental · **Category:** AI history · **Kind:** adapter (`crates/kipple-adapters/src/cursor.rs`) · **Spec:** INTEGRATIONS.md §4.12

Cursor is a VS Code fork. Its chats live in one SQLite database, `state.vscdb`, which can grow to tens of gigabytes. kipple reports what's in it, offers a guarded compaction, and lists the database's backup copy, data for workspaces whose folder you deleted, and agent transcripts. kipple never edits or deletes rows.

Experimental because Cursor doesn't document its storage; key names and the transcript folder are known from Cursor staff forum replies and community tools.

## Where it looks

| OS | Cursor `User` folder |
|---|---|
| macOS | `~/Library/Application Support/Cursor/User` |
| Linux | `$XDG_CONFIG_HOME/Cursor/User` (default `~/.config/Cursor/User`) |
| Windows | `%APPDATA%\Cursor\User` |

| Item | Path | Safety | Action |
|---|---|---|---|
| 1. Database backup | `User/globalStorage/state.vscdb.backup` | `Review` (`BACKUP_COPY`) | `quarantine` |
| 2. Chat database | `User/globalStorage/state.vscdb` | `Review` (`FREE_PAGES`) when ≥ 256 MB is unused, else report-only | `compact:sqlite_vacuum` |
| 3. Workspace data for a deleted folder | `User/workspaceStorage/<hash>/` whose `workspace.json` `folder` (or `workspace`) URI no longer exists | `Review` (`ORPHANED_PROJECT`) | `quarantine` |
| 4. Agent transcripts | `~/.cursor/projects/<encoded-path>/agent-transcripts/` (verify) | `Review` (`CHAT_HISTORY`) | `quarantine` |

Item 2's size shows the file size; its reclaimable size is only the free pages. `details` has bytes per `cursorDiskKV` key prefix (`bubbleId`, `composerData`, `agentKv`, `checkpointId`, other), computed with `SUM(length(value))` on a read-only, `immutable=1` connection. A test asserts a scan leaves no `-wal`, `-shm` or journal file and changes no mtimes. A database without VS Code's `ItemTable` is `UNKNOWN_FORMAT`.

Remote workspaces (`vscode-remote://…`) are skipped. Folders for item 4 are never decoded into paths, so those items have no project.

## In use

Cursor running (process `Cursor`) → every Cursor item is `Keep` (`TOOL_RUNNING`). The spec names items 1–3; kipple also keeps item 4 while Cursor runs, the stricter choice. The database modified in the last 10 minutes is `Keep`.

## Compaction (`compact:sqlite_vacuum`)

Right before acting, kipple checks that Cursor is closed, that there's still ≥ 256 MB of free pages, and that the volumes holding the database and kipple's backups folder each have at least **twice** the database size free (a backup copy plus SQLite's own working copy during `VACUUM`; Cursor staff note compaction can need up to 2×). Then it:

1. removes kipple's own Cursor backups older than 7 days from `<data>/backups/`,
2. copies the database (and its `-wal` file, if any) to `<data>/backups/cursor-<timestamp>.vscdb`,
3. runs `VACUUM`,
4. runs `PRAGMA integrity_check` and reports a failure (with the backup path) unless it says `ok`.

No chats are removed. To remove old chats, use Cursor's command palette: **Developer: Delete Old Chats**, then **Developer: GC Agent KV Blobs**, quit Cursor and scan again.

On Windows kipple can't measure free space yet, so it refuses to compact.

## Gotchas

- Deleting chats in Cursor's UI alone doesn't shrink the file; compaction does.
- Never delete `state.vscdb` itself; Cursor staff warn chats can end up stuck on "Loading Chat…". kipple doesn't offer that.
- Cursor makes `state.vscdb.backup` on a clean exit when the database is healthy, and uses it to recover a damaged database. Removing it is safe while the main database is healthy; Cursor writes a new one next time it closes.

## Verified

| Fact | Source | Checked |
|---|---|---|
| `globalStorage/state.vscdb` on macOS, key prefixes `bubbleId:`, `agentKv:`, `composerData:`, `checkpointId:`, 30 GB growth (Cursor 3.14.7) | [Cursor forum, staff replies Aug 2026](https://forum.cursor.com/t/state-vscdb-grows-to-30gb-due-to-bubbleid-agentkv-entries/167641) | 2026-10-25 |
| "Developer: Delete Old Chats" (deletes then vacuums), "Developer: GC Agent KV Blobs", don't delete `state.vscdb` | [Cursor forum 158574](https://forum.cursor.com/t/how-to-remove-local-archived-chats/158574/4), [167141](https://forum.cursor.com/t/developer-delete-old-chats-claims-175-deleted-but-gc-reports-0-deleted-chats-reappear-after-reload-wsl2/167141) | 2026-10-25 |
| Compaction needs up to ~2× the database size free | [Cursor forum 167641](https://forum.cursor.com/t/state-vscdb-grows-to-30gb-due-to-bubbleid-agentkv-entries/167641), [168926](https://forum.cursor.com/t/cursor-database-too-large-state-vscdb/168926) | 2026-10-25 |
| `workspace.json` with `{ "folder": uri }` or `{ "workspace": uri }`; `ItemTable`; `.backup` copy made on close; optional WAL mode | VS Code source: `src/vs/platform/storage/electron-main/storageMain.ts`, `src/vs/base/parts/storage/node/storage.ts` (main) | 2026-10-25 |
| `~/.cursor/projects/<encoded-cwd>/agent-transcripts/<id>/<id>.jsonl` (older: `*.txt`) | community: [Cursor forum 162757](https://forum.cursor.com/t/agents-sidebar-empty-222-transcripts-still-on-disk-windows/162757), [codeburn docs](https://raw.githubusercontent.com/getagentseal/codeburn/main/docs/providers/cursor-agent.md) (verify) | 2026-10-25 |
| Linux and Windows `User` paths | VS Code conventions for a product named "Cursor" (verify on a real machine) | — |
