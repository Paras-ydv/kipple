# 0010 — AI history adapters: what reality changed in the spec

**Status:** accepted · 2026-10-25 · M6

## Context

INTEGRATIONS.md §4.10–4.13 were written before the tools' current storage was checked. While building M6 we verified each format against official docs and source (links and dates in `docs/integrations/<id>.md`). Some details differ from the spec. This records how kipple handles them.

## Decisions

### Claude Code (§4.10)

- **A session is the transcript plus its companion folder.** Current Claude Code writes `projects/<p>/<session>/` next to `<session>.jsonl` (subagent transcripts, spilled tool output, MCP task state) and deletes both together. kipple's session item uses a `Paths` locator with both, so quarantine and undo move them as one unit and sizes are honest.
- **Set-aside copies are separate items.** `<session>.jsonl.superseded-<ts>` and `<session>.orphaned-<ts>-<suffix>.jsonl` are older transcripts Claude Code put aside. They are listed on their own, `Review`, with a note.
- **Auto memory is never offered.** `projects/<p>/memory/` sits next to the transcripts but is user knowledge, not history.
- **In-use is per item.** The spec keeps only sessions changed in the last 24 h while `claude` runs. A blanket `in_use_processes = ["claude"]` would make every Claude item `Keep` (and the executor would skip all of them), so the adapter applies the rule in discovery and declares no blanket process. The executor still re-checks the 10-minute rule before acting; the 24-hour rule is evaluated at scan time (plans are valid for an hour).
- **`cleanupPeriodDays = 0`** is now rejected by Claude Code's settings validation. The v1 retention action must still never write `0`.

## Consequences

- Snapshot tests cover each rule; `docs/integrations/claude-code.md` lists the verified sources.
- If Claude Code changes these layouts, the snapshot tests and the "verified" tables are the places to update.
