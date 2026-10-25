# kipple — Integrations Specification

Every source of junk is an integration. This file defines (1) the catalog and its milestones, (2) the pack format, (3) adapter rules, and (4) a detailed spec for every MVP integration.

> **Path accuracy rule.** Paths below are the best current knowledge. Every integration MUST resolve paths in this order: **tool's own command** (e.g. `npm config get cache`, `go env GOCACHE`, `pnpm store path`) → **documented env var** → **OS default**. Entries marked **(verify)** must be confirmed on a real machine for that OS before the integration is marked stable; until then the integration is `experimental` and its items are capped at `Review`.

---

## 1. Catalog

Status: **[MVP]**, **[v1]**, **[later]**. Kind: **P** = pack (TOML), **A** = adapter (Rust).

### 1.1 Project artifacts (P)
| id | Matches | Milestone |
|---|---|---|
| `node-modules` | `node_modules` + sibling `package.json` | MVP |
| `js-build` | `.next`, `.nuxt`, `.svelte-kit`, `.turbo`, `.parcel-cache`, `.angular/cache`, `.expo`, `storybook-static`, `.vite`, `.docusaurus` (each with sibling `package.json`); `dist`/`build`/`coverage`/`out` only when `package.json` exists **and** the dir is git-ignored | MVP |
| `python-venv` | dir containing `pyvenv.cfg` (any name: `.venv`, `venv`, `env`) | MVP |
| `python-caches` | `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.tox`, `.nox`, `.hypothesis`, `*.egg-info` — aggregated into **one item per project** | MVP |
| `rust-target` | `target` + sibling `Cargo.toml` (also accept `CACHEDIR.TAG` inside) | MVP |
| `gradle-project` | `build` + sibling `build.gradle(.kts)`; project-local `.gradle`; Android `app/build`, `.cxx`, `.externalNativeBuild` | MVP |
| `maven-project` | `target` + sibling `pom.xml` | MVP |
| `xcode-project` | `.build` + sibling `Package.swift`; `Pods` + sibling `Podfile` (Review: some teams commit Pods); `Carthage/Build` | MVP |
| `flutter` | `.dart_tool`, `build` + sibling `pubspec.yaml` | v1 |
| `dotnet` | `bin`, `obj` + sibling `*.csproj`/`*.fsproj` | v1 |
| `cmake` | `build`, `cmake-build-*` containing `CMakeCache.txt` | v1 |
| `go-vendor` | `vendor` + `go.mod` — **Review** (vendoring is often intentional) | v1 |
| `elixir`, `haskell`, `scala`, `zig`, `bazel`, `terraform`, `ruby-bundle`, `php-vendor`, `unity`, `unreal`, `jupyter`, `latex` | per ecosystem rules (Unity `Library`, `Temp`, `Obj`; Unreal `Intermediate`, `Saved`, `DerivedDataCache`, `Binaries`) | v1/later |
| `cachedir-tag` | any dir with valid `CACHEDIR.TAG` | MVP |

