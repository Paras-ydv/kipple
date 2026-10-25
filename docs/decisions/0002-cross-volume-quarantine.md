# 0002 — Items on another volume go to the OS trash

**Status:** accepted · 2026-10-26 · answers SPEC §22 Q3

## Context
Quarantine moves are atomic renames, which only work on the same volume as kipple's data dir. SPEC §8.4 allowed a per-volume `<volume-root>/.kipple-quarantine/` when writable.

## Decision
Items on a different volume always use `Trash`. kipple never creates hidden folders on other volumes. If the trash is unavailable (headless Linux), the entry is dropped from the plan with a warning.

## Consequences
`kipple undo` cannot restore trashed entries; the plan and the journal say so, and the user restores them from the OS trash. SPEC §8.4 is updated to match.
