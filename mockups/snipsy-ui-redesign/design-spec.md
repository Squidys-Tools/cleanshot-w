# Snipsy design spec — CleanShot X–inspired UI overhaul

Phase 1 deliverable · 2026-10-17 · video + future-surfaces sections are forward design (§7–8) · written from the audit of main @ 387dd66.
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

## 7. Forward design — video / screen recording mode

> **Forward design only.** Snipsy does not record today, and nothing here is scheduled for
> implementation. This section exists so the design language already covers where the product
> is going (Christopher, 2026-10-17: study cleanshot.com/features and design video mode ahead
> of a future feature). Recording mechanics — encoder, GIF support, audio — are engineering
> decisions out of scope; everything below is only what the *UI* would add.

CleanShot's recording surfaces, verified from the features page: region/window/fullscreen
capture, video or GIF output, MP4 H.264, microphone + computer audio, "do not disturb" while
recording, **click capture** (configurable color, size, outline/filled style, animation),
**keystroke capture** (position, size, dark/light style, all-keys vs command-keys-only),
**camera bubble** (position, size, shape, fullscreen), a menu-bar recording timer, and a
native video editor with **smart zooms** (cursor-following), smoothed cursor movement,
cursor appearance styling, backgrounds, motion blur, and landscape/square/vertical exports.

Snipsy's video UI reuses the phase-1 language wholesale — same pills, tokens, radii, icons;
recording adds one new accent (see below) and one new surface class (the timeline).

**New token: `rec` accent.** Recording is signaled in red-orange, never the blue accent:
`#e5484d` (dark scheme) / `#c94a3c` (light) — same values as `danger`, reused so the palettes
stay identical. A pulsing 8 px dot + `micro` elapsed timer is the recording motif everywhere.

### 7.1 Recording region dressing (capture overlay variant)

Same transparent full-screen overlay, same dim, same 1 px frame + glow — but the frame and
glow use `rec` instead of `selection-border`, and the floating pill above the region (where
the dimensions badge sits for screenshots) carries: rec dot, elapsed time, and the region's
dimensions. Bottom-center below the region: the **arming pill** (52 px, `r-pill`,
shadow-floating) with Cancel (`Cancel01Icon`), a size/readout chip (`1280 × 800 · 30 FPS`),
source toggles (region/window/fullscreen as `MonitorIcon`-family icons), and one accent
**Record** button (34 px, `rec` fill, `TickIcon`-family circle). Esc cancels; Enter arms.
CleanShot's overlay stays empty; so does this one — the arming pill dismisses the moment
recording starts.

### 7.2 During recording

- **Control pill** (same geometry as 7.1's arming pill, pinned below the region or docked
  bottom-center for fullscreen): Stop (`rec`-filled square), Pause/Resume, Cancel; rec dot +
  timer in the pill's left end. Fades to 45% opacity after 3 s idle, returns on hover — the
  recording chrome must never leak into the video, which on Windows means the pill lives in
  its own transparent always-on-top window excluded from capture (engineering note for the
  future implementer; the spike already proved click-through and always-on-top).
