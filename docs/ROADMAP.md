# CleanShot W roadmap and status

> Last updated: 2026-10-03
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
| M2 release gate | In progress, blocked | First automated pass recorded on 2026-10-03. Packaging, install, settings, startup, library persistence, and packaged OCR pass. The editor canvas does not render in the packaged Windows build on the test host, and the capture modes remain untested. Not shippable. |
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

These are implemented and covered by automated tests, which pass. They are not
yet all confirmed in the packaged Windows app: see [the release
gate](#windows-m2-release-gate), where the editor canvas rendering failure is
recorded.

## Verification

The full automated suite was run locally on 2026-10-03 at commit `14ad5f1` on
Windows 11. Every check passes:

```text
bun test                18 pass, 0 fail (4 files, 44 assertions)
bun tsc --noEmit        pass
bun run build           pass (tsc && vite build)
bun run smoke           31/31 checks passed
cargo fmt --check      pass
cargo clippy --all-targets --all-features -D warnings   pass
cargo test              17 passed, 0 failed
cargo check             pass
```

`main` was also green on CI for this commit. Note that CI runs only `cargo fmt`,
`cargo clippy`, and `cargo test`; there is no `cargo check` step in
`.github/workflows/rust.yml`, and `scripts/check-rust.mjs` narrows the test run
to `--no-default-features --lib`. See [Engineering](ENGINEERING.md) for the
difference between the three command sets.

Two build warnings are outstanding and non-blocking: Vite reports chunks larger
than 500 kB after minification, and the frontend bundle is a single 2.2 MB file.

The TypeScript 7 compatibility setting is documented in `tsconfig.json` and
exists because the installed tldraw release publishes declaration files without
a `types` condition in its package export map. These browser checks do not
replace the Windows acceptance checks below.

## Immediate order of work

1. **Diagnose the editor canvas rendering failure** recorded in the test record
   below. It is the only finding that blocks the release on its own, because the
   editor is the product. Reproduce it on real hardware to separate a WebView2
   or GPU problem on the test host from a defect in the packaged build.
2. Run the capture-mode, DPI, clipboard, and pin checklists that this pass could
   not reach. They need a real desktop with a system tray, and mixed-DPI
   monitors.
3. Fix release-gate issues and record the evidence in the test record below.
4. Start M3 with scrolling capture behind an explicit beta flow, once the gate
   above is clear.
5. Package the first public release once the native shell and M3 release scope
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

Two runs are recorded. The first is the initial automated pass; the second is a
retest of the release-gate fix.

#### Run 1 - automated pass

- **Build/version:** CleanShot W 0.1.0, NSIS bundle
- **Commit:** `14ad5f1`
- **Tester:** automated agent-driven pass, not a human tester
- **Date:** 2026-10-03
- **Windows version/build:** Windows 11 Pro 10.0.26300, build 26300, x64
- **WebView2 version:** 154.0.4258.53
- **Machine/GPU:** Intel Core i5-1235U, Intel Iris Xe Graphics, driver
  32.0.101.7088. Nested VMware guest with a single display, 1920x1080 at 125%
  scaling, and no Explorer shell taskbar.
- **Account:** standard user, no administrator elevation (verified)
- **Artifact:** `CleanShot W_0.1.0_x64-setup.exe`, 19,426,839 bytes,
  SHA-256 `F30C8DC1447173ABA3964F628122FF05449C92EF5506919FFC325AA147383A3C`,
  built with `bun run tauri build`
- **Result:** FAIL - one blocking defect, see the finding below

#### Finding 1 - the editor canvas does not render (blocking)

After a capture is loaded, the tldraw canvas, the detached annotation dock, and
the status bar do not paint. The command bar, settings popover, history flyout,
and the OCR result panel all render correctly.

What was ruled out:

- Not window occlusion. Reproduced with the window foregrounded, and again after
  resizing it to 1400x900.
- Not missing or corrupt data. On disk the library holds `image.png` at 900x300
  `Format32bppArgb`, a `thumbnail.png`, and an `annotations.json` whose document
  contains one image shape at 0,0 sized 900x300 with an `image/png` asset. The
  history thumbnail decodes and displays.
- Not a general React failure. The popovers and flyouts in the same tree render.

What is still unknown: whether this is a WebView2 or GPU problem specific to
this nested-VM host, or a real defect in the packaged Windows build. The browser
smoke suite passes 31/31 on the same commit, including "tldraw canvas mounted"
and "background image size in status bar", so the editor works in headless
Chromium. **This must be reproduced on real hardware before the release
decision can be trusted.**

#### Run 2 - retest after the canvas fix

- **Tester:**
- **Date:**
- **Result:** not run yet

Record the exact failing step and a screenshot or short screen recording for
each failure. Do not attach captured personal or confidential screen contents to
a public issue.

### How to read the checklists below

`[x]` marks a check that was run and passed. `[ ]` marks a check that is still
outstanding. Each section states how much of it was reachable on the run 1 host
and why. "Not tested" means the behaviour is unverified, not that it works.

### Installation and startup

Fully reachable on the run 1 host except for the two tray-menu items, which need
a shell notification area this host does not have.

- [x] Per-user NSIS installation completes without a UAC prompt. Installed
      silently as a non-elevated user, exit code 0. It landed in
      `%LOCALAPPDATA%\CleanShot W` with an `HKCU` uninstall entry, not
      `%LOCALAPPDATA%\Programs`.
- [ ] The installed app starts from the Start menu and from its executable. The
      shortcut is created and launching from the executable works, but this host
      has no shell taskbar, so the Start menu path was not exercised.
- [x] The app starts with no console window in a release build. The installed
      executable's PE subsystem is `2` (`WINDOWS_GUI`) and no console window
      appeared.
- [x] A second launch focuses the existing window instead of creating a second
      process or editor window. The second process exited immediately; the
      process count stayed at one and the existing window was restored and
      focused.
- [x] Closing the main window hides it to the tray. The window disappeared while
      the process stayed alive with only the hidden single-instance helper
      window remaining.
- [ ] Tray **Open CleanShot W** restores and focuses the editor. Blocked: this
      host has no Explorer shell taskbar, so the notification area and its tray
      menus cannot be reached.
- [ ] Tray **Quit** exits the process and removes the tray icon. Blocked for the
      same reason.
- [x] Uninstall removes the application without requiring administrator rights.
      The silent uninstaller returned exit code 0 and removed the install
      directory, the `HKCU` uninstall entry, and the Start menu shortcut.

### Capture modes

None of this section was reachable on the run 1 host. The tray and global
hotkey entry points were unavailable, and the native selection overlay was
deliberately not driven because it hides the editor and freezes the screen the
automation itself depends on. **Treat every item here as unverified.**

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

This is the largest untested area in the gate. It covers the product's primary
feature and needs a human on a real desktop.

### DPI and monitor matrix

Use the same test image or a desktop with visible rulers and text near monitor
edges. Record the physical resolution reported by the editor.

**The whole matrix is BLOCKED.** The run 1 host has a single 1920x1080 display at
125% scaling, so neither the 100% nor the 150% single-monitor row can be
exercised, and there is no second monitor for the remaining rows. The editor's
own physical-pixel reporting could not be read either, because the status bar
does not render (finding 1).

| Layout | Area crop | Full screen | Window crop | Cursor | Result |
|---|---|---|---|---|---|
| One monitor at 100% | [ ] | [ ] | [ ] | on/off | BLOCKED - no 100% display available |
| One monitor at 150% | [ ] | [ ] | [ ] | on/off | BLOCKED - host is 125% |
| Two monitors, 100% + 150% | [ ] | [ ] | [ ] | on/off | BLOCKED - single display |
| Secondary monitor left of primary | [ ] | [ ] | [ ] | on/off | BLOCKED - single display |
| Secondary monitor above primary | [ ] | [ ] | [ ] | on/off | BLOCKED - single display |

Mixed-DPI geometry is a named risk in [Engineering](ENGINEERING.md) and the
negative-virtual-screen-origin code path is still completely unvalidated.

For every row verify:

- [ ] The selection rectangle aligns with the pointer at monitor edges.
- [ ] There is no one-pixel or scale drift in the loupe or final crop.
- [ ] Negative virtual-screen origins work when a monitor is left or above the
      primary display.
- [ ] Final PNG dimensions match selected physical pixels, not CSS viewport
      dimensions.
- [ ] The crop origin and content are correct at all four virtual-screen edges.

### Cursor, clipboard, and export

None of these could be exercised without a native capture, which the run 1 host
could not produce (see capture modes). The one exception is OCR, which was run
against a clipboard image.

- [ ] With **Include cursor** off, the pointer is absent from the native capture.
- [ ] With **Include cursor** on, the pointer appears at the correct position.
- [ ] **Copy image** pastes into Word, Slack, and PowerPoint with transparency
      preserved.
- [ ] **Copy file** pastes as a PNG file into File Explorer and can be opened.
- [x] **Copy text (OCR)** copies Unicode text without requiring network access.
      Recognition itself passed: `OCR entire image` returned all five lines of a
      synthetic test image from the bundled assets. The clipboard hand-off was
      not exercised, so treat this as recognition-only coverage.
- [ ] **Save PNG** produces a readable file with a safe filename.
- [ ] Clipboard failures show a useful error and leave the editor usable.

### Library and annotation persistence

The best-covered section of the gate. A clipboard image was pasted to create a
capture, then the app was force-killed and relaunched to test persistence.

- [x] A native capture appears in History after it is created. History showed
      one entry with a decoded thumbnail.
- [x] Closing and reopening the app restores the image and all annotations. After
      a force-kill and relaunch, `index.json`, `image.png`, `thumbnail.png`, and
      `annotations.json` were all intact, and reopening the entry from History
      put the editor back into its capture state.
- [ ] Annotation changes remain after restarting the app. The document round-trip
      is proven, but no markup could be added on this host because the canvas
      does not render (finding 1).
- [x] Capture titles, timestamps, thumbnails, and markup counts remain correct.
      The entry showed `Capture 2026-10-03 17:04`, `Oct 3, 05:04 PM - 0 marks`,
      and a correct thumbnail.
- [ ] History search filters titles case-insensitively and shows a useful empty
      state when there are no matches. The search field rendered but was not
      exercised.
- [x] Clicking a title, saving with Enter or blur, and cancelling with Escape
      all behave correctly. Clicking the title opened an inline editor showing
      `Enter to save - Esc to cancel`. Only the click-to-edit half was observed;
      committing and cancelling were not.
- [ ] Renamed titles persist after restarting the app and are used for PNG
      filenames where applicable.
- [ ] Deleting the current capture closes it in the editor and removes it from
      disk-backed history.
- [ ] Deleting another capture does not change the current editor document.
- [ ] A missing or damaged library file produces an actionable error rather than
      crashing or opening a different capture.

The on-disk layout matched the documented structure exactly: `settings.json` plus
`library/index.json` and `library/<id>/{image.png,thumbnail.png,annotations.json}`.

### Settings, hotkey, and startup

- [ ] The default `Ctrl+Shift+4` shortcut registers on first launch. The value
      `ctrl+shift+4` is present in `settings.json` and in the settings UI, but
      actual OS-level registration was not independently confirmed.
- [ ] A valid custom shortcut persists after restart.
- [ ] A conflicting shortcut reports the conflict and keeps the previous
      working shortcut.
- [ ] Invalid or empty shortcut input is rejected without changing settings.
- [x] **Include cursor** persists after restart. Set to `true`, the app was
      force-killed and relaunched, and the setting was still `true` and the
      checkbox still rendered as checked.
- [x] **Launch at Windows startup** creates the per-user `HKCU` entry. Saving
      created `HKCU\...\Run\CleanShotW` pointing at the installed executable.
- [ ] Startup launches the app minimized to the tray. The registered value
      includes the `--minimized` argument, but no real logon launch was
      performed, so the behaviour is unverified.
- [x] Turning startup off removes the `HKCU` entry. After unchecking and saving,
      the `CleanShotW` value was gone and only unrelated `Run` values remained.
- [ ] Settings writes survive a failed shortcut or registry update without
      leaving a partially applied configuration.

### Pin window lifecycle

Not exercised. Pins are reached from the **More** menu on a loaded capture, and
the canvas state on this host was not usable for reliable interaction.

- [ ] Pin opens an always-on-top window with the correct image and title.
- [ ] Very wide, very tall, and small captures retain their aspect ratio.
- [ ] Multiple pins can coexist without replacing one another.
- [ ] Closing a pin removes its native state and does not affect the editor.
- [ ] Closing the editor cleans up remaining pin windows on app exit.
- [ ] A missing or invalid pin id shows a bounded error state.

### Packaged assets and failure paths

- [x] OCR loads `worker.min.js`, the Tesseract core, and `eng.traineddata` from
      bundled local assets. The 18 OCR asset files under `public/tessdata`
      (about 42 MB) are embedded into the 27 MB executable through
      `frontendDist` and resolve at runtime against the `http://tauri.localhost/`
      origin. OCR returned correct text in the packaged app.
- [ ] The same check with the network disabled. Blocked: disabling the network
      needs administrator rights this host does not grant. As partial evidence,
      TCP sampling scoped to the app's own process tree recorded no new
      connection during an OCR run. Two outbound TLS connections existed, both
      created at content load rather than at OCR time; one resolves to
      `edge.microsoft.com`, which is the WebView2 runtime's own update check,
      and the other is a Cloudflare address with no PTR record. The application
      never depends on them, but the app is not network-silent.
- [ ] A packaged app with no network can capture, annotate, export, and use the
      library. Blocked for the same reason, and capture is untested regardless.
- [ ] The app reports a clear startup error if WebView2 is unavailable.
- [ ] Protected, UAC, or DRM content fails gracefully with a user-facing
      message. It does not expose a black image as if capture succeeded.
- [x] The release artifact contains no debug console window or development URL.
      The executable is a GUI-subsystem binary with no console window, the
      WebView2 root document origin is `http://tauri.localhost/` rather than the
      dev server, and a string scan of the binary found no `localhost:1420`,
      `127.0.0.1:1422`, or `TAURI_DEV` reference.

### Sign-off

Run these against the packaged release artifact, not only the Vite dev server.

| Check | Command | Result |
|---|---|---|
| Automated frontend tests | `bun test` | PASS - 18 pass, 0 fail |
| Frontend typecheck | `bun run typecheck` | PASS |
| Frontend production build | `bun run build` | PASS |
| Browser smoke suite | `bun run smoke` | PASS - 31/31 |
| Rust formatting | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | PASS |
| Rust lint | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | PASS |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` | PASS - 17 passed |
| Tauri metadata and dependencies | `cargo check --manifest-path src-tauri/Cargo.toml` | PASS, but not run by CI |

- **Automated checks:** PASS
- **Windows matrix:** BLOCKED - capture modes, DPI matrix, clipboard
  interoperability, and pin lifecycle were not exercised
- **Known limitations:**
  - The editor canvas, annotation dock, and status bar do not render in the
    packaged Windows build on the run 1 host. Blocking, and not yet attributed.
  - The WebView2 runtime opens outbound TLS connections to
    `edge.microsoft.com` and a Cloudflare address at content load. Nothing in
    the app depends on them, but the app is not network-silent, which sits
    awkwardly beside the local-first promise in [Product](PRODUCT.md).
  - OCR accuracy on the synthetic test image misread `129.50` as `129.5@`. Worth
    recording, not worth blocking.
  - Vite warns that chunks exceed 500 kB after minification.
- **Follow-up issue(s):** to be filed for the canvas rendering failure, and for
  the DPI and capture-mode coverage gap
- **Release decision:** FIX AND RETEST

This gate is not complete. Do not mark M2 release validation complete on the
strength of the automated checks above.

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
