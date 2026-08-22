# CleanShot W roadmap and status

> Last updated: 2026-08-22
> Current version: 0.1.0 (unreleased)

This is the single source of truth for product direction, milestone status, and
the Windows M2 release gate. Product promises and feature gaps live in
[Product](PRODUCT.md). Stable implementation details live in
[Engineering](ENGINEERING.md). Visual rules live in [Design](DESIGN.md).
Completed changes live in [the changelog](CHANGELOG.md).

## Current position

| Milestone | Status | Meaning |
|---|---|---|
| M0 - baseline | Done | The Tauri foundation and browser editor are in place. |
| M1 - web editor | Done | Intake, annotation, OCR, export, local history, and tests work. |
| M2 - native shell | Implementation complete | Windows capture, tray, hotkeys, disk history, clipboard, pinning, and hardening are implemented. |
| M2 release gate | In progress | Manual Windows validation is still required before release. |
| M3 - polish and scale | Next | Scrolling capture beta, native OCR evaluation, broader search, and the first public release. |

## What works

- Area, titled-window, and full-screen capture in the Windows shell.
- Paste, drag-and-drop, and file intake in the browser editor.
- Editable drawing, shapes, arrows, text, numbered steps, highlights, blur,
  pixelation, and redaction.
- Zoom, pan, selection, undo, redo, PNG export, image/file clipboard output,
  local OCR, and capture history.
- Native tray actions, configurable capture hotkey, optional cursor inclusion,
  per-user startup, single-instance activation, and always-on-top pins.
- A screenshot-first editor layout with a text-led command bar, detached
  annotation dock, and flyout history. The rules are in [Design](DESIGN.md).

## Verification

The current repository checks pass with TypeScript 7:

```text
TypeScript 7 check: pass
Vite production build: pass
Browser unit tests: 18 pass
```

The TypeScript 7 compatibility setting is documented in `tsconfig.json` and
exists because tldraw 5.3.1 publishes declaration files without a `types`
condition in its package export map. These browser checks do not replace the
Windows acceptance checks below.

## Immediate order of work

1. Run the Windows acceptance checks in this document on Windows 10 and 11
   hardware, including 100% and 150% mixed-DPI monitors.
2. Fix release-gate issues and record the evidence in the test record below.
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

### M0 - baseline `[DONE]`

- Kept the Tauri application shell and Vite, React, and TypeScript frontend.
- Replaced the static studio mock with a working editor shell.
- Added local IndexedDB storage and bundled Tesseract.js assets.
- Established the typed browser and native host boundary.

### M1 - web editor `[DONE]`

- Paste, drag-and-drop, and file-picker image intake.
- Annotation tools, properties, zoom and pan, selection, text editing, and
  undo and redo.
- Flattened PNG export, image and text clipboard actions, and local OCR.
- IndexedDB history with thumbnails, annotation autosave, reopen, title editing,
  and case-insensitive title search.
- Browser tests and the Playwright smoke harness cover the editor workflow.

### M2 - native Windows shell `[IMPLEMENTATION COMPLETE]`

Implemented in the repository:

- Per-Monitor V2-aware area capture with a transparent selection overlay, loupe,
  physical-pixel crop validation, and negative virtual-screen origins.
- Window enumeration and `PrintWindow` capture with a GDI fallback.
- Full-screen capture, optional cursor compositing, and native image, text, and
  file clipboard output.
- Disk-backed library under `%LOCALAPPDATA%\CleanShotW`, including annotations,
  thumbnails, title updates, safe index validation, and atomic writes.
- Global capture hotkey, settings persistence, conflict errors, per-user `HKCU`
  startup registration, tray actions, and single-instance activation.
- Always-on-top pin windows with bounded sizing, aspect-ratio preservation, and
  lifecycle cleanup.
- User-facing native error normalization, PNG payload validation, current-user
  NSIS configuration, and automated frontend and Rust coverage.

The implementation is complete. The release gate remains manual. Use the
acceptance checks below for mixed-DPI geometry, GDI and window behavior,
clipboard alpha, packaged OCR assets, tray and autostart, single-instance
behavior, and pin cleanup.

### M3 - polish and scale `[PLANNED]`

Priority order:

1. **Scrolling capture:** Controlled scrolling, stitched output, bounded failure
   and cancel behavior, and coordinate and stitching tests. Keep it opt-in until
   it works across common applications.
