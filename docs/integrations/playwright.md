# playwright

Browsers Playwright downloads for testing.

**Status:** stable · **Category:** Package caches · **Verified:** 2026-10-25 (folder layout `<browser>-<revision>`)

## Paths
`PLAYWRIGHT_BROWSERS_PATH` → `~/Library/Caches/ms-playwright` (macOS), `~/.cache/ms-playwright` (Linux), `%LOCALAPPDATA%\ms-playwright` (Windows). `PLAYWRIGHT_BROWSERS_PATH=0` keeps browsers inside each project's `node_modules`, which the `node-modules` pack covers.

## Items and safety
One item per `<browser>-<revision>` folder (`chromium-1187`, `chromium_headless_shell-1187`, `firefox-1489`, `webkit-2090`, `ffmpeg-1011`). The newest revision of each browser is **Review**; older ones are **Safe**. Restore with `npx playwright install`. Action: quarantine.