### 1.2 Package caches (P unless noted)
| id | Default location (macOS / Linux / Windows) | Native command | Milestone |
|---|---|---|---|
| `npm-cache` | `~/.npm/_cacache` / same / `%LOCALAPPDATA%\npm-cache\_cacache`; resolve via `npm config get cache` | `npm cache clean --force` | MVP |
| `pnpm-store` | `pnpm store path` (defaults `~/Library/pnpm/store` / `~/.local/share/pnpm/store` / `%LOCALAPPDATA%\pnpm\store`) | `pnpm store prune` (preferred; removes unreferenced packages only) | MVP |
| `yarn-cache` | `yarn cache dir` (v1: `~/Library/Caches/Yarn` / `~/.cache/yarn` / `%LOCALAPPDATA%\Yarn\Cache`); Berry: `~/.yarn/berry/cache` | `yarn cache clean` | MVP |
| `bun-cache` | `~/.bun/install/cache` (`BUN_INSTALL_CACHE_DIR`) | `bun pm cache rm` | MVP |
| `pip-cache` | `pip cache dir` (`~/Library/Caches/pip` / `~/.cache/pip` / `%LOCALAPPDATA%\pip\Cache`) | `pip cache purge` | MVP |
| `uv-cache` | `uv cache dir` (`~/.cache/uv` on macOS+Linux / `%LOCALAPPDATA%\uv\cache`) | `uv cache prune` (Safe) / `uv cache clean` (all) | MVP |
| `poetry-cache` | `~/Library/Caches/pypoetry` / `~/.cache/pypoetry` / `%LOCALAPPDATA%\pypoetry\Cache` | `poetry cache clear --all <name>` | v1 |
| `conda-pkgs` | `conda info --json` → `pkgs_dirs` | `conda clean --all` | v1 |
| `cargo-registry` | `$CARGO_HOME` (default `~/.cargo`) `/registry/cache`, `/registry/src`, `/git/checkouts`, `/git/db` | — (quarantine) | MVP |
| `go-build-cache` | `go env GOCACHE` | `go clean -cache` | MVP |
| `go-mod-cache` | `go env GOMODCACHE` (files are read-only — always use native) | `go clean -modcache` | MVP |
| `gradle-cache` | `$GRADLE_USER_HOME` (default `~/.gradle`) `/caches`, `/wrapper/dists` (old versions = separate items), `/daemon/*/*.log` | — (quarantine; Gradle daemon must not run: `gradle --stop` suggested) | MVP |
| `maven-repo` | `~/.m2/repository` — **Review** (may contain locally installed artifacts not downloadable) | — | v1 |
| `homebrew-cache` | `brew --cache` | `brew cleanup --prune=all` | MVP (macOS/Linux) |
| `cocoapods-cache` | `~/Library/Caches/CocoaPods` | `pod cache clean --all` | v1 |
| `swiftpm-cache` | `~/Library/Caches/org.swift.swiftpm` | — | v1 |
| `nuget-cache` | `dotnet nuget locals all --list` | `dotnet nuget locals all --clear` | v1 |
| `composer-cache`, `gem-cache`, `deno-cache`, `sbt/coursier/ivy`, `vcpkg`, `conan`, `scoop-cache`, `choco-cache` | per tool | per tool | v1/later |

### 1.3 Toolchains & version managers (A) [v1]
`nvm`, `fnm`, `volta`, `pyenv`, `rbenv`, `asdf`, `mise`, `sdkman`, `rustup` (toolchains), `jenv`, old Xcode installs, Android build-tools/platforms/NDK. Each version is an item; `PINNED_BY_PROJECT` when any scanned project pins it (`.nvmrc`, `.node-version`, `.python-version`, `.tool-versions`, `rust-toolchain(.toml)`, `engines` in `package.json`).

### 1.4 Containers & VMs (A)
| id | Milestone |
|---|---|
| `docker` (images, containers, volumes, build cache via API) | MVP |
| `docker-desktop-disk` (report disk image logical vs allocated; compaction guidance) | MVP report, v1 action |
| `podman`, `orbstack`, `colima`, `lima`, `rancher-desktop`, `kind`/`minikube`/`k3d` | v1 |
| `wsl-disk` (ext4.vhdx compaction, Windows side) | v1 |
| `vagrant`, `multipass`, `utm`/`parallels`/`virtualbox` (report-only) | later |

### 1.5 Mobile (A)
`android-avd` (MVP), `android-system-images` (MVP), `android-sdk-extras` (v1), `xcode-derived-data` (MVP), `xcode-archives` (v1), `ios-device-support` (MVP), `core-simulator` (v1: devices + runtimes via `xcrun simctl`), `metro`/`watchman`/`expo` caches (v1).

### 1.6 AI tools (A)
`claude-code` (MVP), `codex` (MVP), `cursor` (MVP), `antigravity` (MVP), `windsurf`, `vscode-copilot-chat`, `gemini-cli`, `aider`, `cline`, `roo`, `continue`, `zed-ai` (v1).

### 1.7 Models (A)
`ollama` (MVP), `huggingface` (MVP), `lm-studio`, `torch-hub`, `whisper`, `keras`, `comfyui-models` (v1).

### 1.8 Browsers for testing (P/A)
`playwright` (A — needs revision logic, MVP), `puppeteer` (P, MVP), `cypress` (P, v1), `selenium` (P, v1).