2. **Native OCR:** Evaluate bundled native Tesseract for packaged Windows use.
   Retain the browser OCR path where it is the better fallback.
3. **Library search expansion:** Search OCR text and add tags after title search
   is stable.
4. **First public release:** Choose portable ZIP and/or per-user NSIS, publish
   SHA-256 checksums, document SmartScreen behavior, and write release notes.
5. **Optional update checker:** Compare a version file over HTTPS but keep
   downloads and installation user-controlled.

## Decisions

| Decision | Status |
|---|---|
| Tauri remains the Windows shell. The editor stays shell-agnostic through a typed host bridge. | Decided |
| The app is local-first. It has no accounts, cloud sync, sharing URLs, or telemetry. | Decided |
| The Windows native build uses the GNU Rust toolchain. MSVC is not required. | Decided; verified in the Windows build environment |
| Keep the name **CleanShot W** for now. Revisit **ShutterW** before the first public release because of the macOS product name collision. | Decided - defer rename |
| Scrolling capture ships as an M3 beta rather than blocking the native shell release gate. | Proposed |
| No auto-update in v1. | Decided |

## Windows M2 release gate

The native implementation and automated unit coverage are in the repository.
The checks below cover behavior that needs a packaged build on real Windows
hardware.

### Test record

- **Build/version:**
- **Commit:**
- **Tester:**
- **Date:**
- **Windows version/build:**
- **WebView2 version:**
- **Machine/GPU:**
- **Account:** standard user, no administrator elevation
- **Artifact:** NSIS installer or other tested release artifact
- **Result:** PASS / FAIL / BLOCKED

Record the exact failing step and a screenshot or short screen recording for
each failure. Do not attach captured personal or confidential screen contents to
a public issue.

### Installation and startup

- [ ] Per-user NSIS installation completes without a UAC prompt.
- [ ] The installed app starts from the Start menu and from its executable.
- [ ] The app starts with no console window in a release build.
- [ ] A second launch focuses the existing window instead of creating a second
      process or editor window.
- [ ] Closing the main window hides it to the tray.
- [ ] Tray **Open CleanShot W** restores and focuses the editor.
- [ ] Tray **Quit** exits the process and removes the tray icon.
- [ ] Uninstall removes the application without requiring administrator rights.

### Capture modes

Run each mode with the main window visible and with it hidden behind another
window.

- [ ] Area capture starts from the global shortcut.
- [ ] Area capture starts from **New capture** and the tray menu.
- [ ] The overlay freezes the pre-capture screen and does not capture itself.
- [ ] The overlay loupe follows the pointer and reports physical pixel sizes.
- [ ] Dragging in all four directions produces the expected crop.
- [ ] A click or selection smaller than the minimum is safely ignored.
- [ ] `Esc` and **Cancel** close the overlay and restore the editor.
- [ ] Full-screen capture opens the correct full virtual-screen dimensions.
- [ ] Window capture lists visible titled windows and captures the selected one.
- [ ] A window that closes before capture returns an actionable error and does
      not leave the overlay or editor hidden.
- [ ] Capturing an occluded VS Code window and an occluded browser either
      returns the rendered window or clearly reports the Windows limitation. It
      must not silently produce an unrelated or stale image.

### DPI and monitor matrix

Use the same test image or a desktop with visible rulers and text near monitor
edges. Record the physical resolution reported by the editor.

| Layout | Area crop | Full screen | Window crop | Cursor | Result |
|---|---|---|---|---|---|
| One monitor at 100% | [ ] | [ ] | [ ] | on/off | |
| One monitor at 150% | [ ] | [ ] | [ ] | on/off | |
| Two monitors, 100% + 150% | [ ] | [ ] | [ ] | on/off | |
| Secondary monitor left of primary | [ ] | [ ] | [ ] | on/off | |
| Secondary monitor above primary | [ ] | [ ] | [ ] | on/off | |

For every row verify:

- [ ] The selection rectangle aligns with the pointer at monitor edges.
- [ ] There is no one-pixel or scale drift in the loupe or final crop.
- [ ] Negative virtual-screen origins work when a monitor is left or above the
      primary display.
- [ ] Final PNG dimensions match selected physical pixels, not CSS viewport
      dimensions.
- [ ] The crop origin and content are correct at all four virtual-screen edges.

