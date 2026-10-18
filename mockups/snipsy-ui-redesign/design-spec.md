# Snipsy design spec — CleanShot X–inspired UI overhaul

Phase 1 deliverable · 2026-10-17 · written from the audit of main @ 387dd66.
Companion files: [icon-mapping.md](icon-mapping.md) (licensing verdict, full icon mapping,
purge list), [index.html](index.html) (interactive mockups, light + dark, all surfaces).
Mockup CSS tokens in [tokens.css](tokens.css) mirror the GPUI constants here 1:1.

An implementation thread should be able to build every surface from this document without
guessing. Anything not derivable from it is marked **open** — it needs Christopher's or the
implementer's decision before phase 3, not mid-flight.

---

## 1. What CleanShot X actually does (design study)

Sources: cleanshot.com (marketing screenshots), Macworld's 2022 review (annotator
walkthrough), TYPENORM's UX profile. Verified traits, in priority order:

1. **The capture is the interface.** Every surface is a small, floating, rounded window or
   overlay holding a screenshot; chrome appears only while needed and floats *over* the
   capture, never docked in a way that shrinks it.
2. **The quick access overlay is the signature move.** After capture, a small card pops in a
   screen corner with the thumbnail and Copy/Save as prominent actions; annotate/pin/close as
   icons. It appears unasked, so it stays small, quiet, and dismissible.
3. **Annotation toolbar is a floating dark translucent pill.** Tool icons only (no text
   labels), generous hit areas, subtle separators between tool groups; properties for the
   active tool (color, width) appear next to the tools or in a small attached strip.
4. **Selection context editing.** Selecting a shape raises a compact strip near the shape /
   toolbar with color dots, stroke width, and destructive actions — not a properties panel.
5. **Native feel.** macOS system font, system light/dark, restrained accents; one blue accent
   carries the whole app. Rounded radii are moderate (8–16 px), shadows are soft and layered,
   borders are hairlines.
6. **History lives in flow**, as a rail/panel of recent captures with search — not a modal
   file browser.
7. **The capture overlay itself is famously empty**: dimmed screen, selection rectangle,
   dimensions label, nothing else.

Distilled north star for Snipsy: **a quiet floating-tool utility where the screenshot owns
every pixel and the chrome is a pill that appears when you need it.**

## 2. Design language

### 2.1 Layout model

- Every surface is a **frameless rounded window** (GPUI: `WindowDecorations::Client` on
  Windows or borderless `PopUp` per surface, 12 px window radius) with a minimal 34 px
  titlebar strip: capture filename + dimensions in `caption` text at left, window buttons at
  right. The tray stays the app's real "home".
- The editor canvas fills the window; nothing docks permanently. The toolbar, selection
  strip, and quick-access card all **float** with shadows.
- History is a **right-side rail inside the editor** (232 px, matches the CLEA-6 bake-off
  decision), toggled, not persistent.

### 2.2 Spacing scale (base 4)

4 · 8 · 12 · 16 · 20 · 24. Controls inside toolbars use 8 between groups, 2 between sibling
buttons. Panels use 12–16 padding. Nothing in the app uses an odd value.

### 2.3 Corner radii

| Token | px | Used for |
|---|---|---|
| `r-window` | 12 | window shells, prefs window |
| `r-panel` | 10 | cards, side panels, popovers, rail items |
| `r-pill` | 16 | floating toolbar, selection strip (height 44–52 → half-round ends) |
| `r-btn` | 8 | buttons, tool buttons, menu rows |
| `r-input` | 6 | text inputs, hotkey chips, dimensions badge |
| `r-thumb` | 6 | capture thumbnails |
| `r-round` | 999 | counter badges, toggles |

### 2.4 Color

Two full schemes; every surface ships both. The editor canvas backdrop and overlay are
always dark-neutral in both schemes (the screenshot must pop); only chrome switches.

| Token | Dark | Light |
|---|---|---|
| bg-window | `#1f1f23` | `#f6f6f8` |
| bg-surface (panels, rail) | `#26262b` | `#ffffff` |
| bg-elevated (inputs, secondary buttons) | `#2e2e34` | `#f0f0f3` |
| bg-canvas (editor backdrop) | `#17171a` | `#e8e8ec` |
| bg-toolbar (floating pills) | `rgba(34,34,39,0.98)` | `rgba(255,255,255,0.98)` |
| text-1 / text-2 / text-3 | `#f2f2f4` / `#a0a0aa` / `#6e6e78` | `#1d1d21` / `#6e6e78` / `#a0a0aa` |
| accent | `#4c8dff` | `#2f6fed` |
| accent-soft (active tool bg) | `rgba(76,141,255,0.16)` | `rgba(47,111,237,0.12)` |
| hairline / strong | `white 8%` / `14%` | `black 8%` / `14%` |
| danger | `#e5484d` | `#c94a3c` |
| selection frame | `#4c8dff` | `#2f6fed` |

