# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Upgraded TypeScript `~5.8.3` → `~7.0.2` (native compiler). Build verified.
- Replaced the marketing-style "Capture Studio" mock with a working editor shell.
- **Fixed drawing/selection interactions dropping or sticking.** The editor now
  calls `setPointerCapture` on pointer-down so pointer-move/pointer-up keep
  reaching the viewport even when the cursor leaves it (e.g. releasing the mouse
  over the toolbar/status bar). Added an `e.buttons` guard that cancels a
  stranded interaction and an `onPointerCancel` reset. Previously a drag that
  ended outside the viewport left a stuck interaction that made subsequent
  draws act like the image was being grabbed/moved. Verified with real-input
  Playwright drag tests.
- M1 is complete. Tool defaults now persist in validated localStorage settings,
  and clipboard, export, and OCR failures report an actionable message in the editor.

### Added

- **M0/M1 web editor** (browser-only prototype):
  - Capture intake via paste, drag-drop, and file picker
  - Annotation tools: select/move, rect, ellipse, arrow, line, text, counter,
    highlight, blur, pixelate, redact
  - Properties (color, stroke width, font size), zoom/pan, undo/redo
  - Select/resize/move editing with handles, inline text editing
  - OCR ("Copy text") via tesseract.js with locally bundled wasm + traineddata
  - Export flattened PNG, copy image to clipboard, copy text
  - IndexedDB history with thumbnails and re-editing (debounced autosave)
  - Quick Access floating action bar (Copy image / OCR / Save PNG / New / Close)
  - `scripts/ocr-assets.mjs` (`bun run ocr:assets`) — local tessdata bundle
  - Geometry library + unit tests (`bun test`)
  - Browser smoke harness (`node scripts/smoke-tldraw.mjs`) with 32 acceptance checks

### Planned

- **M2** — Native Windows host (Tauri, Rust + WebView2):
  - Global hotkeys, transparent selection overlay with magnifier
  - Area / window / fullscreen capture; copy image (alpha-safe) and copy file
  - Disk library under `%LOCALAPPDATA%\CleanShotW`, tray icon, pin windows
- **M3** — Scrolling capture (beta), native OCR, library search, first public
  portable ZIP release with SHA-256 checksums.

### Decisions

- Tauri is retained as the app shell (editor stays shell-agnostic; M1
  develops entirely in the browser). — [ROADMAP.md](./ROADMAP.md)
- Name stays **CleanShot W** for now; planned rename to **ShutterW** before the
  first public release (name collides with the macOS CleanShot product).
- **Tauri builds with the GNU toolchain — MSVC is not required.**
  Verified 2026-08-14: `x86_64-pc-windows-gnu` + MinGW GCC produced
  `src-tauri/target/release/cleanshot-w.exe` (~15 min first build).

## [0.1.0] — 2026-08-14 (not yet released)

### Added

- Initial Tauri + React + TypeScript scaffold.
- "Capture Studio" static mock screen (sidebar, mode cards, recent captures).
