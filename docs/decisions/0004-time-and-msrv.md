# 0004 — `jiff` for time; MSRV is the lowest the dependencies allow

**Status:** accepted · 2026-10-26

## Decision
- Time: `jiff` (0.2.x as of 2026-10). Timestamps serialize as RFC 3339 strings. Relative durations (`30d`, `6m`) are parsed by kipple's own small parser so the CLI grammar stays ours.
- MSRV: `rust-version` is set to the lowest version that builds the workspace with its dependencies, re-checked when dependencies change. CI has an `msrv` job that builds with exactly that version.
