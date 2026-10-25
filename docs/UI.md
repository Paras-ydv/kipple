# kipple — Interface Design

Reference mockups: the **kipple UI** design canvas (Overview, Projects, AI history, Review plan, Terminal UI, Menu bar popover, Share card). This file is the written spec; when the canvas and this file differ, this file wins and the canvas gets updated.

---

## 1. Design intent

**Idea: strata.** kipple is literally the junk that piles up in layers. The signature element is the **core sample**: the disk's kipple drawn as a column of stacked bands, one per category, each band's height proportional to its size, each in its own mineral color. It appears on the Overview, the menu bar popover (as a horizontal strip), the TUI (as a strip of blocks) and the share card. **Color lives in the data; the chrome stays quiet.**

Everything around the core sample is calm and precise: neutral slate surfaces, one typeface family, plain sentences, numbers aligned in tabular figures.

**Never:** gradient washes, glassmorphism, glowing accents, emoji, illustrations of robots/brooms, identical rounded "stat cards" in a grid, uppercase tracked-out eyebrow labels, `→` in button text, confetti on success, fake progress. When something isn't specified, choose the quieter option.

---

## 2. Tokens

### 2.1 Color — chrome

| Token | Bedrock (dark, default) | Limestone (light) | Use |
|---|---|---|---|
| `bg` | `#1A1F26` | `#EEF1F3` | window background |
| `side` | `#1F252D` | `#E4E8EC` | sidebar, panels, popover |
| `surface` | `#2A313B` | `#FFFFFF` | selected nav item, raised rows |
| `line` | `#323A45` | `#CCD3DA` | dividers, borders |
| `track` | `#2C343F` | `#DCE1E6` | empty part of bars |
| `text` | `#E9ECEF` | `#1A222B` | primary text |
| `muted` | `#A3ADB8` | `#4E5966` | secondary text (≥ 4.5:1 on `bg`) |
| `primary.bg` / `primary.fg` | `#E9ECEF` / `#1A1F26` | `#1A222B` / `#FFFFFF` | primary button = inverted text color; there is no brand accent color |

### 2.2 Color — categories (minerals)

Same values in both themes. Each differs in lightness as well as hue.

| Category | Name | Hex |
|---|---|---|
| Containers | slate | `#6E93C4` |
| AI history | amethyst | `#A88BD6` (tool shades `#C4AEE8`, `#A88BD6`, `#8467B8`, `#6A5299`) |
| Package caches | ochre | `#D4A548` |
| Project artifacts | moss | `#8DB07A` |
| Mobile | chrysocolla | `#4FB3AE` |
| Models | sandstone | `#C9AE8E` |
| Toolchains [v1] | jasper | `#C97F6B` |
| Other / system | shale | `#7B8591` |

### 2.3 Color — safety

| Level | Dark | Light | Always paired with |
|---|---|---|---|
| Safe | `#6FC08F` | `#2F7D4F` | the word "Safe"; round dot in legends |
| Review | `#E6A740` | `#A06A00` | the word "Review"; square dot in legends |
| Keep | `#E57A70` | `#B3433A` | the word "Keep" + the reason |

Safety is never communicated by color alone.

### 2.4 Typography

