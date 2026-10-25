# ollama

Models downloaded with Ollama.

**Status:** stable · **Category:** Models · **Verified:** 2026-10-25 (manifest/blob layout, from third-party guides — not yet against Ollama's source)

## Paths
`OLLAMA_MODELS` → `~/.ollama/models` (`%USERPROFILE%\.ollama\models` on Windows). A Linux system-service install in `/usr/share/ollama/.ollama/models` is mentioned in a warning and never changed (needs administrator rights).

Manifests: `manifests/<host>/<namespace>/<model>/<tag>` (JSON, `schemaVersion` 2, `config` and `layers` with `digest`). Blobs: `blobs/sha256-<hex>`.

## Items and safety
- One item per `model:tag`, **Review** (`LARGE_REDOWNLOAD`). Reclaimable counts only blobs no other model uses. Action: `ollama rm <model:tag>` (Ollama must be running). Without the `ollama` command the item is **Keep** (`NATIVE_TOOL_MISSING`).
- Blobs no manifest references: one **Safe** item, quarantine. Partial downloads (`-partial`) are never included. If any manifest can't be read, orphan detection is skipped.
- Unknown manifest versions are **Keep** (`UNKNOWN_FORMAT`).
