# Design reference

`boards/` holds the source boards of the kipple UI design (exported 2026-10-26 from the design canvas). They are a visual reference only, never shipped code. Each board is HTML with template placeholders (`{{…}}`) and expects a runtime that isn't included.

| Board | Surface |
|---|---|
| `Landing.dc.html` | Website (kipple.dev) |
| `Main.dc.html` | Desktop app: Overview |
| `Projects.dc.html` | Desktop app: Projects |
| `Review.dc.html` | Desktop app: Reclaim (review plan) |
| `AiHistory.dc.html` | Desktop app: AI history |
| `Tray.dc.html` | Menu bar popover |
| `AppIcon.dc.html` | App icon |
| `ShareCard.dc.html` | Share card |
| `Tui.dc.html` | Terminal UI |

The design supersedes the earlier tokens in `docs/UI.md` (decision 0006).
