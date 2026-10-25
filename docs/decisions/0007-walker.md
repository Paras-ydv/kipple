# 0007 — `ignore::WalkParallel` is the walker

**Status:** accepted · 2026-10-26 · answers SPEC §22 Q1

## Context
SPEC §22 asked to benchmark `jwalk` against `ignore::WalkParallel` and pick by p50 scan time. Both were implemented behind the `Walker` trait and run through the full scan path on a 532k-entry generated tree (`docs/perf.md`).

## Result
p50 differed by under 2% in both scan and walk-only modes, and the winner changed between runs.

## Decision
Use `ignore::WalkParallel`. With speed equal, it wins on everything else: `.kippleignore` (gitignore syntax) is built in, while jwalk needed a hand-written ignore chain in `process_read_dir`; it is maintained as part of ripgrep; and it is one fewer dependency, since `ignore`'s gitignore matcher is used anyway for `requires_gitignored`.

The jwalk implementation was removed after the benchmark. The `Walker` trait stays, so a different walker can be benchmarked again later with the same harness.
