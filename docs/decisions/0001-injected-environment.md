# 0001 — Environment is injected, not read from the process

**Status:** accepted · 2026-10-26

## Context
SPEC §18 asks tests to point `HOME`, `XDG_*`, `CODEX_HOME` and friends at a temp dir. Mutating the process environment is `unsafe` in Rust 2024 and races between tests that run in parallel.

## Decision
The core reads environment variables only through `platform::Env`, a snapshot captured once with `Env::from_process()` by the binaries. Tests build an `Env` with `testing::TestHome`, which points every known variable into a temp dir, clears variables that could lead to real data (`GOCACHE`, `npm_config_cache`, …) and sets `PATH` to an empty `bin/` so no real tool runs. CLI tests pass the same variables to the child process with `env_clear()`.

`TestHome` also sets `KIPPLE_TEST_SANDBOX`; debug builds refuse filesystem changes outside that directory when it is set.

## Consequences
Tests run in parallel safely. Any code path that calls `std::env::var` directly is a bug.
