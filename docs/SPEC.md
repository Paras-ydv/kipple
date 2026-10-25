# kipple — Product & Technical Specification

> **kipple** /ˈkɪp.əl/ — *rhymes with ripple.* The junk that piles up on its own.
>
> kipple finds the disk space your development tools quietly eat — dependency folders, build output, package caches, Docker, emulators, simulators, model stores and AI-agent history — tells you what each item is, which project it belongs to and whether it is safe to remove, and reclaims it with an undo.

This document is the source of truth for **what** to build. `docs/INTEGRATIONS.md` specifies every integration, `docs/UI.md` specifies the interfaces, and `docs/ROADMAP.md` orders the work. When they disagree, this file wins; fix the other file.

Keywords: **MUST** = required for the milestone it belongs to; **SHOULD** = expected unless there is a written reason; **MAY** = optional. Tags: **[MVP]** first public release, **[v1]** follow-up releases, **[later]** after traction.

---

## 1. Product

### 1.1 Problem

Developer machines lose 50–300 GB to regenerable or forgotten data spread over dozens of locations, each with its own rules:
dependency folders in old projects, global package caches, Docker's virtual disk, Android emulators and system images, Xcode DerivedData and simulators, browser binaries for test runners, ML models, and — newest and fastest-growing — AI coding agents' local history (Codex session logs and Cursor's `state.vscdb` have both been reported at 90+ GB).

Generic disk analyzers show *where* bytes are but not *what they are* or *whether they can go*. Existing dev cleaners cover one ecosystem, one OS, or are CLI-only, and none treat AI-agent history safely.

### 1.2 What kipple is

One Rust core, used by three interfaces (CLI/TUI, desktop app, MCP server), with every source of junk implemented as a pluggable **integration**. kipple:

1. **Finds** items across the whole machine, fast.
2. **Explains** each item: what it is, which project owns it, when it was last used, how much space deleting it actually frees.
3. **Judges** each item: `Safe`, `Review` or `Keep`, always with a human-readable reason.
4. **Reclaims** using the safest available action (the tool's own command, quarantine with undo, compaction, archive) — only after the user confirms.
5. **Keeps it clean** with optional retention rules and alerts.

### 1.3 Principles (non-negotiable)

1. **Read-only by default.** No command modifies anything unless the user explicitly asks for an action and confirms the plan. `scan`, `report`, `explain`, `doctor` never write outside kipple's own data directory.
2. **Every recommendation has a reason.** No item is ever shown with a safety level but no explanation.
3. **Honest sizes.** Report what deleting an item actually frees, not just its apparent size (hardlinks, shared stores, sparse disk images).
4. **Prefer the owner's tool.** If the tool that created the data has a supported cleanup command (`docker`, `codex delete`, `npm cache clean`, `avdmanager`), use it. Raw deletion is the fallback.
5. **Undo by default.** Deletions go to kipple's quarantine (or the OS trash) and can be restored for 7 days.
6. **Refuse when unsure.** Unknown storage format version, unreadable metadata, item in use, path outside scan roots → the item is `Keep` and no action is offered.
7. **Everything is an integration.** The core has no knowledge of npm, Docker or Cursor. New coverage never requires core changes.
8. **No telemetry, no network** (except talking to local daemons such as Docker, and the explicit `kipple packs update` command). Works fully offline.
9. **No sudo** by default. System-level locations are reported, never touched, unless a future integration explicitly supports a privileged action behind a separate flag.

### 1.4 Non-goals

- Not a general system cleaner (no "optimize RAM", no registry cleaning, no malware scanning).
- Not a backup tool. Archive is a convenience, not a backup strategy.
- Never deletes source code, documents, `.git` directories or anything not positively identified by an integration.
- No cloud accounts, sync or paid tiers.

### 1.5 Users

- **Primary:** individual developers on macOS, Linux and Windows/WSL who hit "disk almost full".
- **Secondary:** people using AI coding agents heavily (Claude Code, Codex, Cursor, Antigravity) whose history stores grow without bound.
- **Tertiary:** AI agents themselves, via the MCP server ("free up 40 GB safely").

---

## 2. Glossary

| Term | Meaning |
|---|---|
| **Integration** | A unit of knowledge about one source of junk (e.g. `npm-cache`, `docker`, `codex`). Either a **pack** or an **adapter**. |
| **Pack** | A declarative integration written in TOML under `packs/`. Covers path-based sources. |
| **Adapter** | An integration written in Rust implementing the `Adapter` trait. Covers structured sources (APIs, SQLite, JSONL, manifests). |
| **Item** | One reclaimable thing reported by an integration: a folder, a file, a Docker image, a chat session, an AVD. |
| **Project** | A source-code directory identified by marker files (`package.json`, `Cargo.toml`, …) or a git root. Items are attributed to projects. |
| **Scan** | One run that discovers items. Persisted in the index. |
| **Plan** | An ordered list of (item, action) the user is asked to approve. Serializable, has an id. |
| **Action** | What to do with an item: `quarantine`, `trash`, `native`, `compact`, `archive`, `relocate`, `set_retention`. |
| **Quarantine** | kipple's holding area. Items moved here can be restored until purged (default 7 days). |
| **Journal** | Append-only log of every action executed, with outcome. |
| **Safety** | `Safe` (regenerates automatically / no user data), `Review` (probably fine, user should look), `Keep` (protected, in use, or unknown). |

---

## 3. Architecture

### 3.1 Repository layout (Cargo workspace, monorepo)

```
kipple/
├─ Cargo.toml                  # workspace
├─ crates/
│  ├─ kipple-core/             # model, scanner, attributor, safety, planner, executor, index, config, platform
│  ├─ kipple-packs/            # pack (TOML) loader + generic pack engine
│  ├─ kipple-adapters/         # Rust adapters, one module per integration
│  ├─ kipple-cli/              # `kipple` binary: CLI + TUI (ratatui)
│  ├─ kipple-mcp/              # MCP server (library; exposed as `kipple mcp`)
│  └─ kipple-share/            # share-card renderer (SVG template → PNG)
├─ app/                        # [v1] Tauri 2 desktop app (Svelte 5 + TypeScript)
├─ packs/                      # built-in packs, *.toml, embedded into the binary at build time
├─ fixtures/                   # synthetic trees + sample files for every integration
├─ xtask/                      # dev tasks: fixture generation, schema export, docs generation
├─ docs/
└─ .github/
```

### 3.2 Dependency rules

- `kipple-core` depends on no UI, no async runtime, no integration crate.
- `kipple-packs` and `kipple-adapters` depend on `kipple-core` only.
- `kipple-cli`, `kipple-mcp`, `app/src-tauri` depend on core + packs + adapters.
- Anything that needs async (Docker via `bollard`, MCP server) owns a private Tokio runtime behind a sync facade. The core API is synchronous and uses `rayon` for parallelism.
- The **only** way interfaces talk to the core is the public API in §3.4. The CLI, MCP and desktop app MUST NOT reach into internals. This guarantees every interface behaves identically.

### 3.3 Recommended crates

| Purpose | Crate |
|---|---|
| CLI parsing | `clap` (derive) + `clap_complete` |
| TUI | `ratatui` + `crossterm` |
| Parallel walk | `jwalk` (or `ignore::WalkParallel`, pick one and document why) |
| Parallelism | `rayon` |
| Serialization | `serde`, `serde_json`, `toml` |
| SQLite | `rusqlite` with `bundled` feature |
| Docker | `bollard` |
| OS dirs | `directories` |
| Trash | `trash` |
| Processes | `sysinfo` |
| Git metadata | `gix` (read-only, minimal features) |
| Hashing ids | `blake3` |
| Globs | `globset` |
| Archive | `tar` + `zstd` |
| Time | `jiff` or `time` (pick one) |
| Errors | `thiserror` (libraries), `anyhow` (binaries) |
| Logging | `tracing` + `tracing-subscriber` |
| Share card | `resvg` + `usvg` + `tiny-skia`, embedded font |
| MCP | `rmcp` (official Rust MCP SDK) |
| Tests | `insta`, `tempfile`, `assert_cmd`, `predicates`, `proptest` |
| Release | `cargo-dist` |

### 3.4 Core public API

```rust
pub struct Kipple { /* config, registry, index handle */ }

impl Kipple {
    pub fn open(opts: OpenOptions) -> Result<Self>;                      // loads config, packs, adapters, index
    pub fn integrations(&self) -> Vec<IntegrationInfo>;                   // id, name, kind, detected?, version
    pub fn scan(&self, req: ScanRequest, events: &dyn ScanSink) -> Result<ScanResult>;
    pub fn items(&self, query: ItemQuery) -> Result<Vec<Item>>;           // from latest scan in index
    pub fn projects(&self, query: ProjectQuery) -> Result<Vec<ProjectSummary>>;
    pub fn explain(&self, item: &ItemId) -> Result<Explanation>;
    pub fn plan(&self, req: PlanRequest) -> Result<Plan>;                 // pure, no side effects
    pub fn execute(&self, plan: &Plan, confirm: Confirmation, events: &dyn ExecSink) -> Result<ExecReport>;
    pub fn quarantine(&self) -> Result<Vec<QuarantineEntry>>;
    pub fn restore(&self, entry: &QuarantineId) -> Result<RestoreReport>;
    pub fn purge(&self, req: PurgeRequest) -> Result<PurgeReport>;
    pub fn history(&self, req: HistoryRequest) -> Result<Vec<SizePoint>>; // growth timeline
    pub fn doctor(&self) -> Result<DoctorReport>;
}
```

- `ScanSink` / `ExecSink` receive progress events (`Started`, `Progress{done,total?,bytes}`, `ItemFound`, `Warning`, `Finished`) and support cancellation via a shared `CancelToken`.
- Every public type is `Serialize + Deserialize` and has a JSON Schema exported by `cargo xtask schema` into `docs/schema/`. The CLI `--json` output, MCP responses and the Tauri bridge all use these exact types.

### 3.5 Data flow

```
Registry ──loads──► integrations (packs + adapters)
   │
Scanner ── walks roots once, dispatches entries to pack matchers;
   │       adapters run their own discovery in parallel
   ▼
Attributor ── assigns items to projects, computes last_used
   ▼
Safety engine ── assigns Safety + reasons (hard rules first, then integration rules)
   ▼
Index (SQLite) ── persists scan, items, size history
   ▼
Planner ── user selection → Plan (actions chosen per item)
   ▼
Executor ── re-validates each item, runs the action, writes Journal
```

---

## 4. Data model

### 4.1 Item

```rust
pub struct Item {
    pub id: ItemId,                    // blake3(integration_id + "\0" + canonical_locator), hex, first 16 bytes
    pub integration: IntegrationId,    // e.g. "node-modules", "docker", "codex"
    pub category: Category,            // ProjectArtifact | PackageCache | Toolchain | Container | Mobile | AiHistory | Model | IdeCache | BuildCache | Logs | System
    pub kind: String,                  // integration-specific, e.g. "node_modules", "docker_image", "codex_session"
    pub title: String,                 // human label: "node_modules", "postgres:16 image", "Fix auth flow (Codex session)"
    pub locator: Locator,              // Path(PathBuf) | Paths(Vec<PathBuf>) | Docker{kind,id} | Logical{store, key}
    pub size: Size,
    pub project: Option<ProjectRef>,
    pub last_used: Option<LastUsed>,
    pub safety: Safety,
    pub reasons: Vec<Reason>,          // at least one
    pub actions: Vec<ActionKind>,      // allowed actions, preferred first; empty when Safety::Keep
    pub details: serde_json::Value,    // integration-specific extras (image tags, session first prompt, AVD api level…)
    pub notes: Vec<String>,            // e.g. "3.1 GB is shared with the pnpm store and will not be freed"
}

pub struct Size {
    pub apparent: u64,                 // sum of file lengths
    pub on_disk: u64,                  // allocated blocks (st_blocks*512 on Unix; GetCompressedFileSizeW on Windows [v1])
    pub reclaimable: u64,              // what deleting frees, after hardlink/shared-store accounting (§5.5)
    pub file_count: Option<u64>,
}

pub enum Safety { Safe, Review, Keep }

pub struct Reason {
    pub code: ReasonCode,              // stable machine code, see §7.3
    pub text: String,                  // one sentence, plain English, no jargon
}

pub struct LastUsed { pub at: Timestamp, pub source: LastUsedSource } // GitCommit | SourceMtime | ItemMtime | ToolMetadata | Atime
pub struct ProjectRef { pub id: ProjectId, pub root: PathBuf, pub name: String }
```

### 4.2 Project

```rust
pub struct ProjectSummary {
    pub id: ProjectId,                 // blake3(canonical root)
    pub root: PathBuf,
    pub name: String,                  // package name if known, else folder name
    pub markers: Vec<String>,          // "package.json", "Cargo.toml", ".git", …
    pub last_used: Option<LastUsed>,
    pub total: Size,                   // sum of attributed items
    pub by_category: BTreeMap<Category, u64>, // reclaimable bytes
    pub safety_mix: SafetyMix,         // bytes per Safety
    pub exists: bool,                  // false = orphaned (e.g. AI sessions for a deleted folder)
}
```

### 4.3 Plan, actions, outcomes

```rust
pub struct Plan {
    pub id: PlanId,                    // ulid
    pub created_at: Timestamp,
    pub scan_id: ScanId,
    pub entries: Vec<PlanEntry>,
    pub totals: PlanTotals,            // bytes by safety, by action
    pub warnings: Vec<String>,
}
pub struct PlanEntry { pub item: Item, pub action: ActionKind, pub preflight: Vec<Check> }

pub enum ActionKind {
    Quarantine,                        // move into kipple quarantine (same volume) — default for paths
    Trash,                             // OS trash, used when quarantine would cross volumes
    Native { command_id: String },     // integration-defined command, e.g. "docker.image.remove", "codex.delete"
    Compact { method: String },        // "sqlite_vacuum", "docker_desktop_disk" [v1], "wsl_vhdx" [v1]
    Archive,                           // [v1] tar+zstd to archive dir, then quarantine original
    Relocate { target: PathBuf },      // [v1] move to another volume + symlink back
    SetRetention { key: String, value: serde_json::Value }, // [v1] edit a tool's own setting
}

pub enum Outcome { Done{freed: u64}, Skipped{reason: String}, Failed{error: String} }
```

### 4.4 JSON stability

All JSON output carries `"schema_version": 1`. Additive changes keep the version; breaking changes bump it and are listed in `CHANGELOG.md`.

---

## 5. Scanner

### 5.1 Roots

- **Default roots:** the user's home directory, plus integration-declared absolute locations (e.g. Android SDK root from `ANDROID_HOME`, Docker via API).
- `--root <path>` (repeatable) replaces the home-directory root. Integration locations are still scanned unless `--only` filters them out.
- MUST NOT cross into: other users' homes, `/System`, `/usr` (except integration-declared paths), `/proc`, `/sys`, `/dev`, Windows `C:\Windows`, network mounts (unless `scan.include_network = true`), and kipple's own data directory.
- MUST NOT follow symlinks by default. When `--follow-symlinks` is set, never leave the scan roots.
- On macOS, skip `~/Library` during the general walk except for paths integrations declare explicitly (avoids TCC permission prompts and noise).

### 5.2 Walk

- One parallel walk over the roots. Each directory entry is offered to the **pack matcher set** (precompiled `globset` + marker checks).
- When a directory matches an artifact rule (e.g. `node_modules` with a sibling `package.json`), the scanner records the item, computes its size in a parallel sub-task, and **does not descend further** for matching purposes.
- Skip descending into: `.git` internals, `node_modules` of an already-matched item, directories tagged with `CACHEDIR.TAG` (they become items of the generic `cachedir-tag` integration), and paths in `.kippleignore` files (gitignore syntax) or `scan.exclude` globs.
- Adapters run concurrently with the walk on their own (they usually look at known locations, not the whole tree).
- Permission errors are collected as warnings (shown once, summarized), never fatal.
- Cancellation MUST stop work within 250 ms.

### 5.3 Performance targets

Measured on an Apple-silicon or recent x86 laptop with NVMe, home dir ≈ 1M entries:

- Cold first scan: **≤ 30 s** to complete; first items visible in ≤ 2 s.
- Warm rescan with index: **≤ 5 s** [v1] (re-stat known items, re-walk only directories whose mtime changed).
- Memory: ≤ 300 MB peak.
- Benchmarks live in `crates/kipple-core/benches` and run in CI (non-blocking) on fixture trees.

### 5.4 Project detection

A directory is a **project root** if it contains any marker: `package.json`, `Cargo.toml`, `pyproject.toml`, `setup.py`, `requirements.txt`, `Pipfile`, `go.mod`, `build.gradle(.kts)`, `settings.gradle(.kts)`, `pom.xml`, `*.xcodeproj`, `*.xcworkspace`, `Package.swift`, `pubspec.yaml`, `*.csproj`, `*.sln`, `CMakeLists.txt`, `mix.exs`, `stack.yaml`, `*.cabal`, `build.sbt`, `Gemfile`, `composer.json`, `deno.json(c)`, `ProjectSettings/ProjectVersion.txt` (Unity), `*.uproject`, `main.tf`, `MODULE.bazel`/`WORKSPACE`, `build.zig` — or a `.git` directory/file.
Nested projects (monorepo packages) roll up to the **outermost git root** for the Projects view, while keeping the inner package as `subproject` in `details`.

### 5.5 Size accounting (honest sizes)

- `apparent` = Σ `len`. `on_disk` = Σ allocated blocks.
- Hardlinks: track `(dev, inode)` across the **whole scan**. A file with `nlink > 1` counts toward an item's `reclaimable` only if **all** its links were observed inside that same item; otherwise it contributes 0 and the item gets a note: *"2.4 GB of this is hard-linked from the pnpm store and won't be freed"*.
- Sparse files / disk images (e.g. `Docker.raw`): report `apparent` (logical size) and `on_disk`; `reclaimable` comes from the integration (e.g. Docker API's reclaimable numbers), never from the file size.
- APFS clones cannot be detected cheaply; document this as a known limitation in `docs/`, show `on_disk` as the upper bound.
- Totals in every view use `reclaimable`. Item rows show `reclaimable` with `apparent` on hover/`explain` when they differ by more than 10%.

---

## 6. Attributor

- Assign each item to the nearest enclosing project root (by path), or to a project referenced in tool metadata:
  - AI sessions: `cwd` recorded in the session file / database.
  - Docker: `com.docker.compose.project.working_dir` label on containers/volumes/images built by compose.
  - Xcode DerivedData: `info.plist` → `WorkspacePath`.
  - Android AVD: no project (shown under "Global").
- `last_used` for a project = max of: last git commit time (via `gix`, HEAD commit), newest mtime among top-level source files sampled (≤ 2,000 files, skipping artifact dirs), and `.git/index` mtime. Record which source won.
- Orphaned projects: when metadata points to a directory that no longer exists, create a `ProjectSummary{exists:false}`. These are highlighted ("sessions for 6 projects you've deleted").

---

## 7. Safety engine

### 7.1 Evaluation order

1. **Hard rules** (cannot be overridden by packs or config). Any match → `Keep`, `actions = []`:
   - Path is outside scan roots or integration-declared locations.
   - Path is, or is inside, a protected location (§7.4).
   - Path is a `.git` directory, or a directory containing tracked source files (heuristic: contains a `.git` or a project marker at its own level *and* is not the artifact the rule names).
   - The integration reported `UnknownFormat` (storage version not recognised).
   - The item is **in use** (§7.2) and the action would touch it.
   - The current user cannot write to the item's parent (would need sudo).
2. **Integration rule** — the pack/adapter's default safety and reasons.
3. **Age rule** (project artifacts only): if the owning project was used within `safety.recent_days` (default 14), downgrade `Safe` → `Review` with reason `RECENTLY_USED`.
4. **User overrides** from config (`[[safety.override]]` by integration id, kind or glob) — can only make things *stricter* unless `allow_relax = true` is set explicitly on the override.

### 7.2 In-use detection

- [MVP] Process check via `sysinfo`: integrations declare process names that make their data in-use (`Cursor`, `codex`, `claude`, `Docker Desktop`, `qemu-system-*` for emulators, `java` with `GradleDaemon` in args, `ollama`…). If such a process runs, affected items are `Keep` with reason `TOOL_RUNNING` and the text tells the user what to quit.
- [MVP] Recently written: item modified within the last 10 minutes → `Keep` (`RECENTLY_WRITTEN`). Session files growing between two stats 1 s apart → `Keep` (`ACTIVE_WRITER`).
- [MVP] Docker: images used by any container (running or stopped) are never offered for removal; volumes mounted by running containers are `Keep`.
- [v1] Open-file detection: Linux `/proc/*/fd`, macOS `proc_pidinfo`/`lsof` fallback, Windows Restart Manager API.

### 7.3 Reason codes (stable)

`REGENERABLE`, `REDOWNLOADABLE`, `INACTIVE_PROJECT`, `RECENTLY_USED`, `RECENTLY_WRITTEN`, `ACTIVE_WRITER`, `TOOL_RUNNING`, `IN_USE_BY_CONTAINER`, `UNUSED_BY_ANY_EMULATOR`, `USED_BY_EMULATOR`, `CONTAINS_USER_DATA`, `CHAT_HISTORY`, `MAY_CONTAIN_SECRETS`, `LARGE_REDOWNLOAD`, `ORPHANED_PROJECT`, `MIGRATED_COPY`, `BACKUP_COPY`, `FREE_PAGES`, `PROTECTED_PATH`, `OUTSIDE_ROOTS`, `UNKNOWN_FORMAT`, `NEEDS_ELEVATION`, `NATIVE_TOOL_MISSING`, `SHARED_HARDLINKS`, `PINNED_BY_PROJECT`, `EXPERIMENTAL` (integration not yet verified; capped at Review), `USER_OVERRIDE` (a config override set the level), `CONTAINS_SOURCE` (folder holds source or a repository).
Each code has a default English sentence template with placeholders (`{days}`, `{project}`, `{tool}`), stored in `kipple-core/src/safety/reasons.rs`. Integrations MAY override the text.

### 7.4 Protected locations (always `Keep`)

- **Fully protected (subtree):** `~/.ssh`, `~/.gnupg`, `~/.aws/credentials`, `~/.config/gcloud`, `~/.kube/config`, other credential files, macOS Keychains, iCloud Drive, system folders, kipple's own config/data/quarantine, and any path listed in `protect = [...]` in config.
- **Personal folders:** `~/Documents`, `~/Desktop`, `~/Pictures`, `~/Movies`, `~/Music`, `~/Videos`, `~/Downloads`. The folder itself and anything not positively identified are `Keep`; project artifacts and build caches inside them (and items of opt-in integrations such as `old-installers` the user enabled) follow the normal rules (decision 0005).
- **Exact:** every scan root and the home folder itself.
- Any path that *contains* a protected location is `Keep`.

### 7.5 Defaults by category

| Category | Default |
|---|---|
| Project artifacts (deps, build output) | `Safe` if project inactive ≥ 14 days, else `Review` |
| Package caches | `Safe` (`REDOWNLOADABLE`) |
| Old toolchain versions | `Review` unless no project pins them [v1] |
| Docker build cache, dangling images | `Safe` |
| Unused tagged images | `Review` |
| Stopped containers, unused volumes | `Review` (`CONTAINS_USER_DATA`) |
| Android system image not used by any AVD | `Safe` |
| Android AVD | `Review` (has user data / snapshots) |
| Xcode DerivedData, iOS DeviceSupport | `Safe` |
| Xcode Archives | `Review` |
| AI agent history | `Review` (`CHAT_HISTORY`, `MAY_CONTAIN_SECRETS`) — never `Safe` |
| ML models | `Review` (`LARGE_REDOWNLOAD`) |
| `.backup` copies of app databases | `Review` (`BACKUP_COPY`) |
| Database compaction | `Review` (`FREE_PAGES`), tool must be closed |

---

## 8. Planner and executor

### 8.1 Planning

- Input: a selection (item ids, or a filter like `--safe-only --older-than 30d --category package-cache`) and optional action overrides.
- For each item choose the first allowed action in `item.actions` unless overridden. If quarantine would cross volumes, switch to `Trash`; if trash is unavailable, the entry is dropped with a warning (never fall back to permanent delete silently).
- Group entries: `Safe` first, then `Review`. `Keep` items can never enter a plan.
- Plans are pure data, saved to `<data>/plans/<id>.json`, and can be printed, edited (`--exclude <id>`), and executed later by id (valid for 1 hour or until a newer scan).

### 8.2 Confirmation

- CLI: show the plan (totals + grouped list), then require typing `y` (Safe-only plans) or the word `reclaim` (plans containing `Review` items). `--yes` is accepted only together with `--safe-only`.
- MCP: a two-step protocol — `plan` returns a `confirm_token`; `apply` requires it (§13).
- App: a dedicated Review screen with an explicit primary button stating the exact bytes and the undo window.

### 8.3 Execution guarantees

For **each** entry, immediately before acting:

1. Re-stat the item; verify the path still exists and its identity (dev+inode for directories, Docker id for Docker) matches the scan. Mismatch → `Skipped`.
2. Re-run in-use checks. In use → `Skipped`.
3. Re-check hard safety rules.
4. Run the action. Record outcome + freed bytes in the journal **before** moving on.

Other guarantees:
- Sequential per integration, parallel across integrations (configurable `exec.parallelism`, default 2).
- Ctrl-C / cancel finishes the current entry, then stops. The journal shows what happened.
- Never `rm -rf` a path that resolves (after canonicalization) to a protected location — assert this in code with a dedicated function covered by property tests.

### 8.4 Quarantine

- Location: `<data>/quarantine/`. Only items on the **same volume** as the data dir are quarantined. Items on another volume use `Trash` (decision 0002); kipple never creates folders on other volumes.
- Layout: `<quarantine>/<ulid>/payload/…` + `<quarantine>/<ulid>/manifest.json` (original path, item snapshot, integration, timestamp, size).
- Move = atomic rename (fast, same volume). Never copy+delete for quarantine.
- `kipple undo` restores the most recent action batch; `kipple restore <id>` restores a single entry; restoring refuses if the original path exists again (offer `--to <path>`).
- Purge: entries older than `quarantine.keep_days` (default 7) are purged automatically at the start of the next run that performs an action, and by `kipple quarantine purge`. Quarantine size is shown in `report` so users understand space is freed only after purge — **and** the CLI offers `--purge-now` to free immediately (with an extra confirmation).

### 8.5 Journal

- `<data>/journal.jsonl`, append-only, one line per entry: `{ts, plan_id, item_id, integration, action, outcome, freed, original_path}`.
- `kipple history` prints it; `--json` exports it.

---

## 9. Integrations framework

Details and the full catalog are in `docs/INTEGRATIONS.md`. Summary:

### 9.1 Packs (TOML)

TOML (not YAML) because the Rust TOML stack is first-class and stable, contributors already know it from `Cargo.toml`, and it has no implicit-typing surprises.

```toml
id = "npm-cache"
name = "npm cache"
category = "package-cache"
description = "npm's download cache. Safe to remove; npm re-downloads packages when needed."
detect.any_command = ["npm"]          # optional: only active if one exists on PATH
detect.any_path = ["{npm_cache}"]

[vars]
npm_cache.macos   = "~/.npm/_cacache"
npm_cache.linux   = "~/.npm/_cacache"
npm_cache.windows = "%LOCALAPPDATA%/npm-cache/_cacache"
npm_cache.env     = "npm_config_cache"   # env override, appended with /_cacache

[[locations]]
path = "{npm_cache}"
kind = "npm_cache"
title = "npm download cache"
safety = "safe"
reasons = ["REDOWNLOADABLE"]
actions = ["native:npm-cache-clean", "quarantine"]

[commands.npm-cache-clean]
run = ["npm", "cache", "clean", "--force"]
timeout_secs = 300
```

Project-artifact packs use `[[artifacts]]` instead of `[[locations]]`:

```toml
[[artifacts]]
dir = "node_modules"
requires_sibling_any = ["package.json"]
kind = "node_modules"
safety = "safe"                       # age rule downgrades to review for recent projects
reasons = ["REGENERABLE"]
restore_hint = "npm install"          # shown in explain; used by `kipple restore-deps` [later]
actions = ["quarantine"]
```

The full schema (all fields, variable syntax, OS keys, matchers, version constraints) is defined in `INTEGRATIONS.md §2` and enforced by a JSON Schema exported with `cargo xtask schema`; CI validates every pack.

### 9.2 Adapters (Rust)

```rust
pub trait Adapter: Send + Sync {
    fn info(&self) -> IntegrationInfo;                                   // id, name, category, docs url
    fn detect(&self, ctx: &Ctx) -> Detection;                            // NotPresent | Present{version} | Unsupported{reason}
    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<()>;// emit items (no side effects)
    fn preflight(&self, ctx: &Ctx, item: &Item, action: &ActionKind) -> Vec<Check>;
    fn apply(&self, ctx: &Ctx, item: &Item, action: &ActionKind) -> Result<Outcome>;
    fn in_use_processes(&self) -> &[&str] { &[] }
}
```

- `Ctx` gives access to config, platform layer (paths, env, process list, command runner with timeouts), the cancel token, and a **read-only** SQLite opener (`open_readonly_immutable`) — adapters MUST open third-party databases read-only during discovery.
- Adapters register in `kipple-adapters/src/lib.rs` via a static list. Each lives in its own module with its own fixture directory and tests.

### 9.3 Generic integrations

- `cachedir-tag`: any directory containing a valid `CACHEDIR.TAG` (Cache Directory Tagging Specification signature) is a `Safe` cache item, attributed to its project if inside one.
- `large-files` [v1]: files ≥ `large_files.min_size` (default 1 GB) not claimed by any other integration, `Review`, report-only by default.

---

## 10. Index, config and directories

### 10.1 Directories (layout of the `directories` crate, qualifier `dev`, org `kipple`, app `kipple`; resolved from the injected environment so tests can sandbox it — decision 0001)

| | macOS | Linux | Windows |
|---|---|---|---|
| Config | `~/Library/Application Support/dev.kipple.kipple/config.toml` *and* `~/.config/kipple/config.toml` (XDG path wins if it exists) | `$XDG_CONFIG_HOME/kipple/config.toml` | `%APPDATA%\kipple\config\config.toml` |
| Data (index, journal, plans, quarantine) | `~/Library/Application Support/dev.kipple.kipple/` | `$XDG_DATA_HOME/kipple/` | `%LOCALAPPDATA%\kipple\data\` |

`KIPPLE_CONFIG` and `KIPPLE_DATA_DIR` env vars override. All of these are protected locations.

### 10.2 Index (SQLite, WAL mode)

```sql
CREATE TABLE scans (id TEXT PRIMARY KEY, started_at INTEGER, finished_at INTEGER, roots TEXT, kipple_version TEXT, status TEXT);
CREATE TABLE items (scan_id TEXT, id TEXT, integration TEXT, category TEXT, kind TEXT, title TEXT,
                    locator TEXT, apparent INTEGER, on_disk INTEGER, reclaimable INTEGER,
                    project_id TEXT, last_used INTEGER, safety TEXT, json TEXT,
                    PRIMARY KEY (scan_id, id));
CREATE TABLE projects (scan_id TEXT, id TEXT, root TEXT, name TEXT, last_used INTEGER, exists_ INTEGER, json TEXT,
                       PRIMARY KEY (scan_id, id));
CREATE TABLE size_points (ts INTEGER, scope TEXT, key TEXT, reclaimable INTEGER); -- scope: total|category|integration|project
CREATE INDEX items_by_integration ON items(scan_id, integration);
```

- Keep the last 10 scans' items; keep `size_points` forever (they are tiny) to draw growth timelines.
- Migrations via a `schema_version` pragma and embedded SQL files.

### 10.3 Config (`config.toml`)

```toml
[scan]
roots = ["~"]                       # default
exclude = ["~/Library/Mobile Documents", "**/huge-dataset/**"]
include_network = false
follow_symlinks = false

[safety]
recent_days = 14
protect = ["~/work/client-x"]

[[safety.override]]
integration = "docker"
kind = "docker_volume"
safety = "keep"

[quarantine]
keep_days = 7

[integrations]
disable = ["old-installers"]
enable = ["large-files"]

[retention]                           # [v1] keep-clean rules, evaluated by `kipple auto` / the app
[[retention.rule]]
name = "stale node_modules"
match = { integration = "node-modules", project_inactive_days = 60 }
action = "quarantine"
[[retention.rule]]
name = "old codex sessions"
match = { integration = "codex", older_than_days = 45 }
action = "native"                     # still requires confirmation unless `auto_confirm = true` and safety == safe

[ui]
units = "si"                          # si (GB) | iec (GiB)
color = "auto"
```

`kipple config path|show|edit|validate`. Unknown keys are an error with a suggestion (`did you mean …`).

---

## 11. CLI specification

Binary name: `kipple`. Running `kipple` with no arguments in a TTY opens the TUI on the Overview screen after a scan; in a non-TTY it prints `kipple --help`.

### 11.1 Global flags

`--json` (machine output, no colors, no prompts), `--root <path>`… , `--only <integration|category>`… , `--exclude <glob>`… , `--older-than <dur>` (e.g. `30d`, `6m`), `--min-size <size>` (e.g. `500MB`), `--safe-only`, `--no-color`, `-q/--quiet`, `-v/--verbose` (repeatable), `--config <path>`.

### 11.2 Commands

| Command | Behaviour |
|---|---|
| `kipple scan` | Scan and print the summary report (table grouped by category, top projects, totals by safety). Saves to index. |
| `kipple report` | Re-print the latest scan's report without scanning. `--by project|category|integration`, `--tree`. |
| `kipple projects` | Projects view: name, path, last used, reclaimable, breakdown. `--orphaned`. |
| `kipple items` | Flat list with filters; `--sort size|age|name`; prints item ids. |
| `kipple explain <item-id\|path>` | Everything about one item: what it is, why this safety, actions available, sizes (apparent/on-disk/reclaimable), restore hint. |
| `kipple clean` | Interactive: scan (or use latest), select (TUI picker or filters), show plan, confirm, execute. `--plan-only` writes a plan and prints its id. `--plan <id>` executes a saved plan. |
| `kipple undo` | Restore the last executed batch from quarantine. |
| `kipple quarantine list\|restore <id>\|purge [--older-than]` | Manage quarantine. |
| `kipple history` | Journal of executed actions; `kipple history --growth` prints growth over time per category. |
| `kipple archive <item-id…>` [v1] | Compress to `archive.dir`, then quarantine originals. `kipple archive list|restore <id>`. |
| `kipple ai` | Shortcut view for AI history: per tool, per project, per session. `kipple ai sessions --tool codex --older-than 30d`, `kipple ai secrets` [v1], `kipple ai export <id> --md` [v1]. |
| `kipple doctor` | For each integration: detected? version? locations found? permissions? native tool available? Prints actionable fixes (e.g. "grant Full Disk Access to your terminal to measure Docker's disk image"). |
| `kipple packs list\|show <id>\|validate <file>\|test <id>` | Inspect and test packs. `kipple packs update` [v1] fetches the latest signed community pack bundle. |
| `kipple share` | Render the share card PNG from the latest scan (and the last clean, if any) to `./kipple-card.png`; `--open`; `--no-numbers-by-project`. |
| `kipple mcp` | Run the MCP server on stdio. |
| `kipple config …` | See §10.3. |
| `kipple completions <shell>` | bash, zsh, fish, powershell, elvish. |
| `kipple auto` [v1] | Evaluate retention rules once (for cron/launchd/systemd timers). Never acts on `Review` items without `auto_confirm`. |

### 11.3 Human output rules

- Sizes: SI units by default (`12.4 GB`), 3 significant digits; right-aligned in tables.
- Ages: relative (`3 days`, `7 months`).
- Safety shown as words, not only colors: `safe`, `review`, `keep` (colors per `UI.md §2.3`; respects `NO_COLOR`). Table headers in sentence case, dimmed.
- Plans always state two numbers separately: freed right away (native commands, trash) and held in quarantine (freed after `quarantine.keep_days` or `--purge-now`). Never imply quarantined bytes are already free.
- Paths shortened with `~`. Never print secrets (session content) in tables — only titles derived from the first user prompt, truncated to 60 chars, with obvious secrets masked.
- Example (`kipple scan`):

```
kipple · scanned 1.02M files in 18.4s                         reclaimable 184 GB

Category            Items   Reclaimable      Safe    Review
Containers             31       61.2 GB   22.1 GB   39.1 GB
AI history            412       38.9 GB         —   38.9 GB
Package caches         14       27.5 GB   27.5 GB         —
Project artifacts      96       24.0 GB   18.7 GB    5.3 GB
Mobile                  9       21.6 GB    8.2 GB   13.4 GB
Models                  6       10.8 GB         —   10.8 GB

Heaviest projects           Last used    Reclaimable
~/work/nomad-app            7 months         14.2 GB   docker 8.9, deps 3.1, ai 2.2
~/play/llm-bench            2 months          9.7 GB   venv 6.0, models 3.7
…
Next: kipple clean --safe-only   reclaims 76.5 GB (33.5 GB of it can be undone for 7 days)
```

### 11.4 Exit codes

`0` success · `1` error · `2` usage error · `3` partial success (some entries failed/skipped) · `4` aborted by user · `130` interrupted.

---

## 12. TUI (ratatui) [MVP]

Screens: **Overview**, **Projects**, **Categories**, **AI history**, **Plan/Review**, **Quarantine**. Full layout, keybindings and visual rules in `docs/UI.md §4`. Requirements:

- Starts showing results while scanning (streaming), with a progress line.
- Every list supports: `j/k`/arrows, `/` filter, `s` sort cycle, `space` select, `a` select all safe, `enter` drill-in, `e` explain panel, `c` create plan from selection, `?` help, `q` back/quit.
- Works in 80×24 minimum; uses Unicode box drawing with ASCII fallback (`KIPPLE_ASCII=1` or non-UTF-8 locale).
- Color palette defined once (`ui/theme.rs`), with a monochrome mode.

---

## 13. MCP server [v1, behind `kipple mcp`]

Transport: stdio. Server name `kipple`. Tools (JSON in/out use the same schema types):

| Tool | Input | Output | Side effects |
|---|---|---|---|
| `kipple_scan` | `{roots?, only?, min_size?}` | summary + top items | index write only |
| `kipple_items` | `{filter}` | items | none |
| `kipple_projects` | `{filter}` | projects | none |
| `kipple_explain` | `{item_id}` | explanation | none |
| `kipple_plan` | `{item_ids? , filter?, safe_only}` | plan + `confirm_token` + human summary | writes plan file |
| `kipple_apply` | `{plan_id, confirm_token}` | exec report | **destructive** (quarantine/native), marked with MCP `destructiveHint: true` |
| `kipple_undo` | `{}` | restore report | restores quarantine |

Rules: `kipple_apply` refuses plans containing `Review` items unless the plan was created with `safe_only=false` **and** the server was started with `--allow-review`. Tool descriptions instruct the calling agent to show the plan summary to the user and get approval before calling `kipple_apply`.

---

## 14. Share card [MVP]

- 1200×630 PNG (Open Graph size) plus 1080×1350 portrait variant [v1].
- Content: total reclaimable (or freed), the core-sample column of categories, top 3 categories with sizes, the kipple wordmark and the project URL. Before/after variant when a clean happened in the last 24 h. Layout per `UI.md §5`.
- No paths, project names or usernames by default (`--with-projects` opt-in shows project folder names only).
- Rendered from an SVG template in `kipple-share/assets/card.svg` with an embedded OFL font (see UI.md), via `resvg`. Deterministic output (snapshot-tested).

---

## 15. Desktop app [v1]

- Tauri 2, Svelte 5 + TypeScript, Vite. No UI component library; components are built from the design tokens in `docs/UI.md`.
- Backend: `app/src-tauri` calls the core API; commands mirror §3.4 1:1 and return the schema types; progress via Tauri events.
- Screens: Overview, Projects (+ project detail), Categories, AI history, Review plan, Quarantine & history, Settings, plus a menu-bar/tray popover. Specified in `docs/UI.md §3`.
- Signed and notarized on macOS, signed on Windows (MSI + winget), AppImage/deb/rpm on Linux.
- Auto-update via Tauri updater with signature verification, off by default (asked on first run).
- Accessibility: full keyboard navigation, VoiceOver/Narrator labels on all controls, WCAG AA contrast in both themes, respects reduced motion.

---

## 16. Platform layer (`kipple-core::platform`)

- **Paths:** expand `~`, `%VAR%`, `$VAR`, `{var}` (pack vars); normalize separators; canonicalize safely (no following symlinks for safety checks — use `symlink_metadata`).
- **OS detection:** `macos`, `linux`, `windows`, plus `wsl` (Linux with `/proc/sys/fs/binfmt_misc/WSLInterop` or `WSL_DISTRO_NAME`). Under WSL, integrations MAY also look at the Windows side via `/mnt/c/Users/<name>` [v1].
- **Command runner:** spawn with explicit args (never a shell), timeout, captured output, inherited `PATH`, locale `C`. Commands come only from integration definitions — never from user-provided strings in MCP.
- **Trash:** `trash` crate; detect unavailability (headless Linux) and report.
- **Processes:** `sysinfo`, refreshed once per scan and once per executed entry.
- **Permissions:** macOS Full Disk Access is never required for MVP features; integrations needing it (`~/Library/Containers/...`) degrade gracefully and `doctor` explains how to grant it.

---

## 17. Security & privacy

- No network access except: local Docker socket / named pipe, and `kipple packs update` [v1] (HTTPS to GitHub releases, signature verified with minisign; pinned public key compiled in).
- AI history contents are read only to extract metadata (timestamps, cwd, first user prompt for the title, sizes). Content never leaves the machine and is never written to the index except the truncated, secret-masked title.
- Secret scanner [v1] runs locally with a curated rule set (AWS, GCP, GitHub, OpenAI, Anthropic, Stripe, Slack, private keys, JWTs, generic high-entropy) — reports file + line offsets + rule, never prints the secret in full (first 4 chars + mask).
- Pack commands are allow-listed by the pack schema (program + fixed args; vars only from pack-defined paths). Community packs fetched by `packs update` are signed.
- `SECURITY.md` with a private reporting channel (GitHub security advisories).

---

## 18. Testing strategy

- **Unit tests** per module.
- **Fixtures:** every integration has `fixtures/<integration-id>/` containing (a) a manifest describing a synthetic tree to generate (`fixture.toml`), and (b) small real-format samples (fake JSONL sessions, a tiny `state.vscdb` built by a script, an AVD `config.ini`, an Xcode `info.plist`). Fixtures are generated into a temp dir by `cargo xtask fixtures` / test helpers — never committed as large binaries. **No real user data in fixtures**; generated content only.
- **Golden tests** (`insta`): scan of each fixture produces a snapshot of items (paths relativized). Changing an integration's behaviour shows up as a snapshot diff in review.
- **Safety property tests** (`proptest`): random paths/symlinks never cause `is_deletable()` to return true for protected locations or anything outside roots.
- **CLI tests** (`assert_cmd`): `scan --json` schema validation, exit codes, `clean` refusal without confirmation, `undo` round-trip on a fixture.
- **Never run tests against the real home directory.** Test helpers set `HOME`/`USERPROFILE`/`XDG_*`/`CODEX_HOME`/`CLAUDE_CONFIG_DIR` etc. to a temp dir; a CI check fails if any test touches paths outside its temp dir (use an env guard in debug builds).
- **CI matrix:** `macos-latest`, `ubuntu-latest`, `windows-latest`; stable Rust (MSRV documented in `Cargo.toml` `rust-version`). Jobs: fmt, clippy `-D warnings`, test, `cargo deny`, pack schema validation, docs link check. Docker adapter tests run only on Linux with a Docker service.

---

## 19. Release & distribution

- `cargo-dist` builds: macOS (arm64, x86_64), Linux (x86_64, arm64, musl static), Windows (x86_64, arm64). Shell + PowerShell installers, checksums, GitHub Releases.
- Homebrew: `brew install <org>/tap/kipple` via a separate `homebrew-tap` repo updated by cargo-dist.
- crates.io: `cargo install kipple` (publish `kipple-cli` as package `kipple`; library crates published as `kipple-core` etc.).
- [v1] winget, Scoop bucket, AUR (`kipple-bin`), Nix (community).
- SemVer. `CHANGELOG.md` follows Keep a Changelog. Release notes highlight new integrations.

---

## 20. Documentation & repo hygiene

- `README.md`: name + pronunciation line, one-sentence pitch, demo GIF (TUI), install (brew/cargo/curl/PowerShell), 60-second quickstart (`kipple scan` → `kipple clean --safe-only` → `kipple undo`), **"What kipple will never touch"** section, supported integrations table (generated from packs/adapters by `cargo xtask docs`), comparison with other tools (factual, kind), contributing link, license.
- `CONTRIBUTING.md`: dev setup, architecture overview, "Add an integration in 15 minutes" walkthrough (pack + fixture + snapshot), commit conventions, how to report a tool's storage path on your OS.
- Issue templates: bug, new integration request, "my tool stores data somewhere else" (path report), false-safety report (highest priority label `safety`).
- Labels: `good first issue`, `integration`, `safety`, `platform:macos|linux|windows|wsl`, `ui`, `docs`.
- `CODE_OF_CONDUCT.md` (Contributor Covenant), `SECURITY.md`, `LICENSE-MIT` + `LICENSE-APACHE`.
- `docs/integrations/<id>.md` generated per integration: what it is, paths per OS, safety, actions, known gotchas.

---

## 21. MVP acceptance criteria

The MVP ships when all of these are true on macOS and Linux:

1. `kipple scan` on a real developer home completes within the §5.3 targets and reports every MVP integration listed in `INTEGRATIONS.md §1` that is present.
2. Every item has a safety level, ≥ 1 reason, and accurate `reclaimable` (pnpm hardlink case verified by test).
3. `kipple clean --safe-only` executes via quarantine; `kipple undo` restores byte-identical content (verified by test with checksums).
4. Codex sessions are removed only via `codex delete` when available; Cursor data is never row-edited; Claude Code transcripts are quarantined, never edited; Antigravity is quarantine-only with the app closed.
5. No command modifies anything outside kipple's data dir without an explicit confirmation (verified by a test that runs every read-only command against a fixture and asserts no mtime changes).
6. `--json` output validates against the published schema for every command.
7. TUI usable at 80×24, all screens reachable by keyboard, works with `NO_COLOR`.
8. `kipple doctor` explains every undetected-but-expected integration.
9. Share card renders deterministically.
10. README, CONTRIBUTING, SECURITY, issue templates, CI green on 3 OSes, release artifacts published via cargo-dist, Homebrew tap works.

---

## 22. Open questions (decide and record in `docs/decisions/`)

1. `jwalk` vs `ignore::WalkParallel` — benchmark on fixtures; choose by p50 cold-scan time.
2. Desktop frontend: Svelte 5 (default) vs SolidJS — default to Svelte unless a contributor strongly prefers otherwise.
3. ~~Quarantine on other volumes~~ — decided: always OS trash (`docs/decisions/0002`).
4. ~~`kipple clean` with no flags~~ — decided: requires a selection until the TUI picker exists (`docs/decisions/0003`).
5. Community pack bundle hosting and signing key management.
