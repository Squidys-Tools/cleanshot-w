# Engineering

This is the stable implementation reference for contributors. Planned work
belongs in [the roadmap](ROADMAP.md), completed changes belong in [the
changelog](CHANGELOG.md), product decisions belong in [the product brief](PRODUCT.md),
and visual rules belong in [the design system](DESIGN.md).

## Workspaces

- `app/` is the GPUI desktop application workspace. Its `snipsy` binary runs the
  capture app, and `snipsy-editor` opens an image in the same editor window for
  standalone testing.
- `ocr/` is a separate Rust OCR evaluation CLI. It supports Tesseract and a
  Windows OCR comparison backend, but the editor does not invoke it yet.
- `spikes/` contains standalone GPUI experiments and is built by its own
  workflow.

The desktop app stores captures, annotations, settings, and history locally
under `%LOCALAPPDATA%\Snipsy`. Its Rust modules separate screen capture,
virtual-screen geometry, hotkeys, tray integration, pin windows, the capture
library, and the editor. GPUI owns the windows and UI; Windows APIs provide
capture, DPI awareness, global shortcuts, and startup registration.

## Capture and editor flow

The app snapshots the virtual desktop before showing the selection overlay, so
the overlay cannot appear in a capture. A selection is converted from GPUI
logical coordinates to physical pixels, cropped, encoded as PNG, saved in the
local library, and opened in the editor. Window capture uses `PrintWindow` with
a GDI fallback.

The editor stores an app-owned, versioned JSON document beside each capture.
Shapes use original image coordinates, and undo/redo stores document snapshots.
Export rasterizes the image and annotations at the capture's original
dimensions before saving or copying the result. Older annotation documents
using the former format are not imported; those captures open without their
annotations.

The library keeps an index and per-capture image, thumbnail, and annotation
files. It validates ids before using them as paths and writes updated data
atomically where the platform permits.

## Windows constraints

- The process enables Per-Monitor V2 DPI awareness before reading physical
  display dimensions.
- Virtual-screen origins can be negative when a monitor sits above or left of
  the primary display.
- GDI and `PrintWindow` behavior varies by application and needs hardware
  validation. Protected content and the secure desktop cannot be captured.
- The CI Windows build validates compilation and tests; it does not replace
  real-device checks for DPI, clipboard interoperability, tray behavior, and
  pin lifecycle.

## Verification

Run the app checks from the repository root:

```sh
cargo fmt --manifest-path app/Cargo.toml -- --check
cargo clippy --manifest-path app/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path app/Cargo.toml
cargo check --manifest-path app/Cargo.toml
```

The app workflows run Windows build and Linux checks for changes under `app/`.
The OCR workflow separately checks `ocr/`, and the spike workflow checks
`spikes/gpui-overlay/`. `scripts/check-rust.mjs` runs the same app formatting,
Clippy, test, and compile checks.

To download the English Tesseract model for local OCR runs, execute
`node scripts/ocr-assets.mjs`. The script writes it to `ocr/dist/tessdata`;
pass that directory to `snipsy-ocr` with `--tessdata-dir`.