### 1.9 IDE & build caches (P) [v1]
VS Code (`CachedExtensionVSIXs`, old extension versions, `logs`, stale `workspaceStorage` whose folder no longer exists), JetBrains (caches/logs for IDE versions no longer installed), Electron app caches, `ccache`, `sccache`, Nx, Turborepo global, Jest, Vite, webpack.

### 1.10 Git (A) [v1]
`git-worktrees` (merged/prunable worktrees), `git-lfs-cache`, `stale-clones` (no commit & no source mtime in N months → archive suggestion), `git-gc-hint` (packable loose objects > 500 MB).

### 1.11 System, opt-in (P/A) [v1]
`logs-crash` (`~/Library/Logs/DiagnosticReports`, core dumps), `old-installers` (`*.dmg|pkg|exe|msi|iso|xip` in Downloads older than 30 d), `downloads-large`, `trash` (report), `time-machine-snapshots` (report), `app-leftovers` (support dirs of uninstalled apps, Review), `large-files`.

---

## 2. Pack format (TOML)

### 2.1 Top level

| Field | Type | Required | Notes |
|---|---|---|---|
| `id` | string, kebab-case | yes | unique, stable forever |
| `name` | string | yes | display name |
| `category` | enum | yes | `project-artifact`, `package-cache`, `toolchain`, `container`, `mobile`, `ai-history`, `model`, `ide-cache`, `build-cache`, `logs`, `system` |
| `description` | string | yes | one or two plain sentences, user-facing |
| `docs_url` | string | no | upstream docs for the tool's storage |
| `os` | array | no | restrict to `macos`/`linux`/`windows`/`wsl` |
| `status` | enum | no | `stable` (default) / `experimental` (items capped at Review) |
| `detect.any_command` | array | no | integration active only if one is on PATH |
| `detect.any_path` | array | no | or one of these exists |
| `[vars]` | table | no | named paths, see §2.2 |
| `[[locations]]` | array | * | fixed locations (caches) |
| `[[artifacts]]` | array | * | in-project artifacts found by the walk |
| `[commands.<id>]` | table | no | native commands referenced by actions |
| `in_use_processes` | array | no | process names that make items `Keep` |

\* at least one of `locations` / `artifacts`.

### 2.2 Variables

```toml
[vars]
cache.macos = "~/Library/Caches/pip"
cache.linux = "$XDG_CACHE_HOME/pip"        # $XDG_CACHE_HOME defaults to ~/.cache when unset
cache.windows = "%LOCALAPPDATA%/pip/Cache"
cache.env = "PIP_CACHE_DIR"                 # if set, wins
cache.command = ["pip", "cache", "dir"]     # if present and succeeds, wins over everything (stdout trimmed)
```
Resolution order: `command` → `env` → OS key. Use `{name}` in paths. `~`, `$VAR`, `%VAR%` expand on every OS.

### 2.3 `[[locations]]`

| Field | Notes |
|---|---|
| `path` | path or glob (`{gradle_home}/wrapper/dists/*`) |
| `each` | `"self"` (one item for the path) or `"child"` (one item per child entry — e.g. each wrapper distribution) |
| `kind`, `title` | `title` may use `{name}` (child name) |
| `safety`, `reasons` | defaults |
| `actions` | ordered list: `native:<command-id>`, `quarantine`, `trash` |
| `keep_latest` | for `each = "child"`: keep N newest children as `Keep` (e.g. newest Gradle distribution) |
| `min_size` | skip items smaller than this |

### 2.4 `[[artifacts]]`

| Field | Notes |
|---|---|
| `dir` | directory name or glob (`cmake-build-*`) |
| `requires_sibling_any` / `requires_sibling_all` | marker files in the parent |
| `requires_inside_any` | e.g. `["CMakeCache.txt"]` |
| `requires_gitignored` | `true` = only if git ignores this dir (protects committed `build/` dirs) |
| `aggregate` | `"project"` = merge all matches in one project into a single item (pycache) |
| `restore_hint` | command shown to regenerate |
| `safety`, `reasons`, `actions` | as above |

### 2.5 `[commands.<id>]`

