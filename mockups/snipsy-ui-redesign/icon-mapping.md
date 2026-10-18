# Snipsy icon system — HugeIcons mapping & purge list

Phase 1 deliverable. Goal: after implementation, **HugeIcons is the only icon set in the
repository**. This file maps every current icon to its replacement and lists everything that
gets deleted.

## Licensing verdict (verified 2026-10-17, hugeicons.com/docs/faqs/license-questions)

| Tier | Contents | License | Key/seat required |
|---|---|---|---|
| **Free** | 6,000+ stroke-rounded icons (`@hugeicons/core-free-icons`) | **MIT** | **No** — no seat, no key, no account |
| Pro | 60,000+ icons, 10 styles | HugeIcons agreement (per-seat) | Yes — one seat per person who touches the files |

**Verdict: the free MIT set is sufficient and carries no licensing risk.** It is stroke-rounded
in exactly the style CleanShot X uses for its chrome, and it may be committed to the repository
(MIT permits redistribution; keep the notice — see Bundling rules). A Pro license is *not*
needed for this redesign. If Christopher later wants Pro styles (filled/duotone/solid), note
the constraints: per-seat for everyone committing the files, and Pro source SVGs must not be
redistributed — they could be bundled into the app binary but should not be committed to a
shared repo. Buy **Pro (yearly), one seat** only if that tradeoff is accepted; the mapping
below does not depend on it.

## Rendering path (verified in gpui-pre 0.3.9 vendored source)

- gpui-pre 0.3.9 ships `svg()` (`src/elements/svg.rs`) backed by **usvg/resvg**. It accepts
  bundled asset paths (`AssetSource`), **raw bytes** (`.data(&[u8])`), and tints with the
  element's **text color** (`style.text.color` → `window.paint_svg`). Verified: the SVG only
  paints when a text color is set.
- **Chrome icons (toolbars, buttons, nav): bundle SVGs, render with `svg().data(bytes)` +
  `.text_color(...)`.** No rasterization step, crisp at every DPI.
- **Canvas sticker icons** (shapes placed on the screenshot, flattened by tiny-skia) still need
  pixel data at capture size: keep the existing pre-rasterize-to-PNG approach
  (`icons::rasterize` → tinted RGBA), regenerated from HugeIcons. One source of SVG truth; two
  consumers.
- SVG requirements: single `currentColor` stroke, 24×24 viewBox, no embedded fills — matches
  every free-set icon natively.

## Current state (audited on main, 387dd66)

Everything icon-like in the GPUI app today:

| Location | What's there | Problem |
|---|---|---|
| `app/cleanshot/assets/icons/{lucide,phosphor,tabler,heroicons,remix}/` | 12 pre-rasterized PNGs each (60 total, 292 KB) + per-pack licenses | **Five icon packs bundled at once**; only used by the sticker picker |
| `app/cleanshot/src/editor/icons.rs` | `IconPack` enum (5 packs), `IconGlyph` (12 glyphs), `rasterize()` | Multi-pack API leaks into settings, UI, and the doc format |
| `app/cleanshot/src/editor/icon_assets.rs` | 60-entry `include_bytes!` table | Generated, must be regenerated |
| `app/cleanshot/src/editor/window.rs` `icon_for()` (~line 3225) | Toolbar tool icons **hand-drawn as raw PathBuilder polygons** per tool | Bespoke primitives — inconsistent stroke weights, no optical alignment, hardest code in the app to keep looking right |
| `app/cleanshot/src/editor/window.rs` selection bar | Text glyphs `"◀ Back"`, `"Forward ▶"`, `"× Delete"` | Unicode punctuation as icons |
| `scripts/generate-editor-icons.mjs` | Generates the 5-pack PNGs via Playwright | To be replaced |
| `package.json` | `lucide-react`, `@phosphor-icons/react`, `@tabler/icons-react`, `@heroicons/react`, `@remixicon/react` deps | To be deleted |
| `src/`, `src-tauri/` (legacy Tauri tree, still on main) | Any stray icon assets | Out of scope here; purge when the legacy tree is dropped |

Also current: `IconPack`/`IconGlyph` are serialized into saved annotations (`Shape::Icon`).
The new doc format should store just `{ icon_id, color, size }`; old docs render with a fallback
(see design-spec.md §Compatibility).

## Mapping

Icon names are HugeIcons free-set exports from `@hugeicons/core-free-icons`
(24×24, stroke 1.5, `currentColor`). All names below verified to exist in the package.

### A. Editor toolbar tools (replace hand-drawn `icon_for` paths)

| Tool (today) | HugeIcons icon |
|---|---|
| Select | `CursorRectangleSelectionIcon` |
| Rectangle | `SquareIcon` |
| Ellipse | `CircleIcon` |
| Arrow | `ArrowUpRight01Icon` |
| Pen | `Pen01Icon` |
| Text | `TypeIcon` |
| Counter | `OneSquareIcon` |
| Crop | `CropIcon` |
| Blur | `BlurIcon` |
| Pixelate | `Grid2x2Icon` |
| Redact | `MaskIcon` |
| Icon sticker | `StickerIcon` |