### Cursor, clipboard, and export

- [ ] With **Include cursor** off, the pointer is absent from the native capture.
- [ ] With **Include cursor** on, the pointer appears at the correct position.
- [ ] **Copy image** pastes into Word, Slack, and PowerPoint with transparency
      preserved.
- [ ] **Copy file** pastes as a PNG file into File Explorer and can be opened.
- [ ] **Copy text (OCR)** copies Unicode text without requiring network access.
- [ ] **Save PNG** produces a readable file with a safe filename.
- [ ] Clipboard failures show a useful error and leave the editor usable.

### Library and annotation persistence

- [ ] A native capture appears in History after it is created.
- [ ] Closing and reopening the app restores the image and all annotations.
- [ ] Annotation changes remain after restarting the app.
- [ ] Capture titles, timestamps, thumbnails, and markup counts remain correct.
- [ ] History search filters titles case-insensitively and shows a useful empty
      state when there are no matches.
- [ ] Clicking a title, saving with Enter or blur, and cancelling with Escape
      all behave correctly.
- [ ] Renamed titles persist after restarting the app and are used for PNG
      filenames where applicable.
- [ ] Deleting the current capture closes it in the editor and removes it from
      disk-backed history.
- [ ] Deleting another capture does not change the current editor document.
- [ ] A missing or damaged library file produces an actionable error rather
      than crashing or opening a different capture.

### Settings, hotkey, and startup

- [ ] The default `Ctrl+Shift+4` shortcut registers on first launch.
- [ ] A valid custom shortcut persists after restart.
- [ ] A conflicting shortcut reports the conflict and keeps the previous
      working shortcut.
- [ ] Invalid or empty shortcut input is rejected without changing settings.
- [ ] **Include cursor** persists after restart.
- [ ] **Launch at Windows startup** creates the per-user `HKCU` entry.
- [ ] Startup launches the app minimized to the tray.
- [ ] Turning startup off removes the `HKCU` entry.
- [ ] Settings writes survive a failed shortcut or registry update without
      leaving a partially applied configuration.

### Pin window lifecycle

- [ ] Pin opens an always-on-top window with the correct image and title.
- [ ] Very wide, very tall, and small captures retain their aspect ratio.
- [ ] Multiple pins can coexist without replacing one another.
- [ ] Closing a pin removes its native state and does not affect the editor.
- [ ] Closing the editor cleans up remaining pin windows on app exit.
- [ ] A missing or invalid pin id shows a bounded error state.

### Packaged assets and failure paths

Run these checks against the packaged release artifact, not only the Vite dev
server.

- [ ] OCR loads `worker.min.js`, the Tesseract core, and `eng.traineddata` from
      bundled local assets with the network disabled.
- [ ] A packaged app with no network can capture, annotate, export, and use the
      library.
- [ ] The app reports a clear startup error if WebView2 is unavailable.
- [ ] Protected, UAC, or DRM content fails gracefully with a user-facing
      message. It does not expose a black image as if capture succeeded.
- [ ] The release artifact contains no debug console window or development URL.

### Sign-off

- **Automated frontend tests:** `bun test`
- **Frontend typecheck:** `bun tsc -b --noEmit`
- **Frontend production build:** `bun run build`
- **Rust formatting:** `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- **Rust lint:** `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- **Rust tests:** `cargo test --manifest-path src-tauri/Cargo.toml`
- **Tauri metadata and dependencies:** `cargo check --manifest-path src-tauri/Cargo.toml`

- **Automated checks:** PASS / FAIL
- **Windows matrix:** PASS / FAIL / BLOCKED
- **Known limitations:**
- **Follow-up issue(s):**
- **Release decision:** SHIP / FIX AND RETEST

## Release gates

Before publishing a release:

- Frontend tests, typecheck, production build, and browser smoke checks pass.
- Rust formatting, Clippy, tests, and `cargo check` pass on Windows CI.
- The Windows acceptance checks in this document are complete against the
  release artifact.
- Known limitations and follow-up issues are recorded with the release
  decision.

Do not mark M2 release validation complete based only on browser checks.

## Non-goals for v1

- Video, GIF, or screen recording
- Cloud sync, accounts, public sharing, or hosted libraries
- Installer auto-update or code signing
- Linux or macOS ports
- Full-screen overlay annotation before a capture is committed
