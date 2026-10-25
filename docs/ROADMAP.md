# kipple — Roadmap & Build Plan

Work top to bottom. Each milestone is sized for a few focused sessions and ends in something runnable. A milestone is **done** only when every box is checked, CI is green on macOS, Linux and Windows, and docs are updated.

Spec references: `SPEC.md §n`, `INTEGRATIONS.md §n`, `UI.md §n`.

---

## MVP

### M0 — Repository foundation
- [x] Cargo workspace with crates `kipple-core`, `kipple-packs`, `kipple-adapters`, `kipple-cli` (binary `kipple`), `kipple-mcp`, `kipple-share`, and `xtask` (SPEC §3.1)
- [x] `rust-version` (MSRV) set; `rustfmt.toml`, `clippy.toml`, `deny.toml`
- [x] Dual license files, `README.md` stub (name, pronunciation, one-line pitch, "pre-alpha"), `CONTRIBUTING.md` stub, `SECURITY.md`, `CODE_OF_CONDUCT.md`
- [x] `.github/workflows/ci.yml`: fmt, clippy `-D warnings`, test, cargo-deny — matrix macOS/Linux/Windows
- [x] Issue templates (bug, integration request, path report, false-safety report) and labels list in `docs/labels.md`
- [x] Test helper `TestHome` that creates a temp home and sets `HOME`, `USERPROFILE`, `XDG_*`, `APPDATA`, `LOCALAPPDATA`, `CODEX_HOME`, `CLAUDE_CONFIG_DIR`, `ANDROID_HOME`, etc. (SPEC §18)
- **Done when:** `cargo test` passes on all OSes and `kipple --version` prints.

### M1 — Core model, platform, config, index
- [x] Types from SPEC §4 with serde + JSON Schema export (`cargo xtask schema` → `docs/schema/`)
- [x] `platform`: OS/WSL detection, path expansion (`~`, `$VAR`, `%VAR%`, `{var}`), safe canonicalization, command runner (argv, timeout), process list via `sysinfo`, trash wrapper (SPEC §16)
- [x] Directories + config loading/validation with "did you mean" for unknown keys (SPEC §10.1, §10.3); `kipple config path|show|validate`
- [x] SQLite index with migrations (SPEC §10.2)
- [x] Protected-locations module + `is_actionable(path)` with property tests (SPEC §7.4, §8.3)
- **Done when:** config round-trips, index migrates from empty, protected-path property tests pass.

### M2 — Scanner, pack engine, sizes, projects
- [ ] Pack TOML schema + loader + validation (`INTEGRATIONS.md §2`); packs embedded at build time from `packs/`
- [ ] Parallel walker with skip rules, `.kippleignore`, `CACHEDIR.TAG`, cancellation, progress events (SPEC §5.1–5.2)
- [ ] Honest sizes: apparent / on_disk / reclaimable with hardlink accounting (SPEC §5.5) — test with a pnpm-style hardlinked fixture
- [ ] Project detection with all markers (SPEC §5.4)
- [ ] First packs: `node-modules`, `js-build`, `python-venv`, `python-caches`, `rust-target`, `gradle-project`, `maven-project`, `xcode-project`, `cachedir-tag`
- [ ] Cache packs: `npm-cache`, `pnpm-store`, `yarn-cache`, `bun-cache`, `pip-cache`, `uv-cache`, `cargo-registry`, `go-build-cache`, `go-mod-cache`, `gradle-cache`, `homebrew-cache`, `puppeteer`
- [ ] Fixtures + snapshot tests for every pack
- [ ] Benchmark harness on a generated 500k-entry fixture (SPEC §5.3)
- **Done when:** `kipple scan --json` on fixtures produces stable snapshots; benchmark recorded in `docs/perf.md`.

### M3 — Attributor, safety engine, read-only CLI
- [ ] Attributor: nearest project, git last commit via `gix`, source mtime sampling, orphan projects (SPEC §6)
- [ ] Safety engine with evaluation order, reason codes and texts, in-use process checks, recent-write checks (SPEC §7)
- [ ] Commands: `scan`, `report`, `projects`, `items`, `explain` with global filters and `--json` (SPEC §11)
- [ ] Human output exactly per SPEC §11.3 (tables, units, ages, words for safety, `NO_COLOR`)
- [ ] Test: every read-only command leaves fixture mtimes untouched (SPEC §21.5)
- **Done when:** a real-machine scan reads well and every item has ≥ 1 reason.

### M4 — Plans, execution, quarantine, undo
- [ ] Planner (pure) + plan files + confirmation rules (SPEC §8.1–8.2)
- [ ] Executor with per-entry revalidation, journal, cancellation (SPEC §8.3, §8.5)
- [ ] Quarantine with same-volume rename, manifests, restore, purge, cross-volume → trash (SPEC §8.4)
- [ ] Commands: `clean` (filters + `--plan-only`, `--plan <id>`), `undo`, `quarantine list|restore|purge`, `history`
- [ ] Native command actions for packs with `measure = before-after`
- [ ] Tests: undo restores byte-identical trees (checksums); `--yes` rejected without `--safe-only`; Review plans require typing `reclaim`
- **Done when:** clean → undo round-trip works on fixtures on all three OSes.

