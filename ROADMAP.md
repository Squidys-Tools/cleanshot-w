# CleanShot W roadmap

This is the source of truth for product direction and milestone status. The
[README](README.md) is the user and contributor entry point, the
[CHANGELOG](CHANGELOG.md) records completed changes, and
[docs/ENGINEERING.md](docs/ENGINEERING.md) holds stable implementation details.
The [Windows M2 checklist](docs/WINDOWS-M2-RELEASE-CHECKLIST.md) is the manual
acceptance record and is not a second roadmap.

## Current position

| Milestone | Status | Meaning |
|---|---|---|
| M0 — baseline | Done | The Tauri scaffold and browser foundation were replaced with a working editor. |
| M1 — web editor | Done | Browser intake, annotation, OCR, export, and local history work end to end. |
| M2 — native shell | Implementation complete | Windows capture, tray, hotkeys, disk library, clipboard, pinning, and hardening are implemented. |
| M2 release gate | In progress | Real Windows validation is still required before calling the app release-ready. |
| M3 — polish and scale | Next | Scrolling capture beta, native OCR, broader library search, and first public release work. |

### Immediate order of work

1. Run the [M2 Windows checklist](docs/WINDOWS-M2-RELEASE-CHECKLIST.md) on
   Windows 10/11 hardware, including 100% + 150% mixed-DPI monitors.
2. Fix any release-gate issues found by that validation and record the evidence.
3. Start M3 with scrolling capture behind an explicit beta flow.
4. Package the first public release once the native shell and M3 release scope
   have a tested acceptance path.

## Product goal

Build a focused, native-feeling Windows screenshot tool that keeps captures and
annotations on the user's machine:

- Capture an area, window, or full virtual desktop.
- Annotate the result with shapes, text, effects, and numbered steps.
- Copy, export, or OCR the result.
- Reopen and search local history without an account or cloud service.

## Milestones

### M0 — Baseline `[DONE]`

- Kept the Tauri application shell and Vite/React/TypeScript frontend.
- Replaced the static studio mock with a working editor shell.
- Added local IndexedDB storage and bundled Tesseract.js assets.
- Established the typed browser/native host boundary.

### M1 — Web editor `[DONE]`

- Paste, drag-and-drop, and file-picker image intake.
- Annotation tools, properties, zoom/pan, selection, text editing, and
  undo/redo.
- Flattened PNG export, image/text clipboard actions, and local OCR.
- IndexedDB history with thumbnails, annotation autosave, reopen, title editing,
  and case-insensitive title search.
- Browser tests and the Playwright smoke harness cover the editor workflow.

### M2 — Native Windows shell `[IMPLEMENTATION COMPLETE]`

Implemented in the repository:

- Per-Monitor V2-aware area capture with a transparent selection overlay, loupe,
  physical-pixel crop validation, and negative virtual-screen origins.
- Window enumeration and `PrintWindow` capture with a GDI fallback.
- Full-screen capture, optional cursor compositing, and native image/text/file
  clipboard output.
- Disk-backed library under `%LOCALAPPDATA%\CleanShotW`, including annotations,
  thumbnails, title updates, safe index validation, and atomic writes.
- Global capture hotkey, settings persistence, conflict errors, per-user
  `HKCU` startup registration, tray actions, and single-instance activation.
- Always-on-top pin windows with bounded sizing, aspect-ratio preservation, and
  lifecycle cleanup.
- User-facing native error normalization, PNG payload validation, current-user
  NSIS configuration, and automated frontend/Rust coverage.

The implementation is complete, but the release gate remains manual. Verify
mixed-DPI geometry, GDI/window behavior, clipboard alpha, packaged OCR assets,
tray/autostart, single-instance behavior, and pin cleanup using the checklist.

### M3 — Polish and scale `[PLANNED]`

Priority order:

1. **Scrolling capture (beta):** controlled scrolling, stitched output, bounded
   failure/cancel behavior, and coordinate/stitching tests. Keep it opt-in until
   it is reliable across common applications.
2. **Native OCR:** evaluate bundled native Tesseract for packaged Windows use;
   retain the browser OCR path where it is still the better fallback.
3. **Library search expansion:** search OCR text and add tags after title search
   is stable.
4. **First public release:** choose portable ZIP and/or per-user NSIS, publish
   SHA-256 checksums, document SmartScreen behavior, and write release notes.
5. **Optional update checker:** compare a version file over HTTPS but keep
   downloads and installation user-controlled.

## Decisions

| Decision | Status |
|---|---|
| Tauri remains the Windows shell; the editor stays shell-agnostic through a typed host bridge. | Decided |
| The app is local-first: no accounts, cloud sync, sharing URLs, or telemetry. | Decided |
| The Windows native build uses the GNU Rust toolchain; MSVC is not required. | Decided; verified in the Windows build environment |
| Keep the name **CleanShot W** for now; revisit **ShutterW** before the first public release because of the macOS product name collision. | Decided — defer rename |
| Scrolling capture ships as an M3 beta rather than blocking the native shell release gate. | Proposed |
| No auto-update in v1. | Decided |

## Release gates

Before publishing a release:

- Frontend tests, typecheck, production build, and browser smoke checks pass.
- Rust formatting, Clippy, tests, and `cargo check` pass on Windows CI.
- The manual M2 checklist is completed against the release artifact.
- Known limitations and any follow-up issues are recorded with the release
  decision.

The exact commands and CI jobs are documented in
[docs/ENGINEERING.md](docs/ENGINEERING.md). Do not mark M2 release validation
complete based only on browser checks.

## Non-goals for v1

- Video, GIF, or screen recording
- Cloud sync, accounts, public sharing, or hosted libraries
- Installer auto-update or code signing
- Linux/macOS ports
- Full-screen overlay annotation before a capture is committed
