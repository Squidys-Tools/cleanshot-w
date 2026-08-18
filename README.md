# CleanShot W

A screenshot tool for Windows. Capture your screen, annotate it, copy it to your clipboard. No cloud, no accounts, no telemetry. Everything stays on your machine.

Think of it as a Windows equivalent to macOS's CleanShot X.

## Getting it

Download the latest release and run the installer. No admin rights needed. Works on Windows 10 and 11.

- **Installer** (`CleanShotW-0.1.0-win64-setup.exe`) installs per-user only. No UAC prompt.
- **Portable ZIP** (`CleanShotW-0.1.0-win64.zip`) runs without installation.

## How to use it

**Take a screenshot.** Press `Ctrl+Shift+4` (you can change this in settings). Your cursor turns into a crosshair. Click and drag to select an area. The screenshot opens in the editor immediately.

**Capture a window.** Instead of dragging, right-click and pick a window from the list. The tool grabs that window precisely, even on mixed-DPI setups.

**Capture your full screen.** Press the shortcut and pick "Full screen" instead of selecting an area.

**Paste anything.** If you already have an image on your clipboard, just paste it into the editor with `Ctrl+V`. Drag-and-drop and file picking work too.

## The annotation editor

Once you have a screenshot, the editor lets you mark it up before sharing. Here's what you can do:

**Draw on it.** Freehand drawing, arrows, lines, rectangles, ellipses, and 19 other geometric shapes. Pick a color from the 8-color palette and a stroke width (S/M/L/XL).

**Add text.** Four font families: sans-serif, serif, monospace, and hand-drawn. Choose your size, color, and alignment.

**Blur or pixelate.** Draw over sensitive information. The blur tool smooths it out; pixelate turns it into blocks.

**Redact.** Solid black box over whatever you want hidden. No recovery possible from the exported image.

**Number your steps.** The counter tool stamps sequential numbers onto the image. Useful for tutorials or bug reports.

**Add arrows and callouts.** Nine arrowhead styles including dots, diamonds, and triangles. Arrows can be straight or curved.

**Highlight.** A semi-transparent brush that marks areas without covering the content underneath.

**Undo everything.** `Ctrl+Z` goes back. `Ctrl+Shift+Z` goes forward. There's no limit.

### Keyboard shortcuts

| Key | Tool |
|-----|------|
| V | Select/move |
| R | Rectangle |
| O | Ellipse |
| A | Arrow |
| L | Line |
| T | Text |
| C | Counter |
| M | Note |
| G | Highlight |
| D | Draw (freehand) |
| Q | Blur |
| K | Pixelate |
| E | Eraser |
| F | Frame |
| H | Laser |

Hold `Space` to pan. Scroll wheel zooms. `0` resets zoom.

## Export and share

When you're done annotating:

- **Copy to clipboard.** The flattened image goes straight to your clipboard, ready to paste into any app. Uses alpha-safe formatting so transparency works correctly.
- **Copy as file.** Copies a `.png` file you can paste into File Explorer or drag into an email.
- **Save as PNG.** Downloads the image to your downloads folder.

## Pin to screen

The pin button floats the screenshot as an always-on-top window. Useful when you need to reference something while working in another app. Drag it around, close it when you're done.

## Capture history

Every screenshot you take shows up in a sidebar on the right. Thumbnails, markup counts, and timestamps. Click any capture to reopen it in the editor. Delete ones you don't need. The history persists between sessions.

## OCR (text recognition)

Click the OCR button and the tool reads any text visible in the screenshot. Words come back with bounding boxes so you know exactly what was recognized. Copy the extracted text to your clipboard.

Works best with clear, printed text in English. Handwriting and unusual fonts may not work well.

## Settings

Open settings from the system tray icon or the editor:

- **Capture hotkey.** Change from the default `Ctrl+Shift+4` to whatever you prefer.
- **Include cursor.** Toggle whether the mouse cursor appears in captures.
- **Launch at startup.** Start CleanShot W when you log in. Runs minimized to tray.
- **Close to tray.** Closing the window hides it to the system tray instead of quitting.

Settings are stored in `%LOCALAPPDATA%\CleanShotW\settings.json`.

## System tray

The tray icon gives you quick access:

- **New capture.** Same as pressing the hotkey.
- **Open library.** Brings the editor to the foreground.
- **Quit.** Exits the app.

The app runs as a single instance. If you try to open a second copy, it brings the existing window forward instead.

## How it runs

CleanShot W does not require admin rights for anything. Installation, autostart (via `HKCU` registry), hotkeys, and all capture features work under a standard user account. It uses WebView2, which ships built into Windows 10 and 11.

## Development

Built with Tauri 2 (Rust + WebView2), React 19, TypeScript 7, and tldraw.

```bash
bun install
bun run dev        # browser-only editor
bun run tauri dev  # full Tauri shell
```

## Documentation

All docs live in [`docs/`](./docs/):

| Doc | Description |
|---|---|
| [Status](docs/STATUS.md) | **Start here** — current milestone, what's done, what's next |
| [Roadmap](docs/ROADMAP.md) | Full plan and milestones |
| [Architecture](docs/ARCHITECTURE.md) | hostBridge, data model, rendering, capture pipeline |
| [Changelog](docs/CHANGELOG.md) | Every notable change |
| [Building](docs/BUILDING.md) | Dev, build, and test commands |
| [Distribution](docs/DISTRIBUTION.md) | Packaging and release process |

## License

MIT. Free to use, modify, and distribute.
