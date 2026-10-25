# @kipple/design

Design tokens, fonts and shared Svelte 5 components for the kipple app and website. The written spec is `docs/UI.md`; the visual source of truth is `docs/design/boards/`.

- `src/styles/` — CSS custom properties (`--k-*`), glass materials (`.k-glass`, `.k-clear`, `.k-tint`, …), the Mona Sans `@font-face` and base styles. Import `@kipple/design/styles.css` once.
- `src/tokens.ts` — the same tokens for TypeScript (category colours, safety, type scale).
- `src/stack.ts` — glass stack geometry and an SVG string renderer.
- `src/share/shareCard.ts` — reference share card SVG (docs/UI.md §5).
- `src/components/` — `GlassStack` pieces (`Stage`, `StackSlabs`), `SizeText`, `SafetyTag`, `Button`, `GlassPanel`, `Segmented`, `SearchField`, `ListRow`, `Switch`, `Chip`, `DiskBar`, `MonthTubes`, `Wordmark`, `Icon`, `Dot`, `FlagTag`.
- `fonts/` — Mona Sans variable 2.0.27 from github.com/github/mona-sans (SIL Open Font License 1.1, `fonts/OFL.txt`). The `.woff2` is for the web; the `.ttf` is for the share card renderer. SF is system-only and is never bundled.
