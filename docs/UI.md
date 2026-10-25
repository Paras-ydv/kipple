# kipple — Interface Design

The visual source of truth is the design in `docs/design/boards/` (decision 0006). This file is the written form of that design as implemented: tokens, components, every screen, the menu bar, the app icon, the share card, the terminal UI and the website. When the boards and this file differ, the boards win and this file is fixed.

Implementations:

| Surface | Code | Section |
|---|---|---|
| Design tokens, fonts, shared components | `packages/design` | §2, §3.9 |
| Typed core client (mock + Tauri) | `packages/client` | — |
| Desktop app and menu bar popover | `app/` (Tauri 2 + Svelte 5) | §3 |
| Terminal UI | `crates/kipple-cli` (ratatui) | §4 |
| Share card | `crates/kipple-share` (SVG + resvg) | §5 |
| Website (kipple.dev) | `web/` (Astro + Svelte islands) | §6 |

---

## 1. Design intent

**Idea: a stack of glass.** kipple is the junk that piles up in layers. The signature element is the **core sample**, drawn as a floating stack of coloured glass slabs, one slab per kind of kipple, each as tall as the space it takes. It appears on the Overview, Projects (one stack per project), the Review plan (before and after), the menu bar popover, the website, the app icon and the share card. The terminal draws the same idea as one row of coloured blocks.

Around the stack the app is a macOS Tahoe style window: translucent glass panels over the desktop wallpaper, one blue tint for the primary action, plain sentences, numbers in tabular figures.

**Colour lives in the data.** Category colours are bright and saturated; chrome is white at different opacities on dark glass. Safety is always a word plus a shape, never colour alone.

**Never:** emoji, robots or brooms, confetti on success, fake progress, uppercase tracked-out labels, `→` in button text, exclamation marks, a grid of identical stat cards. When something isn't specified, choose the quieter option.

---

## 2. Tokens

All tokens live in `packages/design/src/tokens.css` (CSS custom properties, prefix `--k-`) and `packages/design/src/tokens.ts` (the same values for TypeScript and SVG). The design defines one appearance: **dark glass**. There is no light theme yet; when one is drawn it gets its own column here.

### 2.1 Colour — ink and surfaces

Text and lines are white at fixed opacities over dark glass. The lowest text opacity is 0.55, which keeps contrast at or above 4.5:1 over the darkest and the lightest glass in the window (the boards use 0.45 to 0.5 for a few captions; those are raised to 0.55 for WCAG AA).

| Token | Value | Use |
|---|---|---|
| `text` | `#F5F6FA` | primary text, icons |
| `text-sub` | `rgba(255,255,255,.88)` | the line under a display number ("of kipple on this Mac") |
| `text-2` | `rgba(255,255,255,.72)` | sizes in lists, the unit next to a display number |
| `text-3` | `rgba(255,255,255,.62)` | secondary sentences, legends |
| `text-4` | `rgba(255,255,255,.55)` | captions, badges, sidebar section label, version |
| `line` | `rgba(255,255,255,.09)` | row dividers |
| `line-soft` | `rgba(255,255,255,.08)` | dividers inside glass panels |
| `line-strong` | `rgba(255,255,255,.12)` | figure outlines |
| `track` | `rgba(255,255,255,.07)` | empty part of bars and switches |
| `accent` | `#1677FF` | focus ring, checkbox accent |
| `accent-top` | `#5BA9FF` | top of the tint gradient |
| `accent-icon` | `#7FB8FF` | icon of the selected sidebar item, selection frame |
| `accent-text` | `#9FCBFF` | selected project name, "waits in quarantine" dot, info tags |
| `positive` | `#8CF0A6` | bytes leaving the disk ("76.5 GB out", "76.5 GB safe to remove") |
| `growth` | `#FFC46B` | growth deltas, "Last commit 7 months ago", dull project ages |
| `secret` | `#FFD597` | secret warning panel title and link |
| `secret-body` | `rgba(255,236,210,.82)` | secret warning panel body |
| `traffic-close` / `-min` / `-zoom` | `#FF5F57` / `#FEBC2E` / `#28C840` | window controls (browser preview only) |
| `stage` | `#06080F` | background of figures (stack stages), website, share card |

**Wallpaper** (shown behind the window in the browser preview; in the native app the real desktop shows through):
```
radial-gradient(70% 55% at 18% 92%, rgba(255,132,84,.62), rgba(255,132,84,0) 62%),
radial-gradient(60% 50% at 88% 86%, rgba(178,92,255,.6), rgba(178,92,255,0) 62%),
radial-gradient(80% 60% at 55% -8%, rgba(52,124,255,.75), rgba(52,124,255,0) 62%),
linear-gradient(180deg, #0C1D52 0%, #1A2462 46%, #3A2662 76%, #62305A 100%)
```

### 2.2 Colour — categories

Same everywhere (app, menu bar, TUI, share card, website). Every colour differs in lightness as well as hue.

| Category (`Category` id) | Label | Hex | Drawn on boards |
|---|---|---|---|
| `container` | Containers | `#2F6BFF` | yes |
| `ai-history` | AI history | `#8B5CF6` | yes |
| `package-cache` | Package caches | `#FFB020` | yes |
| `project-artifact` | Project artifacts | `#3CCB5A` | yes |
| `mobile` | Mobile | `#12C8C2` | yes |
| `model` | Models | `#FF5C8A` | yes |
| `toolchain` | Toolchains | `#FF7A45` | chosen, not drawn |
| `build-cache` | Build caches | `#A3D635` | chosen, not drawn |
| `ide-cache` | IDE caches | `#4CC9F0` | chosen, not drawn |
| `logs` | Logs | `#B0B8C8` | chosen, not drawn |
| `system` | System | `#7B8591` | chosen, not drawn |

The "kipple" legend swatch is `linear-gradient(90deg, #2F6BFF, #8B5CF6, #FFB020, #3CCB5A)`.

### 2.3 Colour — safety

Safety is shown as a **tag**: a word inside a tinted capsule. The word is always there; the shape differs per level so it survives greyscale and colour blindness: Safe is a capsule, Review a capsule with a leading ring, Keep a capsule with a leading bar. In legends the dots are: Safe round, Review square, Keep a short bar.

| Level | Word | Tag background | Tag text (app) | Terminal | Meaning |
|---|---|---|---|---|---|
| Safe | Safe / `safe` | `rgba(60,203,90,.18)` | `#7DE89A` | `#6FC08F` | regenerable, nothing lost |
| Review | Review / `review` | `rgba(255,176,32,.18)` | `#FFC46B` | `#E6A740` | you decide; may hold data |
| Keep | Keep / `keep` | `rgba(255,95,87,.18)` | `#FF9C96` | `#E57A70` | never offered for removal |

Other tags (AI history flags) use the same capsule: neutral `rgba(255,255,255,.1)` / `rgba(255,255,255,.75)` ("Mostly tool output"), secret `rgba(255,176,60,.2)` / `#FFC46B` ("API key inside"), orphan `rgba(127,184,255,.18)` / `#9FCBFF` ("Folder deleted"), locked `rgba(255,95,87,.18)` / `#FF9C96` ("Open in Codex").

