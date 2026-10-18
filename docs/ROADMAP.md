# Roadmap and status

Snipsy is a native GPUI screenshot and annotation app for Windows. The current
implementation is in `app/`; `ocr/` remains a standalone OCR evaluation tool
and is not integrated into the editor.

## Current status

The GPUI app includes area, window, and full-screen capture; an annotation
editor; local capture history; image and file clipboard output; PNG export;
tray controls; global capture hotkeys; startup registration; and always-on-top
pin windows. The editor supports shapes, text, drawing, effects, crop, undo and
redo, and history/preferences. Captures and settings remain local.

Automated checks cover the app's Rust formatting, lint, tests, and compilation.
The Windows workflow also builds both binaries for hardware testing. A fresh
manual acceptance pass on the merged app is still needed, particularly for
mixed-DPI and multi-monitor capture, window capture, clipboard behavior, tray
and startup behavior, and pin lifecycle. There is no public installer yet.

## Next work

1. Run the merged app artifact on representative Windows hardware and record
   the build, Windows version, display configuration, and any failures.
2. Fix issues found in that pass before preparing a public release.
3. Decide separately whether to integrate OCR, add scrolling capture, expand
   library search, or introduce a user-facing project-file format.

Video recording, cloud sharing, accounts, macOS, and Linux remain outside the
current v1 direction.

## Verification

Run the app checks from the repository root:

```sh
cargo fmt --manifest-path app/Cargo.toml -- --check
cargo clippy --manifest-path app/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path app/Cargo.toml
cargo check --manifest-path app/Cargo.toml
```

The app CI workflows run on changes under `app/`; OCR has its own workflow for
`ocr/`, and the GPUI overlay experiment has a separate workflow under
`spikes/gpui-overlay/`.
