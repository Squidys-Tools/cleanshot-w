# CleanShot W — Windows M2 release checklist

This checklist is the remaining acceptance gate for the M2 native shell. The
native implementation and automated unit coverage are in the repository; this
document records the Windows-only behaviors that cannot be verified in a Linux
or browser-only environment.

For product status, see the [roadmap](../ROADMAP.md). For architecture and
commands, see [engineering notes](ENGINEERING.md). This file is only the manual
release evidence record.

## Test record

- **Build/version:**
- **Commit:**
- **Tester:**
- **Date:**
- **Windows version/build:**
- **WebView2 version:**
- **Machine/GPU:**
- **Account:** standard user, no administrator elevation
- **Artifact:** NSIS installer or other tested release artifact (link or filename)
- **Result:** PASS / FAIL / BLOCKED

Record the exact failing step and a screenshot or short screen recording for any
failure. Do not attach captured personal or confidential screen contents to a
public issue.

## 1. Installation and startup

- [ ] Per-user NSIS installation completes without a UAC prompt.
- [ ] The installed app starts from the Start menu and from its executable.
- [ ] The app starts with no console window in a release build.
- [ ] A second launch focuses the existing window instead of creating a second
      process or editor window.
- [ ] Closing the main window hides it to the tray.
- [ ] Tray **Open CleanShot W** restores and focuses the editor.
- [ ] Tray **Quit** exits the process and removes the tray icon.
- [ ] Uninstall removes the application without requiring administrator rights.

## 2. Capture modes

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
      returns the rendered window or clearly reports the Windows limitation;
      it must not silently produce an unrelated or stale image.

## 3. DPI and monitor matrix

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

- [ ] The selection rectangle aligns with the pointer at the monitor edges.
- [ ] There is no one-pixel or scale drift in the loupe or final crop.
- [ ] Negative virtual-screen origins work when a monitor is left or above the
      primary display.
- [ ] The final PNG dimensions match the selected physical pixels, not the CSS
      viewport dimensions.
- [ ] The crop origin and content are correct at all four virtual-screen edges.

## 4. Cursor, clipboard, and export

- [ ] With **Include cursor** off, the pointer is absent from the native capture.
- [ ] With **Include cursor** on, the pointer appears at the correct position.
- [ ] **Copy image** pastes into Word, Slack, and PowerPoint with transparency
      preserved.
- [ ] **Copy file** pastes as a PNG file into File Explorer and can be opened.
- [ ] **Copy text (OCR)** copies Unicode text without requiring network access.
- [ ] **Save PNG** produces a readable file with a safe filename.
- [ ] Clipboard failures show a useful error and leave the editor usable.

## 5. Library and annotation persistence

- [ ] A native capture appears in History after it is created.
- [ ] Closing and reopening the app restores the image and all annotations.
- [ ] Annotation changes remain after restarting the app.
- [ ] Capture titles, timestamps, thumbnails, and markup counts remain correct.
- [ ] History search filters titles case-insensitively and shows a useful empty
      state when there are no matches.
- [ ] Clicking a title, saving with Enter/blur, and cancelling with Escape all
      behave correctly.
- [ ] Renamed titles persist after restarting the app and are used for PNG
      filenames where applicable.
- [ ] Deleting the current capture closes it in the editor and removes it from
      disk-backed history.
- [ ] Deleting another capture does not change the current editor document.
- [ ] A missing or damaged library file produces an actionable error rather
      than crashing or opening a different capture.

## 6. Settings, hotkey, and startup

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

## 7. Pin window lifecycle

- [ ] Pin opens an always-on-top window with the correct image and title.
- [ ] Very wide, very tall, and small captures retain their aspect ratio.
- [ ] Multiple pins can coexist without replacing one another.
- [ ] Closing a pin removes its native state and does not affect the editor.
- [ ] Closing the editor cleans up any remaining pin windows on app exit.
- [ ] A missing or invalid pin id shows a bounded error state.

## 8. Packaged assets and failure paths

Run these checks against the packaged release artifact, not only the Vite dev
server.

- [ ] OCR loads `worker.min.js`, the Tesseract core, and `eng.traineddata` from
      bundled local assets with the network disabled.
- [ ] A packaged app with no network can capture, annotate, export, and use the
      library.
- [ ] The app reports a clear startup error if WebView2 is unavailable.
- [ ] Protected/UAC/DRM content fails gracefully with a user-facing message;
      it does not expose a black image as if capture succeeded.
- [ ] The release artifact contains no debug console window or development URL.

## Sign-off

- **Automated frontend tests:** `bun test`
- **Frontend typecheck:** `bun tsc -b --noEmit`
- **Frontend production build:** `bun run build`
- **Rust formatting:** `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- **Rust lint:** `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
- **Rust tests:** `cargo test --manifest-path src-tauri/Cargo.toml`
- **Tauri metadata/dependencies:** `cargo check --manifest-path src-tauri/Cargo.toml`

- **Automated checks:** PASS / FAIL
- **Windows matrix:** PASS / FAIL / BLOCKED
- **Known limitations:**
- **Follow-up issue(s):**
- **Release decision:** SHIP / FIX AND RETEST
