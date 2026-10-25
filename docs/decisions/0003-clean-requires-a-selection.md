# 0003 — `kipple clean` without a selection

**Status:** accepted · 2026-10-26 · answers SPEC §22 Q4 · updated the same day when the TUI landed

## Decision
- In a terminal, `kipple clean` with no item ids and no filter opens the TUI on the latest scan (scanning first if there is none), where items are picked and planned.
- Outside a terminal (pipes, scripts, `--json`), it exits with a usage error (code 2) and a hint to pass item ids or a filter (`--safe-only`, `--only`, `--older-than`, `--min-size`, `--root`).