### B. Editor actions (replace text buttons / unicode glyphs)

| Control (today) | HugeIcons icon |
|---|---|
| Undo | `UndoIcon` |
| Redo | `RedoIcon` |
| Copy | `Copy01Icon` |
| Save PNG | `Download04Icon` (button keeps its text label) |
| History rail | `HistoryIcon` |
| Preferences | `Settings01Icon` |
| Send backward ("◀ Back") | `LayerSendBackwardIcon` |
| Bring forward ("Forward ▶") | `LayerBringForwardIcon` |
| Delete ("× Delete") | `Delete02Icon` |
| Close editor / cancel | `Cancel01Icon` |
| Zoom in / out | `ZoomInIcon` / `ZoomOutIcon` |
| Fit to window | `Maximize01Icon` |
| Apply crop | `TickIcon` |
| Refresh library | `RefreshIcon` |

### C. Sticker library on canvas (replaces 5 packs × 12 glyphs = 60 PNGs with one curated set)

| Purpose (old glyph) | HugeIcons icon |
|---|---|
| Arrow right | `ArrowRight01Icon` |
| Check | `TickIcon` |
| Close | `Cancel01Icon` |
| Info | `InformationCircleIcon` |
| Warning | `Alert02Icon` |
| Question | `HelpCircleIcon` |
| Star | `StarIcon` |
| Heart | `HeartIcon` |
| Lock | `LockIcon` |
| Eye | `ViewIcon` |
| Person | `UserIcon` |
| Camera | `Camera01Icon` |
| Flag *(new)* | `Flag01Icon` |
| Pin *(new)* | `PinIcon` |
| Thumbs up *(new)* | `ThumbsUpIcon` |
| Clock *(new)* | `Clock01Icon` |
| Mail *(new)* | `Mail01Icon` |
| Smile *(new)* | `SmileIcon` |
| Bookmark *(new)* | `BookmarkIcon` |
| Check circle *(new)* | `CheckmarkCircleIcon` |
| Cross circle *(new)* | `CancelCircleIcon` |
| Plus *(new)* | `AddCircleIcon` |
| Minus *(new)* | `MinusSignIcon` |
| Scissors *(new)* | `ScissorsIcon` |

### D. Other surfaces

| Surface | Control | HugeIcons icon |
|---|---|---|
| Quick access card | Copy / Save / Annotate / Pin / Close | `Copy01Icon`, `Download04Icon`, `Pen01Icon`, `PinIcon`, `Cancel01Icon` |
| History rail | Search / refresh / row actions | `Search01Icon`, `RefreshIcon`, `Copy01Icon`, `PinIcon`, `Delete02Icon` |
| Preferences nav | General / Capture / Editor / Hotkeys / History | `Settings01Icon`, `Camera01Icon`, `Pen01Icon`, `TypeIcon`, `HistoryIcon` |
| Window picker | Window / screen source | `MonitorIcon`, `WebcamIcon` |
| Pin window | Opacity ±, click-through, copy, save, close | `AddCircleIcon`, `MinusSignIcon`, `MouseIcon`, `Copy01Icon`, `SaveIcon`, `Cancel01Icon` |

The tray menu stays native text (Windows tray menus don't take icons per item); the tray *app
icon* is brand art, not part of an icon set.

## Purge list — delete all of this in the implementation phase

1. `app/cleanshot/assets/icons/lucide/`, `phosphor/`, `tabler/`, `heroicons/`, `remix/` (all PNGs)
2. `app/cleanshot/assets/icons/licenses/` (per-pack notices; replaced by one HugeIcons MIT notice)
3. `app/cleanshot/src/editor/icon_assets.rs`
4. In `app/cleanshot/src/editor/icons.rs`: the `IconPack` enum and the pack parameter of
   `rasterize()` (module stays, single-set)
5. `scripts/generate-editor-icons.mjs`
6. `package.json`: `lucide-react`, `@phosphor-icons/react`, `@tabler/icons-react`,
   `@heroicons/react`, `@remixicon/react`
7. Any other icon asset under the legacy Tauri tree (`src/`, `src-tauri/`) when that tree is
   removed — flagged here so the purge is total at that point

After implementation, `grep -ri "lucide\|phosphor\|tabler\|heroicons\|remix"` over the repo must
return nothing outside git history.

## Bundling rules

- New generator: `scripts/generate-hugeicons-icons.mjs` (committed in this PR) renders the
  mapped icons from `@hugeicons/core-free-icons` to `app/cleanshot/assets/icons/hugeicons/*.svg`
  and sticker PNGs at 512 px. Output is committed; the generator is re-run only when the
  mapping changes.
- Keep `hugeicons.LICENSE` (MIT notice) under `app/cleanshot/assets/icons/`.
- Never commit Pro-tier icon files. If Pro is bought later, its files live in the packaging
  pipeline, not in git.
