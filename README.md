# Snipsy

Snipsy is a local-first Windows screenshot tool for capturing, annotating,
and sharing screenshots without getting in the way.

Capture something. Mark it up. Copy it where it needs to go. Find it again later.

> Snipsy is in active development. The native Windows app is implemented,
> but a public release still needs final testing on real Windows hardware.

## The workflow

### 1. Capture

Capture an area, a window, or the full virtual desktop. The capture overlay keeps
selection quick and focused, then returns you to the editor with the result.

### 2. Mark it up

The editor keeps the screenshot at the center of the experience. A toolbar
along the bottom holds annotation tools, undo and redo, copy, and save actions.
History, icon selection, and preferences open in side panels.

Use drawing, shapes, arrows, text, numbered steps, highlights, blur, pixelation,
and redaction to make the image clear.

### 3. Share the result

Copy the annotated image, copy it as a file, or save a PNG. Sharing uses the
Windows clipboard and files. There are no hosted share links or accounts.

### 4. Find it later

Snipsy keeps a local capture history with thumbnails, titles, and title
search. You can reopen captures, continue editing them, or pin them above other
windows for reference.

## What it includes

| Area | Current capabilities |
|---|---|
| Capture | Area, window, and full-screen capture; cursor inclusion; loupe and physical-pixel sizing |
| Annotation | Drawing, shapes, arrows, text, counters, highlights, blur, pixelation, and redaction |
| Output | Copy image, copy file, and save PNG |
| History | Thumbnails, title editing, title search, reopen, delete, and annotation persistence |
| Windows | Global capture shortcut, tray controls, single-instance behavior, startup option, and always-on-top pins |

## Private by default

- Windows 10 and 11 are the supported platforms.
- Captures, annotations, settings, and history stay on your machine.
- No account, cloud sync, telemetry, or hosted library is required.
- OCR is available as a separate local Rust tool in `ocr/`; it is not yet
  integrated into the editor.
- The app is designed for standard-user operation. Administrator access is not
  required for normal use.

## Current status

The native GPUI app and editor are implemented. The remaining pre-release work
is manual Windows acceptance testing, including mixed-DPI monitors, window
capture, clipboard behavior, tray and startup behavior, and pin windows.

There is no public installer yet. Scrolling capture, integrated OCR, richer
library search, and a user-facing project-file format are planned for later
work. Video recording, cloud sharing, accounts, macOS, and Linux are outside
the current v1 direction.

## Project documentation

The [documentation index](docs/README.md) collects the project references:

- [Product and scope](docs/PRODUCT.md)
- [Design system](docs/DESIGN.md)
- [Roadmap and status](docs/ROADMAP.md)
- [Changelog](docs/CHANGELOG.md)
- [Engineering notes](docs/ENGINEERING.md)

## License

Snipsy is released under the MIT License.