### M5 — Platform adapters
- [ ] `docker` + `docker-desktop-disk` (report) (`INTEGRATIONS.md §4.5`) — integration test against a real Docker service on Linux CI
- [ ] `android-avd` + `android-system-images` (§4.6)
- [ ] `xcode-derived-data` + `ios-device-support` (§4.7, macOS only)
- [ ] `playwright` (§4.8)
- [ ] `ollama`, `huggingface` (§4.14–4.15)
- **Done when:** each adapter has fixtures, snapshots, action tests and a docs page.

### M6 — AI history adapters
- [ ] `claude-code` (§4.10)
- [ ] `codex` (§4.11) — native delete only; mock the `codex` binary in tests
- [ ] `cursor` (§4.12) — read-only stats by key prefix, backup-copy item, guarded VACUUM, orphan workspaceStorage
- [ ] `antigravity` (§4.13) — experimental, quarantine-only, migrated-copy detection
- [ ] Secret masking for titles (simple rules now; full scanner in v1)
- [ ] `kipple ai` views (per tool, per project, sessions with filters)
- **Done when:** fixtures for each tool's format exist (generated, fake content), and no adapter opens third-party DBs writable during scan (asserted in tests).

### M7 — TUI
- [ ] Screens per `UI.md §4`: Overview, Projects, Categories, AI history, Plan/Review, Quarantine
- [ ] Streaming results during scan, keybindings, filter, sort, selection, explain panel
- [ ] Theme module with color + monochrome + ASCII fallback; works at 80×24
- [ ] Snapshot tests of rendered frames (ratatui `TestBackend`)
- **Done when:** a new user can scan → select → clean → undo without reading docs.

### M8 — Doctor, share card, docs generation
- [ ] `kipple doctor` (SPEC §11.2) with actionable fixes
- [ ] `kipple share` card per `UI.md §5` (SPEC §14), deterministic snapshot test
- [ ] `kipple completions`
- [ ] `cargo xtask docs`: integrations table for README + `docs/integrations/<id>.md` pages
- **Done when:** README integrations table is generated, share card PNG matches the design.

### M9 — Release & launch
- [ ] `cargo-dist` config: targets, installers, checksums, GitHub Releases
- [ ] `homebrew-tap` repo wired to cargo-dist
- [ ] crates.io publish (`kipple` + library crates)
- [ ] README final: demo GIF (record with `vhs`), install, quickstart, "What kipple will never touch", integrations table, comparison, contributing
- [ ] `CONTRIBUTING.md`: "Add an integration in 15 minutes" walkthrough
- [ ] 20+ `good first issue` integration tickets filed from `INTEGRATIONS.md §1` v1 items
- [ ] Launch checklist in `docs/launch.md` (Show HN post draft, Reddit posts, share-card examples)
- **Done when:** `brew install <org>/tap/kipple && kipple scan` works on a clean Mac; acceptance criteria SPEC §21 all pass.

---

## v1

### M10 — MCP server
`kipple mcp` with tools per SPEC §13, two-step plan/apply with confirm tokens, `destructiveHint`, tests with an MCP client harness. Docs page for adding kipple to Claude Code / Codex / Cursor.

### M11 — Desktop app (Tauri)
App per SPEC §15 and `UI.md §3`: Overview, Projects + detail, Categories, AI history, Review plan, Quarantine & history, Settings, tray popover. Signed builds for macOS/Windows, Linux packages.

### M12 — Windows & WSL polish
Windows on-disk sizes (`GetCompressedFileSizeW`), Restart Manager in-use detection, `wsl-disk` compaction, Windows-side scanning from WSL, winget + Scoop.

### M13 — Keep-clean
Retention rules + `kipple auto`, scheduled runs (launchd/systemd/Task Scheduler helper `kipple auto install`), low-disk alerts, growth timeline views, "what grew since last scan".

### M14 — Advanced actions
Archive/restore (tar+zstd), relocate with symlink, `set_retention` (Claude Code), Docker Desktop guided compaction, secret scanner (`kipple ai secrets`), session export to Markdown.

### M15 — Coverage expansion
Toolchains/version managers (§1.3), Podman/OrbStack/Colima/Lima/kind/minikube, simulators/runtimes, more AI tools, IDE caches, git worktrees/stale clones, system opt-ins, community `packs update` with signed bundles.

---

## Later
WASM plugin adapters, duplicate-dependency analysis and pnpm migration hints, `restore-deps`, VS Code extension, Raycast/Alfred extension, i18n, CI-runner mode, opt-in anonymous "GB reclaimed" counter.