```toml
[commands.pip-purge]
run = ["pip", "cache", "purge"]
timeout_secs = 300
measure = "before-after"      # freed bytes = size before - size after of the location
requires_closed = []          # process names that must not run
```
Commands are argv arrays (no shell), may reference only pack vars. They run with `cwd` = the user's home.

### 2.6 Validation
`kipple packs validate <file>` checks the schema, resolves vars on the current OS, and dry-runs matchers against the matching fixture. CI runs it for every built-in pack.

---

## 3. Adapter rules

1. Discovery is **read-only**. Third-party SQLite databases are opened with `file:<path>?mode=ro&immutable=1` (or copied to a temp file first if the app may be writing) — never with write access during scan.
2. Recognise the storage format version and emit `UnknownFormat` for anything unexpected. Never guess.
3. Prefer the owner's CLI for actions. If the CLI is missing, either report-only (`NATIVE_TOOL_MISSING`) or quarantine — whichever the integration spec below says.
4. Respect `in_use_processes` and `requires_closed`.
5. Every adapter has: a fixture generator, snapshot tests for discovery, a test for each action on fixtures, and a `docs/integrations/<id>.md` page.

---

## 4. MVP integration specs

### 4.1 `node-modules`, `js-build` (packs)
- Artifact rules as in §1.1. One item per `node_modules` folder (root of the package — never nested `node_modules` inside it).
- `reclaimable` must account for pnpm/yarn-berry hardlinks (SPEC §5.5).
- `details`: package name (from sibling `package.json`), lockfile type (npm/pnpm/yarn/bun), restore hint (`pnpm install`, …).
- Monorepos: workspace package `node_modules` are separate items, attributed to the git root project.

### 4.2 `python-venv`, `python-caches` (packs)
- Venv: any dir with `pyvenv.cfg`. `details`: Python version (from `pyvenv.cfg` `version`), whether `pip`/`uv` created it. Restore hint: `uv sync` if `uv.lock` exists, `poetry install` if `poetry.lock`, else `pip install -r requirements.txt`.
- **Review** if the venv is not inside a project (a standalone env someone may rely on).
- Caches aggregate per project into one item: "Python caches (184 folders)".

### 4.3 `rust-target`, `gradle-project`, `maven-project`, `xcode-project` (packs) — per §1.1.

### 4.4 Global caches (packs) — per §1.2 table. Each location resolves via its tool command first.

### 4.5 `docker` (adapter)
- Connect via `DOCKER_HOST`, then the active Docker context (`~/.docker/config.json` `currentContext` → `~/.docker/contexts/meta/*/meta.json`), then defaults: `/var/run/docker.sock`, `~/.docker/run/docker.sock` (Docker Desktop macOS), `npipe:////./pipe/docker_engine` (Windows). If unreachable: `doctor` says "Docker isn't running — start it to measure images and volumes."
- Client: a small blocking HTTP client, API v1.41 paths (decision 0008).
- Data: `GET /system/df` for sizes, plus `/images/json`, `/containers/json?all=1`, `/volumes`, build cache from `/system/df` `BuildCache`.
- Items and safety:
  - Build cache (all non-in-use records as one item, plus "shared" note) → `Safe`, action `native:docker.builder.prune`.
  - Dangling images → `Safe`; unused tagged images → `Review`; images used by any container → not listed (or `Keep` with `IN_USE_BY_CONTAINER` in `--all`).
  - Stopped containers → `Review` (`CONTAINS_USER_DATA` — writable layer), show size of writable layer (`SizeRw`), name, image, exited-at.
  - Volumes not mounted by any container → `Review`; mounted by running container → `Keep`. Show compose project if labelled.
- Actions via API (remove image/container/volume by id, build cache prune). Never `--force` remove images used by containers.
- `reclaimable` for images uses shared-size accounting from `/system/df` (`SharedSize`) — an image whose layers are shared reclaims only unique bytes.
- **`docker-desktop-disk`** (report, opt-in — decision 0009): macOS `~/Library/Containers/com.docker.docker/Data/vms/0/data/Docker.raw` (verify per Docker Desktop version), Windows `%LOCALAPPDATA%\Docker\wsl\disk\docker_data.vhdx` / `...\wsl\data\ext4.vhdx` (verify). Show logical vs allocated size and explain that pruning inside Docker frees space in the image; whether the host gets it back depends on the Docker Desktop version (v1: guided compaction).

