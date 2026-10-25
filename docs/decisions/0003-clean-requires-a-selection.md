# 0003 — `kipple clean` requires a selection until the TUI picker exists

**Status:** accepted · 2026-10-26 · answers SPEC §22 Q4 (for now)

## Decision
`kipple clean` with no item ids and no filter (`--safe-only`, `--only`, `--older-than`, `--min-size`, `--root`) exits with a usage error (code 2) and a hint. This applies to TTY and non-TTY. When the TUI picker ships, a TTY run without a selection opens it; non-TTY keeps requiring a selection.
