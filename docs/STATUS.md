# Project Status

> **Last updated:** 2026-08-18
> **Current version:** 0.1.0 (unreleased)
> **Current milestone:** M2 — Native Shell (in progress)

---

## At a glance

| Milestone | Status | Summary |
|---|---|---|
| M0 — Baseline | Done | Scaffold, TS upgrade, tesseract assets, IndexedDB, editor shell |
| M1 — Web Editor | Done | Full annotation toolset, OCR, export, library, tests passing |
| **M2 — Native Shell** | **In Progress** | 6/10 items done, 4 deferred |
| M3 — Polish & Scale | Planned | Scroll capture, native OCR, search, first release |

---

## M0 — Baseline [DONE]

- [x] Kept Tauri scaffold (deps, `src-tauri/`, Tauri-flavored Vite config)
- [x] Upgraded TypeScript `~5.8.3` → `~7.0.2` (native compiler)
- [x] Added `tesseract.js` with locally bundled wasm + `eng` traineddata
- [x] Hand-rolled IndexedDB storage layer (no ORM/state library)
- [x] Replaced marketing-style studio screen with working editor shell

## M1 — Web Editor Prototype [DONE]

### Intake
- [x] Paste, drag-and-drop, file picker

### Tools
- [x] Select/move (V), rect (R), ellipse (O), arrow (A), line (L)
- [x] Text (T), counter (C), highlight (H), blur (B), pixelate (X), redact (K)

### Properties & Interaction
- [x] Stroke color, width, fill; arrowhead size; font size/color; effect strength
- [x] Tool defaults persist in localStorage with safe fallbacks
- [x] Click-drag to draw, Shift constrains aspect, Esc cancels
- [x] Drag to move, resize handles, double-click text to edit, Delete removes
- [x] Zoom/pan (`+`/`-`/`0`, wheel with Ctrl, middle/Alt-drag pan)
- [x] Pointer capture fix — interactions don't drop or stick when cursor leaves viewport

### Undo/Redo
- [x] Ctrl+Z / Ctrl+Shift+Z, unlimited depth, 20+ ops verified

### OCR
- [x] "Copy text (OCR)" via tesseract.js with bundled local assets
- [x] Word bounding boxes for future OCR layer

### Export
- [x] PNG save, copy image to clipboard (flattened at natural res), copy text

### Library
- [x] IndexedDB history with thumbnails, open → re-edit, autosave

### Quick Access
- [x] Floating action bar: Copy image / OCR / Save PNG / New / Close

### Testing
- [x] `bun test`: 12 unit tests pass
- [x] `bun run build`: TypeScript and Vite production build pass
- [x] `bun run smoke`: 32 browser checks pass

## M2 — Native Shell (Tauri, Rust) [IN PROGRESS]

### Implemented items

- [x] **Item 1 — Selection overlay:** Native transparent Tauri window with pixel
  loupe and validated natural-pixel crop coordinates
- [x] **Item 3 — Window capture:** `EnumWindows` + `PrintWindow` with screen
  capture fallback
- [x] **Item 4 — Native clipboard:** Image (DIB v5, alpha-safe), text, and file
  output via `arboard`
- [x] **Item 5 — Disk library:** Capture storage under `%LOCALAPPDATA%\CleanShotW`
- [x] **Item 6 — Global hotkeys:** Persisted capture shortcut with conflict
  detection and settings remapping
- [x] **Item 8 — Single instance:** Brings existing editor window to foreground

### Deferred items (next up)

- [ ] **Item 2 — Mixed-DPI validation:** Deferred to a dedicated testing pass
- [ ] **Item 7 — Tray icon:** Deferred; needs package/design decision
- [ ] **Item 9 — Autostart:** `HKCU\...\Run` (no admin), deferred until rest
  of M2 is done
- [ ] **Item 10 — Cursor inclusion in capture:** Toggleable, deferred

### M2 risk spikes

| Spike | Status |
|---|---|
| Transparent overlay across mixed-DPI | Pending |
| `xcap`/GDI capture on mixed DPI | Pending |
| Copy image with alpha into Word/Slack | Pending |
| Tesseract wasm under Tauri asset protocol | Pending |
| `PrintWindow` on occluded apps | Pending |
| tesseract.js speed/quality benchmark | Pending |
| ~~Tauri build without MSVC~~ | Done (2026-08-14) |

## M3 — Polish and Scale [PLANNED]

- [ ] Scroll capture: SendInput-based scrolling + stitching (ship as beta)
- [ ] In-process native OCR (bundled Tesseract 5 + tessdata, no system install)
- [ ] Library search by OCR text; tag/title editing
- [ ] First public release: portable ZIP (or per-user NSIS) + SHA-256 checksums
- [ ] Optional: update checker (compare version file over HTTPS)
- [ ] Rename CleanShot W → ShutterW (before first public release)

---

## Decisions log

| # | Decision | Status |
|---|---|---|
| 1 | Keep Tauri as the app shell; editor stays shell-agnostic | Decided |
| 2 | Rename CleanShot W → ShutterW before first public release | Deferred |
| 3 | Tauri builds with GNU toolchain — no MSVC required | Decided (verified 2026-08-14) |
| 4 | Scroll capture: M3 beta | Proposed |
| 5 | .NET/Tauri channel: N/A — shell is Rust/Tauri | Decided |

## Non-goals (v1)

- Video / GIF / screen recording
- Cloud sync, accounts, sharing URLs
- Installer auto-update, code signing
- Linux/macOS ports
- Editing capture in place on the full-screen overlay (stretch)