### 4.6 `android-avd`, `android-system-images` (adapter)
- SDK root: `ANDROID_HOME` → `ANDROID_SDK_ROOT` → defaults `~/Library/Android/sdk` / `~/Android/Sdk` / `%LOCALAPPDATA%\Android\Sdk`.
- AVD home: `ANDROID_AVD_HOME` → `ANDROID_USER_HOME/avd` → `~/.android/avd`. Each `<name>.ini` + `<name>.avd/` = one AVD.
- Parse `<name>.avd/config.ini`: `image.sysdir.1` (e.g. `system-images/android-35/google_apis/arm64-v8a/`), `avd.ini.displayname`, `hw.device.name`. Last used = newest mtime of `userdata-qemu.img*`, `snapshots/`, `*.lock` absent.
- System images: `<sdk>/system-images/<api>/<tag>/<abi>/`. `Safe` + `UNUSED_BY_ANY_EMULATOR` when no AVD's `image.sysdir.1` points to it; otherwise `Keep`/`USED_BY_EMULATOR` with the AVD names.
- AVD items `Review`; sub-item "snapshots" `Safe`-ish → `Review` (`quickboot` snapshots regenerate but lose state).
- Actions: AVD → `avdmanager delete avd -n <name>` (from `<sdk>/cmdline-tools/latest/bin`), fallback quarantine both `.ini` and `.avd`. System image → `sdkmanager --uninstall "system-images;android-35;google_apis;arm64-v8a"`, fallback quarantine.
- In use: `qemu-system-*` / `emulator` process with `-avd <name>`.

### 4.7 `xcode-derived-data`, `ios-device-support` (adapter, macOS)
- DerivedData: `defaults read com.apple.dt.Xcode IDECustomDerivedDataLocation` → default `~/Library/Developer/Xcode/DerivedData/`. One item per child folder (`<Project>-<hash>`). Attribute via `info.plist` `WorkspacePath`. `Safe`; `Keep` if Xcode is running *and* the workspace is open (MVP: if Xcode running → `Review`).
- `~/Library/Developer/Xcode/iOS DeviceSupport/*` (and watchOS/tvOS) → `Safe`, one item per OS version, newest kept as `Review`.

### 4.8 `playwright` (adapter), `puppeteer` (pack)
- Playwright: `PLAYWRIGHT_BROWSERS_PATH` → `~/Library/Caches/ms-playwright` / `~/.cache/ms-playwright` / `%LOCALAPPDATA%\ms-playwright`. Each `<browser>-<revision>` dir is an item. Mark the newest revision per browser as `Review`, older as `Safe` (`REDOWNLOADABLE` — `npx playwright install`).
- Puppeteer: `PUPPETEER_CACHE_DIR` → `~/.cache/puppeteer`. Per-browser-version items, `Safe`.

### 4.9 `homebrew-cache` (pack) — `brew --cache`; native `brew cleanup --prune=all`; `measure = before-after`.

