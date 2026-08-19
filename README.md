# CleanShot W

CleanShot W is a local-first Windows screenshot and annotation app. Capture an area, window, or full screen, mark it up, copy or export the result, and find it again later. No account, cloud sync, telemetry, or admin rights are required.

## Project status

The M2 native shell is implemented and repository hardening is complete. The remaining M2 gate is manual Windows validation on representative hardware, especially a mixed-DPI two-monitor setup. There is not a public release yet.

- [Roadmap](ROADMAP.md) — current milestones, decisions, and next work
- [Changelog](CHANGELOG.md) — completed and release-visible changes
- [Engineering notes](docs/ENGINEERING.md) — architecture, commands, and packaging
- [Windows M2 checklist](docs/WINDOWS-M2-RELEASE-CHECKLIST.md) — manual release acceptance

## Features

- Area, window, and full-screen capture in the Windows app
- Paste, drag-and-drop, and file-based image intake in the browser editor
- Annotation tools for drawing, shapes, arrows, text, counters, highlights, blur,
  pixelation, and redaction
- Zoom, pan, undo, redo, and editable annotations
- Copy image, copy file, save PNG, and copy recognized text
- Local OCR with bundled Tesseract assets
- Capture history with thumbnails, markup counts, timestamps, editable titles,
  title search, and persistence between sessions
- Always-on-top pin windows, system-tray controls, configurable capture hotkey,
  cursor inclusion, and optional launch at startup in the Windows app

## Getting started

There is no public installer yet. To run the browser editor from source:

```sh
bun install --frozen-lockfile
bun run dev
```

Open the URL printed by Vite. The browser editor accepts an image by pasting it
with `Ctrl+V`, dropping it onto the editor, or choosing a file.

To run the native Windows shell, use a Windows development environment with the
Rust GNU toolchain installed:

```sh
bun run tauri dev
```

See the [engineering notes](docs/ENGINEERING.md) for the full toolchain and
verification commands.

## Capturing

- **Area:** choose **New capture** or press `Ctrl+Shift+4`, then drag over the
  region you need.
- **Window:** choose **Window**, select a visible titled window, and capture it.
- **Full screen:** choose **Full screen** to capture the complete virtual desktop.
- **Clipboard or file:** paste, drop, or open an image in the browser editor.

The native app hides the editor while capturing and restores it after the
capture completes or is cancelled.

## Annotating

Use the toolbar to select, draw, add shapes and arrows, place text or numbered
steps, highlight content, blur or pixelate sensitive areas, and redact content
with an opaque block. Hold `Space` to pan, use the mouse wheel to zoom, and use
`0` to reset the zoom.

Common keyboard shortcuts:

| Key | Tool |
|---|---|
| `V` | Select/move |
| `R` | Rectangle |
| `O` | Ellipse |
| `A` | Arrow |
| `L` | Line |
| `T` | Text |
| `C` | Counter |
| `M` | Note |
| `G` | Highlight |
| `D` | Freehand draw |
| `Q` | Blur |
| `K` | Pixelate |
| `E` | Eraser |
| `F` | Frame |
| `H` | Laser |

`Ctrl+Z` undoes an edit and `Ctrl+Shift+Z` redoes it.

## Export and OCR

The Quick Access bar can flatten the annotated image and:

- Copy it as an image with transparency preserved
- Copy it as a PNG file in the native app
- Save it as a PNG from the browser editor
- Run local OCR and copy the recognized text

OCR uses bundled local assets and does not require network access. It works best
with clear, printed English text.

## Capture history

History appears in the right sidebar. Select a capture to reopen it. Click its
title to rename it; press Enter or leave the field to save, or press Escape to
cancel. Search filters titles case-insensitively. History, annotations, images,
and titles persist locally in IndexedDB in the browser and under
`%LOCALAPPDATA%\CleanShotW` in the native app.

## Windows features

The native shell adds a global capture shortcut, tray actions, single-instance
activation, optional per-user startup, cursor inclusion, window capture, and
always-on-top pins. The app is designed to run as a standard user. WebView2 is
provided by Windows 10 and 11.

These behaviors need real Windows validation before release. Use the [M2
checklist](docs/WINDOWS-M2-RELEASE-CHECKLIST.md) rather than treating browser
checks as a substitute for DPI, clipboard, tray, or packaged-app testing.

## Development checks

The project uses Bun, React, Vite, TypeScript, tldraw, Tesseract.js, and Tauri.
Common checks are:

```sh
bun test
bun tsc -b --noEmit
bun run build
```

The browser smoke test requires a running Vite server and Playwright Chromium:

```sh
bun run dev -- --host 127.0.0.1
bun run smoke
```

Native verification is run on the Windows CI job:

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## License

MIT. Free to use, modify, and distribute.