Tag geometry: height 20 (11 px text, weight 600, padding 3 px vertical), radius 9, horizontal padding 8; in fixed columns the tag is 58 px wide and centred.

### 2.4 Typography

Two families plus mono. Fonts are bundled; nothing is loaded from a CDN at runtime.

| Role | Stack | Notes |
|---|---|---|
| UI text | `-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, "Segoe UI", Roboto, "Helvetica Neue", sans-serif` | SF is system-only, never bundled |
| Display numbers | `-apple-system, BlinkMacSystemFont, "SF Pro Display", system-ui, "Segoe UI", Roboto, sans-serif` | big sizes |
| Brand | `"Mona Sans", -apple-system, system-ui, sans-serif` | Mona Sans variable (SIL OFL 1.1), bundled in `packages/design/fonts/` |
| Mono | `ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace` | paths, commands |

- **Wordmark:** "kipple", lowercase, Mona Sans weight 800, `font-stretch: 125%`, letter-spacing −0.02em (−0.03em at 22 px and above).
- Numbers always `font-variant-numeric: tabular-nums`. Text is antialiased (`-webkit-font-smoothing: antialiased`).
- The website uses Mona Sans for all text (headings at `font-stretch: 100%`).

App scale (px / weight / letter-spacing):

| Token | Size | Weight | Tracking | Use |
|---|---|---|---|---|
| `hero` | 132 / line-height 0.9 | 700 | −0.05em | Overview total ("184") |
| `hero-unit` | 44 | 600 | −0.02em | "GB" next to the hero number, `text-2` |
| `display-l` | 64 / lh 1 | 700 | −0.045em | Review total |
| `display-m` | 54 / lh 1 | 700 | −0.045em | AI history tool total |
| `display-s` | 46 / lh 1 | 700 | −0.045em | menu bar total |
| `number-l` | 34 | 700 | −0.03em | project total |
| `subhead` | 24 | 600 | −0.01em | "of kipple on this Mac" |
| `title` | 20 | 700 | −0.01em | panel headings (project name, tool name) |
| `wordmark` | 17 | 800 | −0.02em | sidebar, popover |
| `body-l` | 15 | 650 (titles) / 400 (paragraph, lh 1.5) | 0 | toolbar title, primary button, overview paragraph |
| `body` | 13 | 400–650 | 0 | lists, buttons, controls |
| `small` | 12 | 400–600 | 0 | sizes in rows, legends, footnotes |
| `caption` | 11 | 400–600 | 0 | toolbar subtitle, tags, month labels |

Weights used: 500, 550, 600, 650, 700, 800. Sentence case everywhere.

### 2.5 Space and radius

- Base unit 2 px; common steps 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 28, 32, 36.
- Radii by hierarchy:

| Token | Radius | Use |
|---|---|---|
| `r-dot` | 50% | category dots (8 px; 6 px in dense lists; 10 px on the share card) |
| `r-bar` | 7 | disk bar (14 tall) |
| `r-row` | 9 | sidebar items, tags |
| `r-thumb` | 16 | popover stack thumbnail |
| `r-sidebar` | 17 | sidebar panel |
| `r-panel` | 20 | glass panels |
| `r-figure` | 22 | stack figures, hero panels |
| `r-popover` | 24 | menu bar popover |
| `r-window` | 26 | app window |
| pill | height / 2 | every button, segmented control, chip and search field |

- Control heights: 30 (segment), 32 (sidebar row, small button), 36 (toolbar controls), 38–40 (secondary buttons), 42–48 (primary buttons). Click targets for primary actions are at least 40 px tall.

### 2.6 Materials, shadows and blur

Glass is a fill + `backdrop-filter` + a 1 px light border + an inner top highlight. Each material is a CSS class in `packages/design/src/materials.css` and a component prop.

| Material | Fill | Backdrop | Border | Shadow |
|---|---|---|---|---|
| `window` | `rgba(15,17,29,.56)` | `blur(48px) saturate(170%)` | `rgba(255,255,255,.14)` | `0 60px 140px rgba(0,0,0,.55), inset 0 1px 0 rgba(255,255,255,.1)` |
| `glass` (panels, sidebar) | `linear-gradient(180deg, rgba(255,255,255,.12), rgba(255,255,255,.045))` | `blur(28px) saturate(180%)` | `rgba(255,255,255,.17)` | `inset 0 1px 0 rgba(255,255,255,.42), inset 0 -1px 0 rgba(255,255,255,.05), 0 14px 40px rgba(0,0,0,.26)` |
| `clear` (secondary buttons, toolbar groups, search) | `rgba(255,255,255,.07)` | `blur(20px) saturate(180%)` | `rgba(255,255,255,.24)` | `inset 0 1px 0 rgba(255,255,255,.5), inset 0 0 14px rgba(255,255,255,.05), 0 6px 18px rgba(0,0,0,.22)` |
| `tint` (primary button) | `linear-gradient(180deg, #5BA9FF 0%, #1677FF 100%)`, text `#FFF` | none | `rgba(255,255,255,.38)` | `inset 0 1px 0 rgba(255,255,255,.7), inset 0 -3px 8px rgba(0,50,150,.35), 0 10px 28px rgba(22,119,255,.45)` |
| `sel` (selected sidebar row) | `rgba(255,255,255,.15)` | none | none | `inset 0 1px 0 rgba(255,255,255,.32), inset 0 0 0 1px rgba(255,255,255,.06)` |
| `lens` (selected segment) | `rgba(255,255,255,.2)` | none | none | `inset 0 1px 0 rgba(255,255,255,.55), inset 0 -1px 0 rgba(255,255,255,.08), 0 2px 8px rgba(0,0,0,.25)` |
| `chip` (labels over figures) | `rgba(12,14,26,.42)` | `blur(16px) saturate(180%)` | `rgba(255,255,255,.2)` | `inset 0 1px 0 rgba(255,255,255,.35), 0 6px 16px rgba(0,0,0,.3)` |
| `popover` | `linear-gradient(180deg, rgba(30,34,56,.5), rgba(18,20,34,.58))` | `blur(40px) saturate(180%)` | `rgba(255,255,255,.18)` | `inset 0 1px 0 rgba(255,255,255,.4), 0 30px 80px rgba(0,0,0,.5)` |
| `amber` (secret warning) | `linear-gradient(180deg, rgba(255,176,60,.2), rgba(255,140,40,.08))` | `blur(24px) saturate(180%)` | `rgba(255,196,107,.35)` | `inset 0 1px 0 rgba(255,220,160,.45), 0 12px 30px rgba(0,0,0,.25)` |
| `tube` (month bars) | `rgba(255,255,255,.05)` | none | `rgba(255,255,255,.3)` | `inset 0 1px 0 rgba(255,255,255,.35), inset 0 -6px 12px rgba(255,255,255,.04)` |
| `liquid` (fill of a tube) | `linear-gradient(180deg, #C3ABFF 0%, #8B5CF6 45%, #5B2FD6 100%)` | none | none | `inset 0 2px 0 rgba(255,255,255,.55), inset 4px 0 6px rgba(0,0,0,.25), inset -4px 0 6px rgba(0,0,0,.3), 0 0 18px rgba(139,92,246,.45)` |
| `warm` (website primary) | `linear-gradient(180deg, #FFD29A 0%, #FF9F4D 100%)`, text `#2A1404` | none | `rgba(255,255,255,.45)` | `inset 0 1px 0 rgba(255,255,255,.8), inset 0 -3px 8px rgba(150,60,0,.3), 0 10px 30px rgba(255,150,70,.35)` |