### 4.10 `claude-code` (adapter)
- Root: `CLAUDE_CONFIG_DIR` → `~/.claude` (`%USERPROFILE%\.claude` on Windows).
- Sessions: `<root>/projects/<encoded-cwd>/<session-uuid>.jsonl`. **Do not decode the folder name** to get the project path; read the `cwd` field from the first lines of the JSONL that contain it. Title = first user message text, truncated (60 chars) and secret-masked. `last_used` = last line timestamp or file mtime.
- Item per session (`Review`, `CHAT_HISTORY`, `MAY_CONTAIN_SECRETS`); grouped per project; orphan flag when `cwd` no longer exists (`ORPHANED_PROJECT`). The item includes the session's companion folder `projects/<encoded-cwd>/<session-uuid>/` (subagent transcripts, `tool-results/`, `mcp-tasks/`) so it moves as one unit. Set-aside copies (`<uuid>.jsonl.superseded-<ts>`, `<uuid>.orphaned-<ts>-<suffix>.jsonl`) are separate items (decision 0010).
- Other subdirectories under the root: `file-history`, `plans`, `paste-cache`, `debug`, `shell-snapshots`, `session-env`, `image-cache`, `uploads`, `usage-data`, and the legacy `todos`, `statsig`, `logs` (verified 2026-10-25, see `docs/integrations/claude-code.md`): report sizes as separate items, `Review`, until each is documented as safe.
- Never touch `settings.json`, `settings.local.json`, `CLAUDE.md`, `agents/`, `commands/`, `skills/`, `plugins/`, `rules/`, `output-styles/`, `workflows/`, `agent-memory/`, auto memory (`projects/<p>/memory/`), `history.jsonl`, `backups/`, `sessions/`, `tasks/`, `jobs/`, `daemon/`, credentials (`.credentials.json`).
- In use: `claude` process running → sessions modified in the last 24 h are `Keep`; others unaffected (so the integration declares no blanket `in_use_processes`). Any session file written in the last 10 min → `Keep`.
- Actions: `quarantine` (whole `.jsonl` file plus its companion folder; never edit lines), `archive` [v1].
- Retention [v1]: `set_retention` writes `cleanupPeriodDays` in `<root>/settings.json`, preserving other keys and formatting (use a JSON editor that preserves order), shows a diff, requires confirmation. **Never write `0`** (current Claude Code rejects it in settings validation; older versions reportedly disabled transcript writing); minimum 1.
- Note for users (shown in `explain`): Claude Code deletes transcripts older than `cleanupPeriodDays` (default 30) itself.