- **Tray indicator**: the tray icon swaps to its `rec`-tinted variant and the tray tooltip
  shows the timer (CleanShot uses a menu-bar timer; Windows' equivalent is the tray).
- **Click capture dressing**: a 40 px `rec` ring at the cursor, quick scale-fade — the same
  ring flattened into the video. Style variants (outline/filled, size) are prefs rows under a
  future Recording page, exactly the §5.6 row pattern.
- **Keystroke dressing**: a kbd chip (the §5.6 hotkey-chip style, dark/light variants) fades
  in bottom-left of the frame. Position/size are prefs rows.
- **Camera bubble**: rounded-rect PIP (`r-panel` radius, hairline, shadow-panel), snapped to
  one of the four corners with a 24 px margin, resized by a single chip (S/M/L) — no free
  resize in v1 of the design.

### 7.3 Video editor (future window)

Same shell as the image editor (§2.1, floating toolbar) with the canvas showing the video and
one new surface: the **timeline**, bottom-docked, 64 px tall.

- **Timeline styling**: bg-surface strip with hairline top; the clip renders as a filmstrip
  of 40 px thumbnails (`r-thumb`); the playhead is a 2 px accent line with a small chip head;
  trim handles are 12 px wide `r-input` chips that highlight accent on hover, with the
  trimmed-away region dimmed 40%. Zoom segments ("smart zooms") render as accent-soft
  brackets spanning their time range, draggable, with `+` adding one at the playhead
  (`ZoomInIcon`).
- **Floating toolbar variant**: tools become editor actions — Trim (`ScissorsIcon`), Zoom
  (`ZoomInIcon`), Cursor (`CursorRectangleSelectionIcon`, for smoothing/appearance), Audio
  (`WebcamIcon`), Undo/Redo, and export. No annotation tools in the video toolbar in v1.
- **Cursor/appearance and motion-blur controls** open a small popover (the selection-pill
  pattern, §5.2) — sliders and the S/M/L chip row, never a second window.
- **Export sheet**: a `r-panel` popover with format presets as chips (Landscape / Square /
  Vertical), quality S/M/L, and one accent Export button (`Download04Icon`). Progress renders
  in the sheet as a 4 px accent bar — no modal dialogs anywhere in the language.

### 7.4 Quick access card, video variant

Identical to §5.4 with two changes: the thumbnail area plays the video muted on hover, and
the meta row shows duration + size (`0:42 · 8.4 MB`). Copy/Save keep their positions.

## 8. Future surfaces (forward design, one paragraph each)

These feature areas on cleanshot.com/features imply new UI when (and if) Snipsy grows them.
Each already has a home in the phase-1 language, which is the point of recording them now.

- **Cloud / share links.** One-click upload from the quick-access card and the editor toolbar
  (a `Share08Icon` button next to Save) that turns into a copyable link chip once uploaded;
  history-rail rows gain a small shared-state badge; a Preferences → Sharing page (§5.6
  pattern) holds destination, link-expiry, and password rows. No new surface class — share is
  a button state, a chip, and a prefs page.
- **Scrolling capture.** An overlay variant: after region selection, the dressing swaps to a
  progress state — the region frame stays, a caption pill ("Scroll down · capturing…") with a
  thin accent progress bar replaces the arming pill, Esc cancels. The result opens in the
  normal editor; no dedicated surface.
- **Background tool (social dressing).** An editor panel (the §5.3 rail pattern, left side
  this time) with padding, aspect-ratio, and background presets — 20 built-ins as thumbnail
  chips plus custom color/image — and an "Auto Balance" action row. Renders as canvas chrome,
  flattens like any other edit; zero new window classes.
- **All-in-one mode.** A single hotkey opens the capture overlay with the arming pill extended
  by an aspect-lock chip and a "retake last selection" row; the last region persists as a
  dashed `hairline-strong` ghost frame before selection. Still the one overlay surface.
- **OCR / text recognition.** Snipsy already ships OCR (snipsy-ocr-eval → app). The only UI
  it needs in this language: after region selection in the overlay, a small HUD chip
  ("Text copied") in the dimensions-badge style; plus a Prefs → Capture row for "copy text
  instead of image". No new surface.
- **Capture history retention & filters.** The history rail (§5.3) gains type-filter chips
  (Screenshots / Videos / Pinned) above the list and a retention row in Preferences →
  History ("Keep 30 days" as a select chip). Same surface, no new patterns.
- **Self-timer & freeze.** Timer is a countdown chip (the dimensions-badge style, 3-2-1)
  centered in the overlay; "freeze screen" is an arming-pill toggle that swaps the live
  region for a frozen bitmap. Overlay-internal states only.

## 9. Verified vs assumed

**Verified** (read in repo or vendored source): all current icon locations and the 5-pack
`IconPack` system; hand-drawn `icon_for` paths; text-glyph buttons; overlay scrim 0.15; gpui
`svg()` + usvg + text-color tint; `BoxShadow`; radii/border expressibility.
**Verified via web**: HugeIcons free = MIT/no key (hugeicons.com license FAQ); Pro = per-seat,
no source redistribution; CleanShot X patterns from cleanshot.com, Macworld, TYPENORM.
**Assumed**: Segoe UI Variable available on target Windows (standard on Win10/11); mockup
proportions (toolbar 52, rail 232) are starting values to tune in phase 3; quick-access card
is a net-new surface (CleanShot-inspired) and needs Christopher's feature sign-off.

## 10. Excalidraw tool and option parity (2026-10-20)

Christopher asked for every tool excalidraw.com offers, and every option those tools expose, in Snipsy's editor. The one exception is the pen/freehand tool, which he has held off. This section inventories those tools, proposes how they fit Snipsy's toolbar, and lists the decisions still open. The mockup for it is the "Tool parity" tab in `index.html`.

**Sources.** Excalidraw source at commit `6f0a4d19b577e781ee5491d6cb89799b3fba7ee8` (https://github.com/excalidraw/excalidraw/tree/6f0a4d19b577e781ee5491d6cb89799b3fba7ee8), read on 2026-10-20 from a local copy. Excalidraw's development docs (https://docs.excalidraw.com/docs/introduction/development) and the live app (https://excalidraw.com) informed the tool list only; no line citations come from them. Citations are `file:line` in that commit. Snipsy citations are `app/cleanshot/src/editor/window.rs`. Lines tagged **unverified** were cut short or not read in full, so confirm them before implementation.

Parity means the same tools and options in Snipsy's chrome: the icon-only toolbar with tooltips, sticky tools, and Snipsy's own keys. Excalidraw's revert-to-selection behavior is not specified, per Christopher.

### 10.1 Tool inventory

Snipsy's `Tool` enum is at `window.rs:78-91`: Select, Rectangle, Ellipse, Arrow, Pen, Text, Counter, Crop, Blur, Pixelate, Redact, Icon. The key bindings at `window.rs:446-500` cover V, R, O, A, D, T, C, and numerics 1-6 for these tools, with Pen bound to D and 5. The bindings for Blur (B), Pixelate (M), Redact (Shift+R), Icon (I), numerics 7-9, and Shift+X were not visible in that read, so they are **unverified**. The key labels in the mockup (Blur B, Pixelate M, Redact ⇧R, Icon I) follow the design, not the code.

Excalidraw's numeric keys differ from Snipsy's: its Ellipse is 4, Arrow 5, and Text 8. Snipsy keeps its numbers, and each new tool takes a letter. Snipsy binds `d` to Pen, so Excalidraw's Diamond (D) cannot take D without a remap. The Shift+X binding for Draw to shape is unchecked against Snipsy, and Draw to shape is deferred below, so it needs no key yet.

Excalidraw's tool registry is `Tools.tsx:69-159`. Its overflow menu is `Toolbar.tsx:124-207`.

| Excalidraw tool | Excalidraw key | Source | Snipsy today | Decision |
|---|---|---|---|---|
| Selection | V, 1 | `Tools.tsx:75-80` | Select (1) | Have |
| Lasso | none | `Tools.tsx:156-159`; overflow `Toolbar.tsx:184-192` | none | Include in the Select group; key open |
| Hand | H | `Tools.tsx:70-73` | none | Include once zoom exists; Space-drag |
| Rectangle | R, 2 | `Tools.tsx:81-86` | Rectangle (2) | Have; add roundness |
| Diamond | D, 3 | `Tools.tsx:87-92` | none | Include; key open, D is Snipsy's Pen |
| Ellipse | O, 4 | `Tools.tsx:93-98` | Ellipse (3) | Have |
| Arrow | A, 5 | `Tools.tsx:99-104` | Arrow (4) | Have; add arrow types and arrowheads |
| Line | L, 6 | `Tools.tsx:105-110` | none | Include; key L proposed |
| Freedraw | P or X, 7 | `Tools.tsx:111-115` | Pen (5) | Excluded (pen held off); Snipsy's Pen stays |
| Text | T, 8 | `Tools.tsx:116-120` | Text (6) | Have; add font, size, alignment |
| Sticky note | N | `Tools.tsx:121-124` | none | Include; key N proposed |
| Image | 9 | `Tools.tsx:125-128`; overflow `Toolbar.tsx:123-130` | none | Include; Snipsy's Icon (I) stays separate |
| Eraser | E, 0 | `Tools.tsx:129-134` | none | Include; key E proposed |
| Frame | F | `Tools.tsx:135-138`; overflow `Toolbar.tsx:135-143` | none | Include; needs a container in the model |
| Draw to shape | Shift+X | `Tools.tsx:139-143`; overflow `Toolbar.tsx:154-163` | none | Defer with the pen, since it recognizes freehand strokes |
| Bucket fill | B | `Tools.tsx:152-155`; overflow `Toolbar.tsx:174-183` | none | Include; Snipsy uses B for Blur, so key G proposed |
| Web embed | none | `Tools.tsx:145-147`; overflow `Toolbar.tsx:145-153` | none | Exclude; see 10.7 |
| Laser pointer | K | `Tools.tsx:148-151`; overflow `Toolbar.tsx:164-173` | none | Recommend exclude; see 10.4 |

The overflow menu also holds a Generate section (`Toolbar.tsx:195-216`). Mermaid to Excalidraw reportedly shows without AI gating, and Wireframe to code arms only when AI is enabled and a diagram-to-code plugin is set. Both are excluded in 10.7. Lines past `Toolbar.tsx:207` were not read in full, so the complete overflow count is **unverified**.

Snipsy's Counter, Crop, Blur, Pixelate, Redact, and Icon have no Excalidraw equivalent and stay as they are.

### 10.2 Options per tool

Excalidraw keeps options as shared `currentItem*` properties in app state. `Actions.tsx` decides which controls each tool shows. Citations below come from grep output and partial reads; the ones tagged **unverified** have not been read in full.

- **Stroke color.** `COLOR_PALETTE` at `colors.ts:193-212` covers transparent, black, white, gray, hue families, and bronze. Quick stroke picks are at `colors.ts:239-245` (black, red, green, blue, yellow).
- **Background color.** Quick picks at `colors.ts:248-254` (transparent, red, green, blue, yellow).
- **Bucket fill background.** `BUCKET_FILL_BACKGROUND_PICKS` at `colors.ts:259-265` starts with white in place of transparent; the comment at `colors.ts:257-258` says transparent is a no-op for the bucket. Transparent is excluded from the control at `actionProperties.tsx:584-589`.
- **Sticky note color.** Stroke picks reuse the element stroke picks (`colors.ts:270-271`). Background picks at `colors.ts:275` never include transparent (comment at `colors.ts:273-274`). The default note color is **unverified**.
- **Fill style.** Hachure, cross-hatch, solid, and zigzag (`elemtypes.ts:19`). Picker `actionChangeFillStyle` at `actionProperties.tsx:611` (**unverified**).
- **Stroke width.** Thin 1, medium 2, bold 4 (`constants.ts:486-488`). `extraBold` (8, `constants.ts:489`) is left out. The medium default is **unverified**.
- **Stroke style.** Solid, dashed, and dotted (`elemtypes.ts:28`). Picker around `actionProperties.tsx:906-942` (**unverified**).
- **Sloppiness.** Stored as roughness: architect 0, artist 1, cartoonist 2 (`constants.ts:469-473`). Picker options at `actionProperties.tsx:794-808`; the 0/1/2 mapping there is **verified**.
- **Roundness.** Sharp and round. Action around `actionProperties.tsx:1762` (**unverified**), options around 1811-1816 (**unverified**). Default round (**unverified**).
- **Opacity.** `actionChangeOpacity` around `actionProperties.tsx:961`, range 0 to 100 in steps of 10 (**unverified**).
- **Arrow type.** Sharp, round, and elbow (`commonconst.ts:591-595`). Default round (**unverified**). Pressing the arrow key while the arrow tool is active cycles the type (`App.tsx` around 5346-5378, **unverified**).
- **Arrowheads.** Picker around `actionProperties.tsx:1850-1946` (**unverified** as a full list). Triangle outline, bar, and cardinality markers appear there. Defaults are an arrow at the end and none at the start (**unverified**).
- **Font family.** Nine faces in `FONT_FAMILY` (`commonconst.ts:143-153`): Virgil, Helvetica, Cascadia, Excalifont, Nunito, Lilita One, Comic Shanns, Liberation Sans, and Assistant. Default Excalifont (`commonconst.ts:271`, **unverified**). Whether these faces can be bundled under their licenses is **unverified**.
- **Font size.** Small 16, medium 20, large 28, extra large 36 (`constants.ts:125-130`). Default medium is **unverified**.
- **Text alignment.** Horizontal left, center, right (`constants.ts:434-438`). Vertical top, middle, bottom (`constants.ts:428-432`). Defaults are **unverified**.
- **Frame.** Frame rendering carries flags for enabled, clip, name, and outline (**unverified**). The controls that set them are **unverified**.
- **Image.** Options are **unverified**; read them before implementation.

Tool behavior that the options do not cover:

- **Eraser.** No options. Pointer movement marks elements pending erasure, and those render at 20% opacity: `renderElement.ts:197` multiplies opacity by `ELEMENT_READY_TO_ERASE_OPACITY` (20, `constants.ts:440`). The commit path after pointer-up (`App.tsx` around 10150) is **unverified**.
- **Bucket fill.** Pointer-up commits the fill through `bucketFill.handlePointerUp` (around `App.tsx:9476-9480`, **unverified**). Pressing B again while the tool is active cycles the background color (around `App.tsx:5367-5372`, **unverified**).
- **Laser pointer.** The trail is drawn by `LaserTrails`. Its decay constants at `laserTrails.ts:29-30` are `DECAY_TIME` 1000 and `DECAY_LENGTH` 50. An upstream file, `excal2/index.ts:50-51`, shows 200 and 10, so which tool uses which constants is **unverified**.

### 10.3 UI surfaces

Snipsy's floating toolbar holds twelve tool buttons at 36 px (`tokens.css` `--tool-btn`, mockup `index.html:202-213`). The nine new tools would take it to twenty-one buttons. Grouping keeps it at six buttons. Each group button shows its last-used tool; clicking it selects that tool, and the caret opens the group's flyout. Tooltips show the name and key, as the locked icon-only toolbar requires.

The six groups hold 21 tools:

- **Select:** Select (1), Lasso, Hand (Space-drag once zoom exists).
- **Shapes:** Rectangle (2), Diamond, Ellipse (3), Line, Arrow (4), and Snipsy's existing Pen (5).
- **Text:** Text (6) and Sticky note.
- **Insert:** Counter (7), Icon (I), Image (9), and Frame.
- **Effects:** Blur (B), Pixelate (M), and Redact (Shift+R).
- **Edit:** Crop (8), Eraser, and Bucket fill.

Undo and redo stay outside the groups, as they are now.

For properties, the selection strip in section 5.2 becomes one contextual strip that serves the active tool and the current selection. It shows only the controls the tool or selection supports. Excalidraw hides its properties panel for some tools (`Actions.tsx` around 232-238, **unverified**), so Snipsy shows an empty strip for the eraser, hand, and laser. The strip carries the most-used controls: stroke color, background, stroke width, stroke style, and opacity. A Style popover holds fill style, sloppiness, roundness, arrow type, arrowheads, and text options. This is a Snipsy layout, not a copy of Excalidraw's panel, because Snipsy's chrome floats over the screenshot.

### 10.4 Laser pointer (recommendation: exclude)

The laser draws a fading trail in the live view and creates no element. Snipsy's output is a flattened PNG, so a laser trail has nothing to flatten. This reasoning is from the design, not from source. The laser's only use is pointing during a live screen share, and Snipsy's capture flow has no such step.

If Christopher still wants it, the light spec is an overlay layer that never enters the model, a trail on the decay timing above, no undo entry, and no flatten step. This is a recommendation for Christopher to decide.

### 10.5 Sticky tool behavior

A tool stays active until the user switches away, and this spec keeps that behavior. Excalidraw's revert-to-selection behavior is intentionally not specified, per Christopher.

### 10.6 Flatten notes (CPU, original image dimensions)

Flatten runs on the CPU at the original image dimensions, with tiny-skia for shapes and fontdue for text. These per-option rules are design proposals to check against the crates before implementation:

- **Fill styles.** Solid is a path fill. Hachure, cross-hatch, and zigzag are generated line sets clipped to the shape path, then stroked.
- **Stroke styles.** Dashed and dotted use stroke dashing.
- **Sloppiness.** Offset polylines seeded from the shape's id, so one shape flattens the same way on every export. Amplitude steps with the level (0, 1, 2). This is not roughjs and will not match Excalidraw's pixels.
- **Roundness.** Round uses rounded corners; sharp uses miter joins.
- **Opacity.** One alpha per shape, multiplied into the fill and stroke paints.
- **Arrow types.** Sharp is a straight segment, round is a quadratic curve, and elbow is an orthogonal route between the endpoints.
- **Arrowheads.** Each head is a small path placed at the endpoint and oriented along the segment.
- **Text.** Rasterized with fontdue. Alignment applies to the layout box.
- **Image.** A pixmap drawn with an affine transform and the element's opacity.
- **Bucket fill.** Writes color, fill style, and opacity to the shape it hits, then flattens as a filled shape. It is not a pixel flood fill.
- **Eraser, laser, hand.** No flatten step. They edit the model or the view only.

### 10.7 Verified, assumed, unverified, and open

**Verified** against the Excalidraw commit above: the tool keys and registry lines in 10.1 (`Tools.tsx`), the sloppiness 0/1/2 mapping at `actionProperties.tsx:794-808`, the eraser opacity at `renderElement.ts:197`, and the Snipsy tool enum, label table, and key bindings at `window.rs:78-91`, `2827-2831`, and `446-500` for the keys listed in 10.1.

**Unverified:** every line tagged in 10.1 through 10.3, including the overflow entries past `Toolbar.tsx:207`, Mermaid gating, default values, the frame and image option lists, the full arrowhead list, the eraser commit path, and the Snipsy bindings for B, M, Shift+R, I, and numerics 7-9.

**Assumed:** tiny-skia provides the clip, mask, and dash calls 10.6 needs, and the Excalidraw font faces can be bundled under their licenses.

**Open for Christopher:**

- Laser pointer: exclude, as recommended in 10.4.
- Web embed: exclude, or replace it with a link card.
- Mermaid to Excalidraw and wireframe to code: both excluded here; confirm.
- Draw to shape: deferred with the pen; confirm.
- Zoom and the Hand tool.
- Keys for Lasso, Diamond (D collides with Pen), Line, Sticky note, Image, Frame, Eraser, and Bucket fill. The table proposes L, N, 9, F, E, and G; Diamond and Lasso are open.