- **Family:** [Recursive](https://www.recursive.design/) (SIL OFL), variable. One family for everything:
  - UI text: `MONO 0`, `CASL 0`.
  - Paths, commands, code: `MONO 1`.
  - Wordmark only: weight 800, `CASL 0.6`, letter-spacing −0.02em.
- Numbers always `font-variant-numeric: tabular-nums`.
- Scale (px / weight / line-height): display 60 / 780 / 1.02 · h1 34 / 760 / 1.15 · h2 19–20 / 680–700 / 1.3 · body 15–16 / 400 / 1.5 · small 13–14 / 400 / 1.45. Big numbers (52, 40, 44) at weight 780–790, letter-spacing −0.03em.
- Sentence case everywhere. No all-caps labels (the TUI is the one exception allowed by terminal convention, and it doesn't use it either).
- Bundle the font with the app and the share card renderer; never load from a CDN at runtime.

### 2.5 Space, radius, borders

- 4 px base. Common steps: 8, 12, 16, 20, 24, 32, 40, 48.
- Radius by hierarchy, not one value for everything: bars 3–7, controls 9–11, panels 12–14, the core sample's end caps 16 (22 on the share card).
- No drop shadows. Separation comes from `line` borders and the `side`/`surface` steps. The core sample bands get a 2 px inner bottom shade only.
- Touch/click targets ≥ 44 px tall for primary controls; ≥ 40 px for filters.

### 2.6 Motion

One orchestrated moment: **during a scan, the core sample fills band by band** as categories are discovered (each band grows from 0 to its height, 240 ms, ease-out). Nothing else animates on its own. Responses to actions (expanding a row, opening the explain panel, confirming) may transition in ≤ 150 ms. `prefers-reduced-motion`: no growth animation, bands appear at final size.

---

## 3. Desktop app screens

Layout: sidebar (200–232 px) + content (max 1040–1100 px wide, left-aligned). At narrow widths the sidebar stacks above content; tables scroll horizontally inside their box.

Sidebar: wordmark, then Overview · Projects · AI history · Review plan (shows current plan size) · Quarantine · Settings, then a disk meter at the bottom ("41 GB free of 512 GB. Clearing kipple would leave 225 GB free.").

### 3.1 Overview
- Line: "Scanned 2 minutes ago. 1.02 million files in 18 seconds."
- Display headline: **"184 GB of kipple"**. Under it one sentence that splits safe vs review in plain words.
- Actions: primary **Review safe items**, secondary **Scan again**.
- Left (3 fr): **What it's made of** — the core sample (104 px wide, ~2.85 px per GB, min band 52 px) with each band's row beside it: category name, size, one-line description of what's inside, and a small safe/review split bar.
- Right (2 fr): **Heaviest projects** (name, path in mono, last used, size, mini strata bar) and **Grew this week** (top 3 growers with deltas).
- States: *scanning* — headline shows the running total ("128 GB so far"), bands grow; *empty* — "Nothing to reclaim. kipple checked 1.02 million files." with the list of integrations that found nothing; *error* — inline message naming what failed and the fix ("Docker isn't running. Start Docker to measure images and volumes.").

### 3.2 Projects
- Header: "61 projects hold 112 GB…" explaining that Docker data and AI sessions are counted with their project.
- Filter field + toggles: All · Not touched in 30 days · Folder deleted (n).
- Table: Project (name + path), Last used (recent ones in Review amber), Made of (strata bar), Reclaimable.
- Detail panel for the selected project: name, path, one-line description (last commit, stack), total, and every item with its color tick, size, plain reason and safety word. Actions: **Add N GB of safe items to plan**, **Show in Finder/Explorer/Files**.

### 3.3 AI history
- Header sentence states the policy: kipple never edits these files and uses each tool's own delete command where one exists.
- Tool strip: one proportional bar in the four amethyst shades, then tool tabs (name, size, count).
- Selected tool's sessions table: checkbox, title (first prompt, secret-masked), project, flags (*Mostly tool output*, *Possible API key*, *Folder deleted*, *Open in Codex, kept*), last active, size. Locked rows (in use) show a disabled checkbox and the reason.
- Footer bar: what to close first + the exact action ("Delete 23 sessions with Codex (11.4 GB)").
- Side column: secret warning panel (the only warm-tinted panel in the app), then one short panel per other tool that explains its specific situation (Cursor: use *Delete Old Chats* in Cursor, then compact here; Claude Code: self-cleans after 30 days; Antigravity: old copies after migration).

### 3.4 Review plan
- Two groups, each with its own total: **Move to quarantine** ("Undo for 7 days") and **Removed by the tool that made it** ("Downloads again when needed"). Each row: checkbox (checked), color tick, name, how it's removed or how it comes back, size.
- "Not in this plan: 107.5 GB to review" as a dashed panel that points to Projects / AI history.
- Summary panel: "This plan frees **76.5 GB**", split bar, and two honest lines — what is freed right away vs what waits in quarantine — plus the opt-in "Free the quarantined 33.5 GB now (no undo)". Pre-flight checks with check marks. Primary button states exact bytes: **Reclaim 76.5 GB**; secondary **Save plan for later**.
- After reclaiming: the same screen becomes a result view ("Reclaimed 76.5 GB. 43.0 GB is free now; 33.5 GB is in quarantine until 1 November.") with **Undo** and **Make a share card**.

### 3.5 Quarantine (not drawn)
List of batches (date, what, size, days left) with **Restore** per batch and per item, and **Free now**. Total quarantined size at the top, explained: "This space is still used until it's freed."

### 3.6 Settings (not drawn)
Scan folders and exclusions, protected folders, recent-days threshold, quarantine days, integrations on/off (grouped by category, each with its doctor status), theme (System / Bedrock / Limestone), units (GB / GiB), retention rules [v1], updates (off by default).

### 3.7 Menu bar / tray popover (360 × 560)
Wordmark + "Checked 1 hour ago", big total, horizontal core strip, "Grew this week" with top 3, primary **Review 76.5 GB of safe items**, secondary **Open kipple** and **Scan now**. Tray icon: the 4-band mark in monochrome; a small dot appears when kipple grew more than 10 GB since you last looked.

---

## 4. Terminal UI (ratatui)

- Truecolor when available, else map minerals to the nearest 256-color entries; `NO_COLOR` → monochrome with safety words and `[x]` boxes only.
- Layout (80×24 minimum, shown at ~127×32):
  1. Inverted header bar: `kipple   184 GB reclaimable   76.5 GB safe   107.5 GB review` … `scanned 2m ago`.
  2. Tabs: `1 Overview  2 Projects  3 AI history  4 Plan  5 Quarantine` (active = bold + underline).
  3. Core strip: one row of colored blocks proportional to categories, legend below.
  4. Left pane: categories table (`▸` marks the cursor row, highlighted background).
  5. Right pane: items of the selected category with `[x]`/`[ ]`, size, safety word; below it an **explain** box for the highlighted item (what it is, why this safety, the action and whether it can be undone).
  6. Status line: "Selected 22.1 GB in 2 items … press c to make a plan".
  7. Key bar: `space select  a all safe  e explain  c make plan  / filter  s sort  ? help  q quit`.
- At 80 columns the right pane moves below the left pane; the explain box becomes a popup on `e`.
- Plan screen mirrors §3.4: two groups, honest totals, and confirmation by typing `reclaim` when Review items are included.

---

## 5. Share card

- 1200 × 630, Bedrock background. Left: wordmark, headline "My laptop was hiding **184 GB** of kipple." (74 px, 800), the three largest categories as number + label, and a footer line "Found with kipple, an open-source disk cleaner for developers." + the repo URL in mono. Right: the core sample column (150 px wide, full height).
- After a clean (within 24 h): headline becomes "I just reclaimed 76.5 GB of kipple." and the core shows reclaimed bands in full color, the rest in `track`.
- Never shows usernames, paths or project names unless `--with-projects`.

---

## 6. Words

- The product is lowercase: **kipple**. In headlines "kipple" is also the noun for the junk ("184 GB of kipple").
- One verb per action, used everywhere: **Scan**, **Review**, **Reclaim**, **Undo**, **Restore**, **Free now**. The button that says Reclaim produces "Reclaimed 76.5 GB".
- Reasons are one plain sentence that says what the thing is and what happens if it goes: "npm install brings it back." / "Holds database rows." / "No emulator uses it."
- Errors say what happened and what to do, without apologizing.
- Sizes in GB (SI) by default with one decimal; MB under 1 GB.

---

## 7. Accessibility

- WCAG AA contrast in both themes (muted text ≥ 4.5:1).
- Every bar and the core sample have a text equivalent (`aria-label` / screen-reader table).
- Full keyboard navigation; visible focus ring (2 px `text` color outline, 2 px offset).
- Checkboxes and buttons are real controls; locked rows explain why they're locked.
- Respects reduced motion, OS text size, and high-contrast modes.
