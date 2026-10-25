# huggingface

The Hugging Face Hub cache.

**Status:** stable · **Category:** Models · **Verified:** 2026-10-25 against the `huggingface_hub` cache docs

## Paths
Hub cache: `HF_HUB_CACHE` → `HF_HOME/hub` → `~/.cache/huggingface/hub` (all systems). Also `HF_HOME/xet` (the Xet chunk cache) and `HF_DATASETS_CACHE` → `HF_HOME/datasets`.

Each repo: `{models,datasets,spaces}--<org>--<name>/` with `refs/<branch>` (commit hash), `snapshots/<commit>/` (symlinks into `blobs/`; plain files on Windows without symlinks) and `blobs/`.

## Items and safety
- One item per repo, **Review** (`LARGE_REDOWNLOAD`), quarantine of the whole folder (symlinks inside stay valid).
- Revisions no ref points to: a separate **Safe** item with their snapshot folders and the blobs only they use. The repo item's size leaves those bytes out so totals don't count them twice.
- Xet and datasets caches: **Review**, quarantine.