Figures (stack stages) have `box-shadow: 0 0 0 1px rgba(255,255,255,.12), 0 30px 60px rgba(0,0,0,.35)` (Overview) or `0 24px 50px` (Projects, Review) and an inner rim `inset 0 1px 0 rgba(255,255,255,.25), inset 0 0 0 1px rgba(255,255,255,.06)`.

When `prefers-reduced-transparency: reduce` is set, or `backdrop-filter` is unsupported, materials fall back to opaque fills: `window` `#141726`, `glass` `#24283A`, `clear` `#2A2E40`, `chip` `#1A1D2C`, `popover` `#1C1F30`.

### 2.7 Motion

- One orchestrated moment: **during a scan the stack builds slab by slab** as categories are found (each slab grows from 0 to its height in 240 ms, `cubic-bezier(.2,.8,.2,1)`). The scan button shows a spinning refresh glyph while the scan runs. Nothing else animates on its own.
- Responses to actions (selection, hover, opening a dialog, switching a tab) transition in at most 150 ms, ease-out.
- Progress is real: the scan line counts files and bytes from `ScanEvent::Progress`; there is no progress bar without a known total.
- `prefers-reduced-motion: reduce`: no growth, no spinner rotation, all transitions 0 ms.

---

## 3. Desktop app

Tauri 2 + Svelte 5 + TypeScript (SPEC §15). The app talks to the core only through `KippleClient` (`packages/client`), whose methods mirror SPEC §3.4. In a browser it runs on `MockClient`; inside Tauri it uses `TauriClient`.

### 3.0 Window frame

