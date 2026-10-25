# CLAUDE.md — working on kipple

kipple is an open-source, cross-platform tool that finds and safely reclaims disk space used by developer tools (dependencies, build output, caches, Docker, emulators, models, AI-agent history). Rust core, CLI/TUI, MCP server, Tauri desktop app.

## Read first
1. `docs/SPEC.md` — what to build and the rules. **Source of truth.**
2. `docs/ROADMAP.md` — milestones in order. Work on the first unchecked milestone unless told otherwise.
3. `docs/INTEGRATIONS.md` — pack format and every integration's exact behaviour.
4. `docs/UI.md` — TUI, share card and desktop app design. Follow tokens and layouts exactly.

## How to work
- Pick the next unchecked task in `docs/ROADMAP.md`. Before coding, state which spec sections it implements.
- Keep changes scoped to one task. Tick the checkbox in `ROADMAP.md` in the same change when done.
- If the spec is ambiguous or wrong, **do not silently diverge**: propose an edit to the spec file in the same change, explain it, and record significant decisions in `docs/decisions/NNNN-title.md` (short ADR).
- Before finishing any task run:
  ```
  cargo fmt --all
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```
  and fix everything. For pack changes also run `cargo run -p kipple-cli -- packs validate packs/*.toml`.

## Safety rules (never break these)
- **Never run kipple, its tests or ad-hoc scripts against the real home directory or real tool data.** Use the `TestHome` helper / temp dirs and fixtures. No `rm -rf` outside a temp dir. Ever.
- Development builds (`cargo test`, `cargo run`, `tauri dev`, `tauri build --debug`) refuse every change outside a `KIPPLE_TEST_SANDBOX` (`platform::fsx::change_guard`). Don't weaken this, and don't set `KIPPLE_ALLOW_REAL_DATA` while developing. Launch any build you test, release or debug, with a sandbox environment (`cargo xtask fixtures`).
- Read-only commands must not modify anything outside kipple's data dir.
- Every destructive code path goes through the executor (revalidation + journal + quarantine). No shortcuts in adapters.
- Third-party databases (Cursor, Codex, Antigravity) are opened read-only during discovery. Never edit rows in them unless a spec section explicitly says so.
- Codex sessions are removed only via `codex delete`. Claude Code transcripts are moved whole, never edited.
- When unsure whether something is safe → `Keep` or `Review`, never `Safe`.
- Any change in `kipple-core/src/safety/` or the executor needs new or updated tests in the same change.

## Paths and facts
- Never invent tool storage paths. Use `INTEGRATIONS.md`; if a path is unverified, mark the integration `status = "experimental"` and the path `(verify)` in docs.
- Resolve paths via the tool's own command → env var → OS default.

## Code conventions
- Libraries: `thiserror` errors, no `unwrap()`/`expect()` outside tests (use `?` with context). Binaries: `anyhow`.
- Logging with `tracing`; no `println!` in libraries.
- Public types are `Serialize + Deserialize` and part of the JSON schema; adding a field is fine, renaming/removing bumps `schema_version`.
- Interfaces (CLI, MCP, app) only use the public `Kipple` API (SPEC §3.4).
- New dependencies: justify in the PR description; prefer small, maintained crates; run `cargo deny check`.
- User-facing text: short, plain English, no jargon, no exclamation marks, no emoji. Reason texts are one sentence.
- Commits: Conventional Commits (`feat(adapters): add android-avd`), one logical change each.

## Fixtures
- Generated from `fixtures/<id>/fixture.toml` by test helpers or `cargo xtask fixtures`. Fake content only — never copy real sessions, real databases or anything with personal data.
- Snapshots via `insta`; review diffs deliberately (`cargo insta review`).

## UI work
- The design boards in `docs/design/boards/` are the visual source of truth (decision 0006); `docs/UI.md` documents them. TUI, app, share card and website follow its tokens, materials, spacing, copy and layouts. No emoji, no fake progress, never safety by colour alone. When something isn't specified, choose the quieter option.
- The share card SVG exists twice: `packages/design/src/share/shareCard.ts` and `crates/kipple-share`. Change both together.
