# 0006 — The kipple UI design boards replace UI.md's visual tokens

**Status:** accepted · 2026-10-26

## Decision
The design in `docs/design/boards/` is the visual source of truth for the desktop app, menu bar, share card, TUI and website. Where it differs from `docs/UI.md` (glass materials, gradients, shadows, Mona Sans and SF fonts, a brighter palette), the design wins, and `docs/UI.md` is rewritten to match.

Frontends are built as proper codebases from the design (components, design tokens, typed data), never by copying the board HTML.

Frontend and backend are built side by side. Frontends talk to the core through a typed client that mirrors SPEC §3.4, with a mock implementation until each core API lands.
