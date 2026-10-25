# 0005 — Personal folders protect themselves, not dev artifacts inside them

**Status:** accepted · 2026-10-26 · changes SPEC §7.4

## Context
SPEC §7.4 made `~/Documents`, `~/Desktop`, `~/Pictures`, `~/Movies` and `~/Music` fully protected. Many developers keep code under `~/Desktop` or `~/Documents`, so their `node_modules`, `target` and `.venv` folders would never be offered.

## Decision
Two kinds of protection:
- **Subtree** — the location and everything in it are always `Keep`: `~/.ssh`, `~/.gnupg`, credentials files, keychains, iCloud Drive, kipple's config and data, system folders, and paths in `safety.protect`.
- **Folder** — personal folders (`Documents`, `Desktop`, `Pictures`, `Movies`, `Music`, `Videos`, `Downloads`). The folder itself is always `Keep`, and so is anything inside that an integration didn't positively identify (kipple never touches unidentified files anyway). Items of the categories `project-artifact` and `build-cache`, and items of opt-in integrations the user enabled, are judged by the normal rules.

Any path that *contains* a protected location (e.g. `~/.config`, which holds kipple's config) is always `Keep`.

## Consequences
Package caches, AI history and similar fixed-location items never live in personal folders, so this only opens up project artifacts. `is_actionable` still refuses every hard-protected path, `.git` directories and repositories, and anything outside the scan roots.