Annotation palette (kept from current app, works on both schemes):
`#e5484d #f7a94b #46a758 #4c8dff #f2f2f4 #17171a`.

The old `.impeccable/design.json` ("Clear Markup Desk", light-first, text-labeled dock) is
superseded by this spec; phase 3 should replace that file's component HTML with these tokens.

### 2.5 Type ramp

Segoe UI Variable Text (fallback Segoe UI), one family everywhere:

| Style | px | Weight | Used for |
|---|---|---|---|
| display | 20 | 600 | empty-state titles |
| title | 15 | 600 | prefs section headers |
| body | 13 | 400 | descriptions, meta |
| label | 12 | 400/600 | buttons, nav items |
| caption | 11 | 400 | titlebar, dimensions, shortcuts, hints |
| micro | 9–10 | 500 | kbd chips, badge digits |

Toolbar buttons are **icons only** — the shortcut digit renders as micro text in the button's
bottom-right corner (this replaces the current text-labeled toolbar; tooltips carry names).

### 2.6 Elevation

Two shadows, both expressible in GPUI (`Style::box_shadow` → `BoxShadow { color, offset,
blur_radius, spread_radius }`, verified in gpui-pre 0.3.9 `src/style.rs:349`):

| Token | Value (dark) | Used for |
|---|---|---|
| `shadow-floating` | `0 12px 40px rgba(0,0,0,.45)` + `0 2px 8px rgba(0,0,0,.35)` (two BoxShadows) | toolbar pill, selection strip, quick-access card |
| `shadow-panel` | `0 8px 28px rgba(0,0,0,.40)` + `0 1px 4px rgba(0,0,0,.30)` | rail popovers, prefs window, context menus |

Light scheme scales alpha to ~.16/.08 (see tokens.css). **No backdrop blur** — gpui has no
backdrop-filter, so floating pills use near-opaque backgrounds (0.98) instead of macOS-style
translucency. Do not fake it with lower alpha; text over the canvas becomes unreadable.

## 3. GPUI expression notes (verified against gpui-pre 0.3.9 vendored source)

- **SVG icons**: `svg()` element exists (`src/elements/svg.rs`), renders via usvg/resvg,
  accepts raw bytes via `.data(&[u8])`, tints with `.text_color(...)`. Chrome icons are
  therefore bundled SVGs, no raster pipeline, DPI-crisp. Icon only paints when a text color is
  set — every icon element must set one.
- **Rounded quads + borders**: `div().rounded(px()).border_1().border_color(...)` and
  `window.paint_quad(quad, corner_radius, bg, border_*, ...)` cover every radius and hairline
  in this spec.
- **Shadows**: `BoxShadow` vec on style (offset, blur, spread, inset). Two-layer soft shadows
  = two entries.
- **Paths**: `PathBuilder::stroke(px())` + `paint_path` stays for the annotation shapes
  themselves (arrows, pen strokes, selection frames) — not for icons.
- **Text**: system font stack via `default_font_family()`; no web fonts.
- Not expressible / avoid: backdrop blur, gradient chrome fills (canvas gradients exist but
  keep chrome flat), blur-behind popovers.

## 4. Icon system

Full licensing verdict, per-icon mapping, and the purge list live in
[icon-mapping.md](icon-mapping.md). Summary:

- **HugeIcons free set (MIT, 6,000+ stroke-rounded)** — no key, no seat, safe to commit.
  Pro is *not* needed; if bought later, one seat and no committed Pro files.
- **Chrome icons**: bundled SVGs from `scripts/generate-hugeicons-icons.mjs`, rendered with
  `svg().data(bytes).text_color(...)`; 20 px in toolbars, 16 px in text buttons/rail rows.
- **Canvas stickers**: same SVGs pre-rasterized to 512 px PNGs by the generator (canvas
  painting + tiny-skia flatten need pixels); tinted at runtime like today.
- **Purge**: the five inherited packs (lucide/phosphor/tabler/heroicons/remix), their PNGs,
  licenses, npm deps, `generate-editor-icons.mjs`, and the `IconPack` enum. After phase 2,
  no other icon set remains — the grep gate is in the mapping doc.

## 5. Component patterns

### 5.1 Floating toolbar (editor)

- Geometry: height 52, pill radius 16, floats 18 px above the window bottom edge, centered;
  padding 0 8; shadow-floating.
- Content L→R: 12 tool buttons (36×36, radius 8, icon 20, micro shortcut digit bottom-right)
  · separator · undo, redo (18 px icons, 45% opacity when disabled) · separator · history,
  preferences toggles · separator · **Copy** (text button) · **Save PNG** (accent-filled,
  text button, icon 16).