- Default window 1328 × 820, minimum 1100 × 720. In the browser preview the window floats on the wallpaper (§2.1) at 56 px from the left and 40 px from the top of a 1440 × 900 canvas, centred when the viewport is larger. Natively the window has no system title bar (`titleBarStyle: Overlay`, traffic lights drawn by macOS at the sidebar's top-left), a transparent background and macOS vibrancy under the `window` material.
- Window: material `window`, radius 26, padding 10, flex row with gap 10.
- **Sidebar** (`nav`, label "Sidebar"): material `glass`, width 236, radius 17, padding 16 10 14, column with gap 18.
  1. Traffic lights row: three 12 px circles, gap 8, padding 2 8 (browser preview only; natively macOS draws them in the same place).
  2. Section label "Library" (11 / 600 / `text-4`, padding 4 10).
  3. Six rows, gap 2, each 32 tall, padding 0 10, radius 9, gap 10: 17 px stroke icon (1.8 stroke, `rgba(255,255,255,.8)`; selected `#7FB8FF`), label (13 / 500; selected 600 on `sel`), trailing badge (12, `text-4`):
     - Overview · Projects · AI history (badge: AI history size, "38.9 GB") · Review plan (badge: plan size, "76.5 GB") · Quarantine (badge: quarantined size when not zero) · Settings.
  4. Footer at the bottom: wordmark (17) left, version "0.1.0" (11, `text-4`) right, baseline aligned, padding 0 10.
- **Content** (`main`): flex column, padding 6 14 14 18, gap 12.
  - **Toolbar**, 44 tall: left a two-line title (screen name 15 / 650; subtitle 11, `text-4`), right the screen's controls (36 tall, gap 10).
  - Toolbar controls on Overview: a `clear` icon group (36 tall, radius 18, padding 0 4) with Search, Filter and Share card buttons (36 × 30, 16 px icons), and a `clear` **Scan** button (padding 0 16 0 12, 15 px refresh icon, gap 7, 13 / 600).

Sidebar icons (24 × 24 viewBox, stroke paths):

| Item | Path |
|---|---|
| Overview | `M4 15a8 8 0 1 1 16 0M12 15l4-5` |
| Projects | `M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z` |
| AI history | `M4 5h16v11H9l-5 4z` |
| Review plan | `M9 6h11M9 12h11M9 18h11M4 6l1 1 1.5-2M4 12l1 1 1.5-2M4 18l1 1 1.5-2` |
| Quarantine | `M3 7l9-4 9 4v10l-9 4-9-4zM3 7l9 4 9-4M12 11v10` |
| Settings | circle r 4 at 12,12 plus 8 spokes: `M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1` |

Keyboard: `⌘1`–`⌘6` (Ctrl on Windows/Linux) open the six sidebar items in order, `⌘R` scans, `⌘F` or `/` focuses the screen's search, `Esc` closes dialogs and clears search, arrow keys move inside lists and segmented controls, `Enter`/`Space` activate. Focus ring: 2 px `#9FCBFF` outline, 2 px offset, on every control.

### 3.1 Overview

Toolbar: "Overview" / volume name ("Macintosh HD"). Body: row, gap 36, padding-top 12.

- **Stack figure** (left, 552 × 690, radius 22, stage background, figure shadow): the core sample (§3.9 `GlassStack`) of the six largest categories, largest at the bottom, centred at x = 190 with slab width 220, filling y 120 to 580. To the right of each slab a label (left edge 12 px after the stack): a 7 px white dot with a 3 px `rgba(255,255,255,.18)` ring, a 12 px × 1 px `rgba(255,255,255,.55)` leader, then a `chip` (28 tall, radius 14, padding 0 11 0 9, gap 8, 12 px): 8 px category dot, category label (600), size (`rgba(255,255,255,.7)`). Each label is vertically centred on its slab. Clicking a label opens that category (§3.8). Accessible name: "A floating stack of six glass slabs, one per kind of kipple, sized by how much space each takes".
- **Summary column** (flex 1, gap 22, padding-top 18):
  1. Scan line (13, `rgba(255,255,255,.58)` → `text-3`): "Scanned 2 minutes ago. 1.02 million files in 18 seconds."
  2. `h1`: hero number "184" + unit "GB" (baseline aligned, gap 10).
  3. "of kipple on this Mac" (`subhead`, `text-sub`).
  4. Paragraph (15 / 1.5, `rgba(255,255,255,.66)`, max 40ch, margin-top 4): "76.5 GB is safe to remove right now. The rest is chat history, database volumes and emulators, so you decide."
  5. Buttons (gap 10): `tint` **Reclaim 76.5 GB** (44 tall, padding 0 22, 15 / 650) → Review plan; `clear` **Review everything** (44 tall, padding 0 20, 15 / 550) → Projects.
  6. **Disk panel** (`glass`, radius 20, padding 16 18, gap 12): header row "Macintosh HD" (600) / "41 GB free of 512 GB" (`text-3`), 13 px; bar 14 tall, radius 7, `track` fill with `inset 0 1px 2px rgba(0,0,0,.35)`, 1 px gaps: your files `rgba(255,255,255,.3)`, then one segment per category in category colour (width = bytes / capacity), the rest empty; legend (12, `text-3`, gap 16): "Your files 287 GB", "kipple 184 GB" (gradient swatch), "Free 41 GB" (outlined swatch). The bar has `role="img"` and the label "287 GB of your files, 184 GB of kipple, 41 GB free".
  7. **Grew this week**: header row (15 / 650) "Grew this week" and the total delta in `growth` ("+10.7 GB"), padding-bottom 8; then the top three growers, 36 tall rows with a top `line` border: 8 px dot, name, delta (`text-2`).

States:
- *Scanning*: the scan line reads "Scanning. 412,000 files so far." (counts from progress events), the hero shows the running total, slabs grow as categories appear, the Scan button reads "Stop scan" and cancels the scan (the core finishes the current step, then stops).
- *Empty*: hero "0 GB", line "Nothing to reclaim. kipple checked 1.02 million files.", no Reclaim button.
- *Error*: an inline `glass` panel naming what failed and the fix ("Docker isn't running. Start Docker to measure images and volumes.").
- *Not available yet* (Tauri build before the core is wired): the panel reads "This build can't talk to the kipple core yet." and the screen shows no numbers.

### 3.2 Projects

Toolbar: "Projects" / "61 projects holding 112 GB". Controls: a segmented control (`clear` group 36 tall, radius 18, padding 3, gap 2; segments 30 tall, radius 15, padding 0 14, 13 px; selected `lens` 600, others `rgba(255,255,255,.8)`): **All** · **Untouched 30+ days** · **Folder deleted** (with the count, 11 px `rgba(255,255,255,.6)`); then a `clear` search field (36 tall, 190 wide, radius 18, padding 0 14, gap 8, 15 px magnifier, placeholder "Search").

- **Shelf figure** (full width × 429, radius 22, stage background): one stack per project for the seven largest matching projects, slabs per category, stack height proportional to the project's size (tallest 210 px), bottoms on one floor line at y = 300. Stacks are centred at x = 170 + 115·i (scaled to the figure width) and are 92 px wide. Projects untouched for 180 days or more, or whose folder is gone, are **dull**: `saturate(.25) brightness(.7)`. The selected project gets a frame 128 wide from y 64 to 300: radius 18, `1.5px solid rgba(127,184,255,.85)`, `0 0 0 4px rgba(127,184,255,.16), 0 0 40px rgba(80,150,255,.35)`. Labels under each stack (bottom 18): name (13 / 650; selected `#9FCBFF`), size (12, `text-2`), age (11; dull `growth`, otherwise `text-4`) "7 months ago" / "Today". Stacks are buttons in a `listbox`; ←/→ move the selection.
- **Detail panel** (`glass`, radius 20, padding 18 20, gap 28, fills the rest):
  - Left column 220: project name (`title`), path in mono (12, `rgba(255,255,255,.6)`), last-used line in `growth` ("Last commit 7 months ago"; recent projects in `text-3`), and the total (`number-l`, display font) at the bottom.
  - Item list: two columns (gap 24), rows 38 tall with a bottom `line-soft` border: 8 px category dot, title (ellipsised), size (`text-2`), a 58 px safety tag (§2.3). Clicking a row opens the explain sheet (§3.9).
  - Right column 196, bottom aligned, gap 10: `tint` **Add 6.1 GB to plan** (40 tall; the project's Safe bytes; adds those items to the plan and opens Review plan; disabled with "Nothing safe here" when zero) and `clear` **Show in Finder** ("Show in Explorer" on Windows, "Show in Files" on Linux).

### 3.3 AI history

Toolbar: "AI history" / "38.9 GB across 4 tools". Controls: a segmented control of tools, each segment "Codex" + size (11, `rgba(255,255,255,.6)`), selected weight 650.

- **Tool panel** (`glass`, radius 22, padding 20 24, gap 32, 250 tall, bottom aligned):
  - Left 270: tool name (`title`), total (`display-m`), "in 318 sessions" (13, `text-3`), and at the bottom a one-paragraph explanation (13 / 1.5, `rgba(255,255,255,.7)`): "Each session keeps every tool output, so one long session can pass 1 GB. This month alone added 5.2 GB."
  - Right: twelve month tubes (space-between): value label (11, `text-2`, one decimal), a `tube` 30 × 160, radius 8, with a `liquid` fill from the bottom (height = value / (max × 1.08), at least 6%, radius 5) and a 3 × 120 highlight at left 6 / top 8 (`linear-gradient(180deg, rgba(255,255,255,.55), transparent)`), month label (11; current month `text`, others `text-4`). The chart has an accessible label ("Codex history added per month, from 0.2 GB a year ago to 5.2 GB this month").
- **Sessions panel** (`glass`, radius 20, padding 10 18 14, flex 3): rows 46 tall with a bottom `line-soft` border: checkbox (16 px, accent `#1677FF`), title (13 / 600, ellipsised; masked first prompt) over "project, age" (11, `text-4`), an optional flag tag (§2.3), size (64 wide, right aligned, 13, `rgba(255,255,255,.8)`). Locked rows (session open in the tool) have a disabled checkbox and the "Open in Codex" tag. Footer: note (12, `text-4`) "Quit Codex first. The open session stays." and a `tint` button (38 tall) stating the exact action: **Delete 23 sessions with Codex (11.4 GB)**. AI history items are Review, so the button opens the confirmation dialog (§3.4).
- **Side column** (flex 2, gap 12):
  - Secret panel (`amber`, radius 20, padding 16 18, gap 6): "3 sessions contain API keys" (14 / 700, `secret`), body (12 / 1.5, `secret-body`) "An OpenAI key and two GitHub tokens were pasted into chats. Rotate them, then remove the sessions.", link "Show the 3 sessions" (12 / 650, `secret`) which filters the list. Shown only when secrets were found.
  - One `glass` panel for each of the two largest other tools (the name links to its tab): name + size (14 / 700), one or two sentences (12 / 1.5, `rgba(255,255,255,.68)`), an optional `clear` small button (32 tall). Cursor: "9.6 GB is chat data that only Cursor can remove safely. Run Delete Old Chats in Cursor, then compact the database here." + **Compact database, frees 1.9 GB**. Claude Code: "Cleans transcripts older than 30 days by itself. 41 sessions belong to folders you deleted." Antigravity: "Keeps an old copy of each conversation after it migrates them. Support is experimental, so kipple asks before it moves them."
- Pre-checked sessions are kipple's suggestion: for Codex the old, large sessions in projects you left (23 sessions, 11.4 GB in the sample); for Claude Code and Cursor the ones whose folder was deleted; for Antigravity every old copy. Footer notes: Claude Code "Transcripts are moved whole and never edited."; Cursor "Quit Cursor first."; Antigravity "Antigravity support is experimental." Buttons: "Move 41 sessions to quarantine (1.2 GB)", "Move 5 folders to quarantine (2.2 GB)", "Move 8 copies to quarantine (2.5 GB)".

### 3.4 Review plan ("Reclaim")

Toolbar: "Review plan" / "13 safe items, picked by kipple" (the count is the number of rows in the two groups; after the user changes the selection it reads "N items you picked"). Control: `clear` **Edit selection** (opens the selection sheet: every item grouped by category with checkboxes; Keep items are listed but locked with their reason).

- **Stage figure** (600 × 690, radius 22): two stacks on one floor. Left "Today" (all categories), right "After this plan" (each category minus what the plan removes; empty categories vanish). Chips (`chip`, radius 16, padding 7 14, centred above each stack): caption (11, `rgba(255,255,255,.65)`) and size (17 / 700). Between the stacks an arrow (54 × 14, stroke `rgba(255,255,255,.7)` 1.6) over a chip "76.5 GB out" (13 / 700, `positive`).
- **Plan column** (flex 1, gap 16, padding-top 8):
  1. "This plan reclaims" (13, `rgba(255,255,255,.6)`) over the total (`display-l`).
  2. Two honest lines (13 / 1.45, gap 8), each with an 8 px dot: `positive` dot, **"43.0 GB is freed right away"** + "by the tools that made it. They download it again when needed."; `accent-text` dot, **"33.5 GB waits in quarantine for 7 days"** + "so you can undo it, then it is deleted." (Sentences come from `PlanTotals.freed_now` and `held_in_quarantine`; the 7 is `quarantine.keep_days`.)
  3. Groups panel (`glass`, radius 18, padding 4 16): two groups, each padding 12 0 10 with a `line-soft` divider between them: title + total (13 / 650), the three largest rows (12, `text-2`: 6 px dot, name, size), then "and 4 more: uv, pip, Homebrew, pnpm" (12, `text-4`). Group 1 "Removed by the tool that made it" (native actions), group 2 "Moved to quarantine" (quarantine and trash). Rows group plan entries by integration ("node_modules in 41 projects").
  4. **Skip quarantine** switch (label 13 / 600, sub-line "Free all 76.5 GB now. No undo." in `text-4`; switch 42 × 26, radius 13, off `rgba(255,255,255,.16)`, on `#1677FF`, white 22 px knob).
  5. At the bottom (gap 10): `tint` **Reclaim 76.5 GB** (48 tall, 15 / 700) stating the exact bytes; `clear` **Save plan for later** (40 tall); a footnote link (12, `text-4`, centred): "107.5 GB needs your decision: database volumes, emulators, models and chat history." → AI history.
- **Confirmation** (SPEC §8.2): a Safe-only plan runs after one click on the primary button. A plan that contains Review items, or Skip quarantine turned on, opens a dialog (`popover` material, 440 wide, radius 24, padding 20): what will happen in two lines (freed now, held in quarantine), the Review items listed by name and size, a text field "Type reclaim to continue", and **Reclaim 76.5 GB** enabled only when the field equals `reclaim`. `Esc` cancels.
- **Running**: the primary button becomes a determinate line "Reclaiming 6 of 13. Docker build cache." from `ExecEvent`s; **Stop after this item** cancels.
- **Result**: the stage chips read "Before" and "Now" and the right stack shows what is left; the column becomes "This plan reclaimed" over the total (`display-l`), with "43.0 GB is free now. 33.5 GB is in quarantine until 2 November." and buttons **Make a share card** (`tint`, §5), **Undo** (`clear`, restores this batch) and a plain **Make another plan**. Skipped or failed entries are listed with their reason.

### 3.5 Quarantine & history (not drawn)

Toolbar: "Quarantine" / "33.5 GB waiting, freed after 7 days". Body:
- A `glass` panel with the quarantined total (`number-l`) and the sentence "This space is still used until it is freed." and a `clear` **Free all now** button (opens the typed confirmation).
- Batches (one per plan run), newest first, in a `glass` panel: date and time, item count, size, "6 days left" (`text-3`), **Restore** (`clear`, small). Expanding a batch lists its items with per-item **Restore**.
- History: a list of journal entries (time, title, action in words, outcome, freed), 36 tall rows.

### 3.6 Settings (not drawn)

A single scrolling column of `glass` panels (max width 720), each with a 15 / 650 heading and rows of label + control:
- **Scanning**: scan folders, exclusions, "Treat projects as untouched after" (30 days).
- **Safety**: protected folders, quarantine days (7).
- **Integrations**: grouped by category, each with a switch and its doctor status ("Found 26.1.1", "Not found").
- **Menu bar**: show in menu bar, check every (1 hour), notify when kipple grew more than (10 GB).
- **Updates**: check for updates (off by default).

Changes save immediately; there is no Save button. The core has no settings API yet (SPEC §10.3 `config.toml`), so until it does the app keeps these values on the device.

### 3.7 Menu bar and popover

- **Menu bar item**: a `pill` (22 tall, radius 11, padding 0 8, gap 6, `rgba(255,255,255,.2)` with `inset 0 1px 0 rgba(255,255,255,.5)`) holding the three-slab glyph and the total (12 / 600). Natively this is a template tray icon (the glyph only) with the title "184 GB". Glyph, 10 × 16 viewBox: rects `(0.5,1.5,9,2.6,r1)` opacity .7, `(0.5,5.6,9,3.4,r1)` opacity .85, `(0.5,10.5,9,4.5,r1.2)` opacity 1. A 4 px dot appears at the glyph's top-right when kipple grew more than 10 GB since the popover was last opened.
- **Popover** (`popover` material, 380 wide, radius 24, padding 18, gap 16), a separate frameless, transparent, always-on-top window shown under the tray icon and hidden when it loses focus:
  1. Wordmark (17) and "Checked 1 hour ago" (12, `text-4`), baseline aligned.
  2. Row (gap 16): stack thumbnail (104 × 140, radius 16, stage background, outline `0 0 0 1px rgba(255,255,255,.14)`) and a column: "184 GB" (`display-s`), "of kipple on Macintosh HD" (13, `rgba(255,255,255,.66)`), "76.5 GB safe to remove" (13 / 600, `positive`, margin-top 6).
  3. Grew this week: header (13) "Grew this week" (`text-3`) / "+10.7 GB" (`growth`, 650), padding-bottom 6; three 34 tall rows with top `line` borders.
  4. Actions (gap 8): `tint` **Review 76.5 GB of safe items** (42 tall, 14 / 650) opens the main window on Review plan; then two `clear` buttons side by side (38 tall): **Open kipple**, **Scan now**.

### 3.8 Categories (not drawn)

Reached from the Overview stack labels and from Overview search; it is not in the sidebar (the boards show six sidebar items). Toolbar: category label / "31 items, 61.2 GB". A segmented control lists the categories with sizes. Body: a `glass` list of items (rows like Projects detail, plus project name and last used), sorted by size, with checkboxes that add items to the plan, and the explain sheet on click.

### 3.9 Components (`packages/design`)

| Component | What it is |
|---|---|
| `GlassStack` | The core sample. `Stage` is the figure (stage background, floor plane, rim, an HTML overlay in the same coordinates); `StackSlabs` draws one stack from `layoutStack()` (geometry below). Props: layout, dull, glow, floor, grow (scan animation). |
| `SizeText` | Formats bytes (§7) in tabular figures; optional unit split for display numbers. |
| `SafetyTag` | Word + shape capsule per §2.3. |
| `FlagTag` | Capsule for AI history flags. |
| `Dot` | Category or status dot; `shape` round / square / bar. |
| `Button` | `tint`, `clear`, `warm`; sizes sm (32), md (40), lg (44), xl (48). |
| `GlassPanel` | Panel in any §2.6 material. |
| `Segmented` | Pill segmented control with roving focus (`tablist`). |
| `SearchField` | Pill search input with icon. |
| `ListRow` | 36–46 px row with dot, title, subtitle, trailing size/tag. |
| `Switch` | 42 × 26 switch on a real checkbox with `role="switch"`. |
| `Chip` | Glass label used over figures. |
| `DiskBar` | Volume bar with legend and text equivalent. |
| `MonthTubes` | Twelve glass tubes with liquid fill. |
| `Wordmark` | "kipple" in Mona Sans. |
| `Icon` | Stroke icons from §3.0 plus search, filter, share, refresh, copy, star, arrow. |

**GlassStack geometry** (used by the app, website, app icon and share card; all numbers in px of the SVG):
- Input: layers ordered largest first (drawn at the bottom), stack centre x `cx`, slab width `w`, top `y0`, bottom `y1`.
- Lid depth `d = round(w × 0.13)`, gap between slabs `g = round(d × 0.4)`, minimum front height `hmin = round(w × 0.06)`.
- Front heights: `h_i = max(hmin, bytes_i × s)` with `s` chosen so `Σh_i + n·d + (n−1)·g = y1 − y0` (solve iteratively: layers at `hmin` are fixed, `s` is recomputed over the rest).
- Slab *i* occupies `[top_i, top_i + d + h_i]`, stacked bottom-up from `y1`.
- Draw order: floor shadow, then the glows of all slabs, then each slab bottom slab first (body, lid, streak).
- Each slab:
  1. Glow: ellipse centred on the slab, `rx = w × 0.55`, `ry = (d + h) × 0.6`, fill = colour, opacity .28, Gaussian blur σ 18.
  2. Body: rect `x = cx − w/2`, `y = top + d/2`, width `w`, height `h + d/2`, corner radius `r = min(14, w × 0.08)`; fill a vertical gradient from colour mixed 20% toward white (opacity .92) to colour mixed 38% toward black (opacity .92); stroke `rgba(255,255,255,.28)` 1 px.
  3. Lid: rect `y = top`, height `d`, radius `d / 2`, same x and width; fill colour mixed 45% toward white (opacity .96); stroke `rgba(255,255,255,.55)` 1 px.
  4. Specular streak: rect `x = cx − w/2 + w × 0.06`, `y = top + d + 4`, width `max(3, w × 0.025)`, height `max(0, h − d/2 − 10)`, radius 2, white gradient opacity .5 → 0 downwards.
- Floor shadow under the bottom slab: ellipse centred `(cx, y1 + d × 0.35)`, `rx = w × 0.7`, `ry = d × 0.35`, black .45, blur σ 10.
- Dull: the whole group gets `saturate(.25) brightness(.7)` (SVG `feColorMatrix` saturate .25 then multiply .7).

**Explain sheet**: a right-side `popover` sheet (380 wide) for one item: title, size, safety tag, every reason sentence, "Action: docker volume rm (no undo)" in mono, notes, the restore hint, and **Add to plan** / **Remove from plan**.

### 3.10 App icon

1024 × 1024 PNG source rendered from `app/src-tauri/icons/icon.svg` (`pnpm --filter app icons`), then `tauri icon` generates every size.
- Artwork on Apple's grid: a rounded square 824 × 824 at (100, 100), radius 185, fill `#0A0B26`; transparent outside.
- A warm ring behind the stack: circle centred (512, 470) r 250, stroke 46 with a gradient `#FF8454` → `#FF5C8A` → `#B25CFF` (top-left to bottom-right), blurred glow copy (σ 30, opacity .7) behind it.
- A `GlassStack` of four slabs (blue `#2F6BFF`, purple `#8B5CF6`, amber `#FFB020`, green `#3CCB5A`; relative sizes 40, 28, 20, 14), slab width 360, centred at x 512, from y 250 to 800.
- Rim overlay on the rounded square: `linear-gradient(160deg, rgba(255,255,255,.34) 0%, transparent 32%, transparent 70%, rgba(255,255,255,.12) 100%)` plus a 1.5 px inner top highlight `rgba(255,255,255,.6)` and a 1 px inner outline `rgba(255,255,255,.18)`.
- Small sizes (32, 64, 128) are the same artwork. The menu bar uses the one-colour three-slab glyph (§3.7) as a template image (`icons/tray.png`, 44 × 44, black on transparent).

---

## 4. Terminal UI (ratatui)

Drawn on `Tui.dc.html` at 127 × 33 cells (14 px mono, 20 px line height). Minimum 80 × 24.

### 4.1 Colours

Truecolor when available; otherwise map each to the nearest 256-colour entry. `NO_COLOR` set → no colours at all: safety words, `[x]` boxes, `▸`, bold and underline carry everything.

| Role | Hex |
|---|---|
| Background | `#1A1F26` |
| Text | `#D9DEE3` |
| Strong text (keys, active tab, explain title) | `#E9ECEF` |
| Header bar background / text | `#E9ECEF` / `#1A1F26` |
| Muted (headers, inactive tabs, legend labels, footers, key labels) | `#8E99A6` |
| Secondary (explain body, status line) | `#A3ADB8` |
| Pane border | `#3A4350` |
| Rule above the key bar | `#323A45` |
| Cursor row background / text | `#2E3742` / `#FFFFFF` |
| Safe / Review / Keep | `#6FC08F` / `#E6A740` / `#E57A70` |
| Categories | as §2.2 (`#2F6BFF`, `#8B5CF6`, `#FFB020`, `#3CCB5A`, `#12C8C2`, `#FF5C8A`, …) |

### 4.2 Layout (top to bottom)

Column 0 is the left edge; regions use a 1-column margin left and right unless stated.

| Row | Region | Content |
|---|---|---|
| 0 | Header bar | Full width, background `#E9ECEF`, text `#1A1F26`. Left, bold: ` kipple   184 GB reclaimable   76.5 GB safe   107.5 GB review` (fields separated by 3 spaces). Right, regular: `scanned 2m ago ` (trailing space). |
| 1 | Tabs | `1 Overview`, `2 Projects`, `3 AI history`, `4 Plan`, `5 Quarantine`, separated by 3 spaces, starting at column 1. Active: `#E9ECEF`, bold, underlined. Others: `#8E99A6`. (Categories is reached with `enter` on a category row.) |
| 2 | — | blank |
| 3 | Core strip | From column 1 to width − 2: one segment per category in summary order (largest first), width proportional to bytes (largest-remainder rounding, every non-empty category at least 1 cell), drawn with `█` in the category colour. ASCII fallback `#`. |
| 4 | Legend | From column 1: `█ containers   █ ai history   █ caches   █ artifacts   █ mobile   █ models` — the block in the category colour, the label lowercase in `#8E99A6`, 3 spaces between entries. Truncate entries that don't fit. |
| 5 | — | blank |
| 6 … H−5 | Body | Left pane and right pane side by side, 2 columns apart (below). |
| H−4 | — | blank |
| H−3 | Status line | Left from column 1, `#A3ADB8`: `Selected ` + size in bold `#6FC08F` + ` in 2 items`. Right, ending at width − 2: `press ` + `c` in bold `#E9ECEF` + ` to make a plan`. |
| H−2 | Rule | `─` across the full width in `#323A45`. |
| H−1 | Key bar | From column 1, entries separated by 3 spaces: key in bold `#E9ECEF`, a space, label in `#8E99A6`: `space select   a all safe   e explain   c make plan   / filter   s sort   ? help   q quit`. |

**Body, left pane** (categories): width 56 columns including the border, rounded border (`╭╮╰╯─│`, ASCII `+-|`) in `#3A4350`, no title. Inside, 1 column of padding.
- Header row in `#8E99A6`: ` Category          Items   Reclaimable      Safe`.
- One row per category: cursor marker (`▸` on the cursor row, space otherwise; ASCII `>`), name padded to 17, item count right-aligned in 6, reclaimable right-aligned in 14, safe bytes right-aligned in 10 (`—` when zero). Example: `▸Containers            31       61.2 GB   22.1 GB`.
- Cursor row: background `#2E3742`, text `#FFFFFF`, spanning the inner width. Other rows `#D9DEE3`.
- Footer on the last inner row, `#8E99A6`: ` 6 categories   568 items`.

**Body, right pane** (rest of the width), two boxes stacked with no gap:
1. **Items box** (height = items shown + 3): header row in `#8E99A6` with the selected category's label at column 1, `Size` right-aligned to the size column and `Safety` left-aligned to the safety column. Each item row: ` [x] ` (`#6FC08F`) or ` [ ] ` (`#8E99A6`), name padded to 30, size right-aligned in 10, 3 spaces, safety word in its colour (`safe` / `review` / `keep`). The cursor row gets background `#2E3742`. Example: ` [ ] volume nomad_pgdata              6.1 GB   review`.
2. **Explain box** (fills the rest): for the cursor item, padding 1 column.
   - Title line, bold `#E9ECEF`: item title, 3 spaces, size, 3 spaces, safety word in its colour (`Volume nomad_pgdata   6.1 GB   review`).
   - Reason sentences wrapped to the box width, `#A3ADB8`, one sentence per line where it fits: "Docker volume from the nomad-app compose project. No container mounts it." / "It holds Postgres data, so removing it deletes those database rows for good."
   - Blank line, then `Action: docker volume rm (no undo)` in `#8E99A6` (the preferred action in words and whether it can be undone).

### 4.3 Narrow terminals

- Below 110 columns the right pane moves below the left pane (left pane full width, then items box); the explain box becomes a centred popup opened with `e` and closed with `e` or `esc`.
- Below 30 rows the legend row and the blank rows 2 and 5 are dropped.
- At 80 × 24 the header keeps ` kipple   184 GB reclaimable   76.5 GB safe` and drops the review field and `scanned` text if they don't fit.

### 4.4 Behaviour

- Keys per SPEC §12: `j`/`k`/arrows move, `enter` drills in, `space` toggles the cursor item, `a` selects all Safe items in view, `e` explain, `c` make plan, `/` filter (input shown on the status line), `s` cycles sort (size, name, last used), `?` help overlay, `q` back or quit, `1`–`5` switch tabs.
- While scanning, the status line reads `Scanning. 412,000 files, 128 GB so far.` and rows appear as categories are found.
- The Plan tab mirrors §3.4: two groups ("Removed by the tool that made it", "Moved to quarantine") with their totals, the two honest lines, and confirmation by typing `reclaim` when Review items are included.
- Keep items are listed but `space` does nothing on them; the status line shows their reason.

---

## 5. Share card

1200 × 630 PNG, rendered from an SVG by `kipple-share` with `resvg`. Fonts embedded: four static Mona Sans 2.0.27 instances in `packages/design/fonts/` (SIL OFL, `OFL.txt` next to them), because resvg does not apply variable-font axes: `MonaSansExpanded-ExtraBold.ttf` (family "Mona Sans Expanded", wordmark), `MonaSansDisplaySemiExpanded-ExtraBold.ttf` (family "Mona Sans Display SemiExpanded", headline), `MonaSans-Bold.ttf` and `MonaSans-Regular.ttf` (family "Mona Sans", chips and footer). The SVG lists the static family first and "Mona Sans" second, so browsers use the variable font with `font-stretch`. All coordinates are SVG user units = px. The reference implementation of this exact SVG is `packages/design/src/share/shareCard.ts` (`renderShareCardSvg`), used by the app's share preview.

### 5.1 Background

1. `rect` 0,0 1200 × 630 fill `#06080F`.
2. Four radial gradients (`gradientUnits="userSpaceOnUse"`), each a full-size rect, colour at the centre fading to fully transparent at radius r:
   - centre (930, 300), r 460, `#347CFF` opacity .42
   - centre (1120, 640), r 420, `#B25CFF` opacity .38
   - centre (760, 640), r 360, `#FF8454` opacity .26
   - centre (40, −40), r 520, `#347CFF` opacity .16

### 5.2 Left column (x = 64, width 600)

| Element | Position | Type | Colour |
|---|---|---|---|
| Wordmark "kipple" | x 64, baseline y 86 | Mona Sans Expanded ExtraBold 26 (weight 800, `wdth` 125), letter-spacing −0.78 | `#F4F5F8` |
| Headline line 1 "My Mac was hiding" | x 64, baseline y 196 | Mona Sans Display SemiExpanded ExtraBold 64 (weight 800; `wdth` 108 in the variable font), letter-spacing −2.56 | `#F4F5F8` |
| Headline line 2 "184 GB of kipple." | x 64, baseline y 261 | same | `#F4F5F8` |
| Chips row | top y 496, height 42 | see below | |
| Footer | x 64, baseline y 574 | Mona Sans 16, weight 400 | `rgba(255,255,255,.62)`, URL `#FFFFFF` |

- Headline words: `Mac` on macOS, `PC` on Windows, `computer` on Linux. If line 1 + line 2 would exceed 600 px at 64 px, the font size steps down by 4 px until both lines fit (minimum 48); baselines keep a 1.02 line height.
- **Chips** (the three largest categories), left to right from x 64, 10 px apart. Each chip: rect height 42, radius 18, fill vertical gradient `rgba(255,255,255,.12)` → `rgba(255,255,255,.04)`, stroke `rgba(255,255,255,.18)` 1 px, and a 1 px highlight line `rgba(255,255,255,.4)` along the top inside edge (from x + 18 to x + width − 18, y + 1). Inside: dot circle r 5 centred at (x + 19, y + 21) in the category colour; text at x + 33, baseline y + 27, Mona Sans 17: the size in weight 700, a space, the label in weight 400 (`Docker`, `AI chat history`, `caches`, `Docker` is the share-card name for Containers, `models`, `mobile`, `build output` for project artifacts). Chip width = 33 + text width + 14.
- **Footer**: "Found with kipple, an open-source disk cleaner for developers. " followed by "github.com/Paras-ydv/kipple" in white.

### 5.3 Right: the stack

- `GlassStack` (§3.9) of all categories with bytes, largest at the bottom: centre x 930, slab width 240 (so `d` = 31, `g` = 12, `hmin` = 14), from y 70 to y 556.
- Floor shadow ellipse at (930, 566.85), rx 168, ry 10.85, black .45, blur σ 10 (from §3.9: `cy = y1 + d × 0.35`, `ry = d × 0.35`).
- Glow blur σ 18 as in §3.9 (resvg supports `feGaussianBlur`).

### 5.4 After a clean (within 24 h)

- Headline "I just reclaimed" / "76.5 GB of kipple.".
- Chips: the three categories that freed the most.
- Stack: one coloured slab per category with freed bytes (sized by freed bytes), then on top one clear slab for what is left (fill `rgba(255,255,255,.06)`, lid `rgba(255,255,255,.14)`, stroke `rgba(255,255,255,.3)`, no glow), sized by the remaining bytes.

### 5.5 Privacy

Never shows usernames, paths or project names. With `--with-projects` the chips row is replaced by up to three project folder names with sizes, same chip style without dots.

---

## 6. Website (kipple.dev)

Static site in `web/` (Astro, Svelte islands only where something is interactive; the only island is the copy button). The sample numbers come from `MockClient` at build time. Background `#06080F`, text `#F2F3F7`, Mona Sans for all text, no tracking, no external requests at runtime.

1. **Hero** (900 tall, full width):
   - Background: stage colour with the wallpaper glows (§2.1) toned down to stage, and a large `GlassStack` of the sample categories (slab width 300, centred, from y 170 to y 810).
   - The word "kipple" huge behind the stack: centred, top 545, `font-size: clamp(150px, 27vw, 440px)`, Mona Sans 800, `font-stretch: 125%`, letter-spacing −0.055em, line-height 0.82, filled with `linear-gradient(180deg, rgba(255,255,255,.96) 0%, rgba(255,255,255,.78) 55%, rgba(255,255,255,.08) 100%)` clipped to text. The stack is drawn in front of the word.
   - Header (top 22, left/right 24): wordmark (22, 800, `font-stretch: 125%`, −0.03em); a `glass` nav pill (padding 5, radius 22, gap 4) with links "How it works" (selected, `rgba(255,255,255,.12)`), "What it cleans", "Docs", "Changelog" (14, padding 8 14, `rgba(255,255,255,.75)`); a `glass` **Star on GitHub** link with a star icon (14 / 600, padding 10 16).
   - Copy block (left 64, top 176, width up to 420 so both buttons fit on one line, gap 18): `glass` badge "Open source. Mac, Linux and Windows." (13, padding 7 12, radius 16); `h1` "Your dev tools are hoarding." (52 / 1.02 / 700, −0.035em); paragraph (17 / 1.55, `rgba(255,255,255,.7)`) "kipple finds the node_modules, Docker layers, emulators and AI chat logs piling up on your machine, tells you what is safe to remove, and gives you the space back with a 7 day undo."; buttons: `warm` **Download for Mac** (46 tall, padding 0 22, 15 / 700) and a `glass` copy button "brew install kipple" in mono with a copy icon (46 tall, padding 0 16, 14), which copies the command and says "Copied" for 2 s.
   - Found panel (`glass`, right 56, top 236, width 268, radius 22, padding 18): "Found on one 512 GB MacBook" (13, `rgba(255,255,255,.6)`), "184 GB" (48 / 800, `font-stretch: 112%`, −0.04em), then four 34 tall rows with top borders `rgba(255,255,255,.1)`: Docker 61.2 GB, AI chat history 38.9 GB, Package caches 27.5 GB, Emulators and the rest 56.4 GB.
2. **How it works** (`#how`, max 1240, padding 96 32 40, gap 36): `h2` "Every project, as a stack." (48 / 1.04 / 700, max 14ch) beside a paragraph (17 / 1.55, `rgba(255,255,255,.68)`, max 46ch) "Each glass slab is something kipple can measure: dependencies, build output, Docker data, emulators, models, chat history. Projects you haven't opened in months turn dull."; a shelf figure (aspect 2.4, radius 28, the Projects shelf with seven stacks, three dull) with a seven-column caption (14 / 600 name, 13 detail; dull names `#C9CFDA`); three points in a row with a top border (title 21 / 700, body 15 / 1.55 `rgba(255,255,255,.64)`): "Knows what is safe", "Undo for 7 days", "Understands AI tools".
3. **What it cleans** (`#covers`, padding 56 32 72): `h2` "Knows where your tools hide things" (34 / 700) and tool pills (`glass`, 14, padding 8 13, radius 16); a `glass` terminal panel (mono 14 / 1.9): comments `rgba(255,255,255,.45)`, prompt `$` in `#FFB36B`: `# see what's there. Nothing is touched.` / `$ kipple scan` / `# remove only what's safe, with undo` / `$ kipple clean --safe-only` / `$ kipple undo`.
4. **Footer** (top border `rgba(255,255,255,.08)`, padding 26 32, 13, `rgba(255,255,255,.5)`): "kipple is free and open source under MIT and Apache 2.0. No telemetry." and the repository link.

Below 900 px wide the hero copy and the found panel stack in one column above the stack, the nav pill hides behind a menu button, and the shelf caption scrolls horizontally.

---

## 7. Words

- The product is lowercase: **kipple**. In headlines "kipple" is also the noun for the junk ("184 GB of kipple").
- One verb per action: **Scan**, **Review**, **Reclaim**, **Undo**, **Restore**, **Free now**. The button that says Reclaim produces "Reclaimed 76.5 GB".
- Buttons that remove things state exactly what and how much: "Reclaim 76.5 GB", "Delete 23 sessions with Codex (11.4 GB)", "Compact database, frees 1.9 GB".
- Reasons are one plain sentence that says what the thing is and what happens if it goes: "npm install brings it back." / "Holds database rows." / "No emulator uses it."
- Errors say what happened and what to do, without apologising.
- Sizes are SI (1 GB = 10⁹ bytes). Under 1 GB: whole MB ("860 MB"). From 1 GB: one decimal ("24.0 GB", "107.5 GB"), except whole numbers at 100 GB and above drop the decimal ("184 GB"). Disk capacity and free space are whole GB ("41 GB free of 512 GB"). Growth deltas carry a sign ("+6.2 GB"). The CLI keeps SPEC §11.3's three significant digits.
- Ages are relative and plain: "2 minutes ago", "7 months ago", "Today".
- Sentence case everywhere, no exclamation marks, no emoji.

---

## 8. Accessibility

- WCAG AA: text at least 4.5:1 over the glass it sits on (lowest text opacity .55); large display numbers at least 3:1.
- Safety is never colour alone: always the word, plus a distinct tag shape (§2.3).
- Every figure has a text equivalent: the stacks, disk bar and month tubes have `role="img"` with an `aria-label` that reads the numbers; the shelf is a `listbox` of project buttons.
- Full keyboard navigation (§3.0), visible focus ring, logical tab order, skip link to the main content.
- Checkboxes, switches and buttons are real controls with labels; locked rows say why they are locked.
- Respects `prefers-reduced-motion` (§2.7), `prefers-reduced-transparency` and `prefers-contrast: more` (opaque materials, borders at .35).