### 4.11 `codex` (adapter)
- Root: `CODEX_HOME` → `~/.codex` (`%USERPROFILE%\.codex`).
- Sessions: `<root>/sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl` (or `.jsonl.zst` after Codex's paginated-history migration); archived: `<root>/archived_sessions/`. Thread metadata in `$CODEX_SQLITE_HOME` (default `<root>`) `/state_<n>.sqlite` — newest `n`, currently `state_5.sqlite` — table `threads` (columns verified against codex-rs migrations up to 0062, rust-v0.164.0, 2026-10-25) and `<root>/session_index.jsonl` (may be stale — prefer SQLite, read-only).
- Item per session: title from thread metadata, else first user message; project from recorded `cwd`; size; created/updated; `source` (cli, exec, vscode, app) if available. Flag sessions > 100 MB in `notes` ("dominated by tool output").
- Safety `Review` always.
- Actions: **only** `native:codex.delete` → `codex delete <uuid> --force` (`--force` is the non-interactive flag and requires a UUID; kipple checks `codex delete --help` for it at runtime; if the installed version lacks it, the session is report-only with `NATIVE_TOOL_MISSING` and the explanation tells the user to update Codex). **Never** remove rollout files directly (orphans threads in the state DB).
- `requires_closed`: all Codex surfaces (`codex`, Codex desktop app, VS Code extension host running Codex). If a rollout grows between two stats → `Keep` (`ACTIVE_WRITER`).
- `codex archive` does not free disk space — do not offer it as a reclaim action.

### 4.12 `cursor` (adapter)
- User dir: macOS `~/Library/Application Support/Cursor/User/`, Linux `~/.config/Cursor/User/`, Windows `%APPDATA%\Cursor\User\`.
- Items:
  1. `globalStorage/state.vscdb.backup` → `Review` (`BACKUP_COPY`), action `quarantine` (requires Cursor closed).
  2. `globalStorage/state.vscdb` → report: total size, bytes by key prefix in table `cursorDiskKV` (`bubbleId:`, `composerData:`, `agentKv:`, `checkpointId:`, other) computed read-only with `SUM(length(value))`, and free pages (`PRAGMA freelist_count * page_size`).
     - Action `compact:sqlite_vacuum` (`Review`, `FREE_PAGES`) only when free pages ≥ 256 MB: requires Cursor closed, copies the DB to `<data>/backups/cursor-<ts>.vscdb` first (needs free space ≥ 2 × DB size on both volumes — the backup plus SQLite's working copy during `VACUUM`, decision 0010 — check and refuse otherwise, and refuse where free space can't be measured), runs `VACUUM`, verifies `PRAGMA integrity_check`, keeps the backup for 7 days (pruned at the next compaction).
     - **kipple MUST NOT delete rows** in MVP. For chat history, `explain` tells the user to use Cursor's own command palette actions *"Developer: Delete Old Chats"* then *"Developer: GC Agent KV Blobs"* (names verified 2026-10-25 from Cursor staff replies) and then rescan.
  3. `workspaceStorage/<hash>/` whose `workspace.json` `folder` (or `workspace`) `file://` URI points to something that no longer exists → `Review` (`ORPHANED_PROJECT`), quarantine. Remote URIs are skipped.
  4. `~/.cursor/projects/*/agent-transcripts` → per project items, `Review` (path from community sources — verify).
  5. `~/.cursor/extensions` older versions of the same extension → `Safe` (v1).
- In use: Cursor process running → items 1–4 `Keep` (kipple also keeps item 4, the stricter choice).
- Status: `experimental` — Cursor doesn't document this storage.

### 4.13 `antigravity` (adapter) — `experimental`
- Data roots (check both): `~/.gemini/antigravity-ide/` (current) and `~/.gemini/antigravity/` (legacy). Inside: `conversations/*.pb` (older) / `*.db` (newer), `brain/<id>/`, `code_tracker/`, `implicit/`, `context_state/`.
- App state DB (UI index): macOS `~/Library/Application Support/Antigravity IDE/User/globalStorage/state.vscdb` or `.../Antigravity/...` (verify), Linux `~/.config/Antigravity IDE/User/globalStorage/state.vscdb` or `~/.config/Antigravity/...`, Windows `%APPDATA%\Antigravity IDE\User\globalStorage\state.vscdb` or `%APPDATA%\antigravity\...`. **Read-only, never modified.**
- Item per conversation: conversation file + matching `brain/<id>/` dir (sizes summed). Title only if cheaply available; otherwise "Conversation <short-id> · <date>".
- **Migrated copies:** when the same conversation id exists in both legacy and current roots, the legacy copy is `Review` with `MIGRATED_COPY` (and only if the current copy is ≥ the legacy copy in size).
- Safety `Review`; actions `quarantine` only; requires all Antigravity processes closed. No compaction, no DB edits.

### 4.14 `ollama` (adapter)
- Models dir: `OLLAMA_MODELS` → `~/.ollama/models` (Linux system service install: `/usr/share/ollama/.ollama/models` — report-only, needs elevation) → Windows `%USERPROFILE%\.ollama\models`.
- Parse `manifests/registry.ollama.ai/<ns>/<model>/<tag>` JSON → layer digests → `blobs/sha256-*` sizes. Item per model:tag; `reclaimable` = blobs not referenced by any other manifest.
- `Review` (`LARGE_REDOWNLOAD`). Action `native:ollama.rm` → `ollama rm <model:tag>` (needs server running) — fallback: report-only. Orphan blobs (referenced by no manifest) → `Safe`, quarantine.

### 4.15 `huggingface` (adapter)
- Hub cache: `HF_HUB_CACHE` → `HF_HOME/hub` → `~/.cache/huggingface/hub` (all OSes; Windows `%USERPROFILE%\.cache\huggingface\hub`).
- Item per repo dir `models--<org>--<name>`, `datasets--…`, `spaces--…`; sub-detail per revision (`snapshots/<rev>` → symlinks into `blobs/`). `reclaimable` per repo = its unique blobs; older revisions (not pointed to by `refs/main`) are a separate `Safe` item "old revisions".
- Repo items `Review` (`LARGE_REDOWNLOAD`). Action `quarantine` of the whole repo dir (symlinks inside stay valid relative to it).
- Also report `~/.cache/huggingface/datasets` and `xet` dirs if present (verify), `Review`.

---

## 5. Integration checklist (copy into each PR)

- [ ] Pack/adapter with paths for macOS, Linux, Windows (or `os` restriction)
- [ ] Path resolution order: command → env → default
- [ ] Fixture generator + snapshot test
- [ ] Safety defaults + reason texts reviewed
- [ ] Action tested on fixture (and native command mocked)
- [ ] `in_use_processes` / `requires_closed` set where relevant
- [ ] `docs/integrations/<id>.md` (generated + hand-written gotchas)
- [ ] `status = "experimental"` until verified on a real machine per OS