- States: hover `bg-hover`; active tool `accent-soft` bg + accent icon; disabled 45% opacity.
- The current full-width bottom bar (48 px, `#2b2b31`, text buttons "Undo"/"Redo"/"Copy",
  hand-drawn tool paths) is replaced by this pill.

### 5.2 Selection strip (editor, when a shape is selected)

- Geometry: same pill, height 44, sits 12 px above the toolbar (mockup: bottom 82).
- Content: 6 palette dots (18 px, active gets accent ring) · separator · width chips
  S/M/L/XL (only for stroke tools) · separator · z-order icons · separator · Delete in
  danger color.
- Replaces the current full-width 44 px bar with "◀ Back"/"Forward ▶"/"× Delete" text.

### 5.3 History rail (editor, right side)

- Geometry: 232 px column, bg-surface, left hairline; head "History" + refresh; search field
  (26 px, bg-elevated, radius 6, search icon); list grouped by day headers (micro, uppercase).
- Rows: 64×40 thumbnail (radius 6) + name (caption, ellipsis) + sub (micro: dims · time).
  Hover raises an inline action card (pin / copy / delete icons, 24 px buttons). The open
  capture is tinted `accent-soft`.
- Replaces the current Library side panel (which mixes library + confirm-delete flows into
  text buttons).

### 5.4 Quick access card (post-capture)

- Geometry: 320 px wide card, bottom-right of the primary screen, 26 px margin; radius 10;
  shadow-floating; thumbnail area 170 px; meta row (micro); action row: **Copy** and
  **Save** as equal 34 px buttons (Save accent-filled), then annotate / pin / close icons.
- New surface (Snipsy currently auto-pins with no intermediate choice). Esc dismisses; the
  card pre-selects the default action from prefs. **open**: default action list (copy/save/
  annotate/pin) — confirm with Christopher.

### 5.5 Capture overlay dressing

- Keep the current mechanics (transparent full-screen window, rubber band). Change only the
  dressing: scrim `rgba(0,0,0,0.45)` (today 0.15 — too weak to read as "capture mode");
  selection shows the **live screen content** (un-dimmed interior, already how it works),
  1 px accent frame + 4 px accent glow (second quad), corner handles 8×8;
  dimensions badge (caption, tabular numerals, bg-toolbar, radius 6) centered 12 px below
  the selection; one hint pill top-center on first use only ("Esc cancel · Shift lock axis").
- No toolbar in the overlay. CleanShot's overlay is famously empty; Snipsy's should be too.

### 5.6 Preferences window

- Geometry: 780×540, radius 12; left nav 190 px (icon 15 + label, radius 8 rows; active =
  accent-soft + accent text); body 24 px padding; section title 15/600 + description caption.
- Rows: label (12) + sub (micro) left, control right, hairline between rows. Controls: toggle
  (34×20, accent when on), hotkey chip recorder (kbd chips, radius 6, bottom border 2 px),
  select chip. Pages: General, Capture, Editor, Hotkeys, History.
- Replaces the current Preferences *side panel inside the editor*; preferences become their
  own window reachable from tray + editor toolbar.

### 5.7 Pin window chrome

- Keep frameless image window. Add hover chrome only: a small floating pill (like 5.1)
  fades in at the bottom-center of the pin with opacity ±, click-through toggle, copy, save,
  close — replacing the undiscoverable keyboard shortcuts (`+`/`-`/`t`), which stay working.

## 6. Compatibility / migration notes

- `Shape::Icon` serializes `IconPack`+`IconGlyph` today. New docs store `{ icon_id, color,
  size }` from the single curated set. **Old annotations**: render stored pack/glyph names to
  the closest new icon (the mapping in icon-mapping.md §C is deliberately a superset of the
  old 12 glyphs); unknown ids render as `StarIcon`. The old packs are deleted, not shipped.
- The legacy Tauri tree (`src/`, `src-tauri/`) still holds old icon assets; it's untouched in
  this phase — finish the purge when the tree is dropped.

## 7. Verified vs assumed

**Verified** (read in repo or vendored source): all current icon locations and the 5-pack
`IconPack` system; hand-drawn `icon_for` paths; text-glyph buttons; overlay scrim 0.15; gpui
`svg()` + usvg + text-color tint; `BoxShadow`; radii/border expressibility.
**Verified via web**: HugeIcons free = MIT/no key (hugeicons.com license FAQ); Pro = per-seat,
no source redistribution; CleanShot X patterns from cleanshot.com, Macworld, TYPENORM.
**Assumed**: Segoe UI Variable available on target Windows (standard on Win10/11); mockup
proportions (toolbar 52, rail 232) are starting values to tune in phase 3; quick-access card
is a net-new surface (CleanShot-inspired) and needs Christopher's feature sign-off.
