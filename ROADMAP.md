# CleanShot W — Roadmap

Locked-in plan for building a CleanShot-style screenshot tool on Windows,
without admin rights or paid tooling.

**Status:** M2 implementation complete; Windows hardware release validation is the remaining gate before shipping M3 polish.

## 1. Goal

A native-feeling Windows screenshot tool that lets you:

- Capture an area, a window, or a full screen
- Annotate the result (rect, ellipse, arrow, line, text, numbered steps,
  highlight, blur, pixelate, redact)
- Select and copy text out of screenshots (OCR)
- Export/copy/save the result and find it again later for re-editing

Everything local-first. No account, no telemetry, no cloud.

## 2. Guiding constraints

| Constraint | Consequence |
|---|---|
| **Tauri stays the app shell** `[DECIDED]` | Rust + WebView2 host; the web editor is shell-agnostic so it develops in a plain browser |
| **Tauri builds with the GNU toolchain — no MSVC needed** `[DECIDED]` | Verified 2026-08-14: `x86_64-pc-windows-gnu` + MinGW GCC built `cleanshot-w.exe` locally (~15 min first build). Local `tauri dev/build` works on this machine |
| No admin rights on the dev machine | User-local tool installs only; see §7 |
| No paid tooling | Free: Vite/React/TS, Rust, tesseract.js; unsigned ZIP/per-user installer distribution with SHA-256 |
| End users also may lack admin | Portable ZIP or per-user NSIS; WebView2 Evergreen (ships with Win10/11) |

## 3. Architecture

Two-stage, one codebase. Stage 1 is 100% browser-runnable. Stage 2 wires the
same editor into the Tauri shell. The editor talks to the shell through a
typed `hostBridge` that is mocked in the browser and backed by real Tauri
commands in the shell.

```
┌─────────────────────────────────────────────────────────────┐
│  Web editor (React + Vite + TS)                             │
│  capture intake → annotation canvas → export / library      │
│  OCR via tesseract.js (bundled wasm + traineddata)          │
└──────────────▲───────────────────────────────┬──────────────┘
               │ invoke()/commands (bridge)    │ capture + clipboard + storage
┌──────────────┴───────────────┐  ┌────────────▼──────────────┐
│ Browser mock (M1)            │  │ Tauri shell (Rust, M2+)   │
│ paste / drag-drop / file     │  │ hotkeys, native capture,  │
│ picker; IndexedDB storage    │  │ transparent overlay, tray,│
│                              │  │ clipboard, disk library   │
└──────────────────────────────┘  └───────────────────────────┘
```

### 3.1 The `hostBridge` contract

The editor imports a bridge that resolves to either the browser mock (dev /
`npm run dev`) or real Tauri commands (`invoke` from `@tauri-apps/api/core`).
The web code never branches on the shell — the mock and the Rust commands
implement the same interface.

Commands (M1 = mock, M2 = real Rust):

| cmd | M1 | M2 | Notes |
|---|---|---|---|
| `capture:area` | mock (paste/drop) | Rust (GDI/DXGI) | Returns full-res PNG + natural size |
| `capture:window` | mock | Rust (`EnumWindows` + `PrintWindow`) | |
| `capture:fullscreen` | mock | Rust (per-monitor aware) | |
| `clipboard:copyImage` | browser clipboard | Rust `arboard` | PNG as DIB v5 (alpha-safe) |
| `clipboard:copyFile` | no-op | Rust (CF_HDROP temp PNG) | |
| `library:save/list/open` | IndexedDB | Rust, disk under `%LOCALAPPDATA%` | |
| `ocr:run` | tesseract.js | tesseract.js first, native later | Block list with bbox |
| `pin:show` | mock | Rust window | Always-on-top |
| `settings:get/set` | localStorage | Rust, JSON in `%LOCALAPPDATA%` | |
| event `hotkeyPressed` | none | `tauri-plugin-global-shortcut` | |

Envelope for custom commands (unless the existing Tauri command API fits
better for a given call):

```jsonc
{ "id": "req-123", "cmd": "capture:area", "params": {} }
{ "id": "req-123", "ok": true, "data": { "pngBase64": "...", "width": 1920, "height": 1080, "dpiScale": 1.0 } }
```

### 3.2 Data model

All geometry lives in **image natural-pixel space** so exports stay crisp.
The view applies a single transform: `screenPoint = imagePoint * scale + pan`.

```ts
type CaptureDoc = {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  image: { width: number; height: number };      // natural resolution
  annotations: Annotation[];
};

type Annotation =
  | { kind: "rect" | "ellipse" | "line"; x: number; y: number; width?: number; height?: number; stroke: { color: string; width: number } }
  | { kind: "arrow"; points: { x: number; y: number }[]; stroke: { color: string; width: number } }
  | { kind: "text"; x: number; y: number; text: string; fontSize: number; fontFamily: string; color: string }
  | { kind: "counter"; x: number; y: number; n: number; radius: number; color: string }
  | { kind: "highlight" | "redact"; x: number; y: number; width: number; height: number; color: string; opacity?: number }
  | { kind: "blur" | "pixelate"; x: number; y: number; width: number; height: number; strength: number };
```

### 3.3 Annotation rendering

- Vector shapes (rect/ellipse/arrow/line) and text render as **SVG** over an
  `<img>` — cheap hit-testing, native text editing, handles.
- Effects (highlight/blur/pixelate/redact) render as absolutely-positioned
  divs clipped to their region, **not** painted into a canvas:
  - blur: `background-image` = same image with matching `background-size`, `filter: blur(Npx)`
  - pixelate: downscaled copy with `image-rendering: pixelated`
  - highlight: translucent `background-color`
  - redact: opaque `background-color`
  This keeps effects editable (move/resize/restrength later) and re-renders
  cleanly on zoom. Export flattens by compositing to an offscreen canvas at
  natural resolution.

### 3.4 Undo / history

- Command stack (snapshot of `annotations[]`), unlimited-depth, `Ctrl+Z` /
  `Ctrl+Shift+Z`.
- Capture auto-saved to the library (debounced). Re-opening a capture restores
  image + annotations + undo history.

## 4. Milestones

### M0 — Baseline `[DONE]`

- Kept the Tauri scaffold (deps, `src-tauri/`, Tauri-flavored Vite config).
- Upgraded TypeScript to 7.x (`~7.0.2`, native compiler — build verified).
- Added `tesseract.js` with locally bundled wasm + `eng` traineddata
  (`scripts/ocr-assets.mjs`, `bun run ocr:assets`).
- Hand-rolled IndexedDB storage layer (no ORM/state library).
- Replaced the marketing-style studio screen with a working editor shell.

### M1 — Web editor prototype (browser-only) `[DONE]`

**In scope**
- Intake: paste, drag-and-drop, file picker (no native capture yet).
- Tools: select/move (V), rect (R), ellipse (O), arrow (A), line (L),
  text (T), counter (C), highlight (H), blur (B), pixelate (X), redact (K).
- Properties: stroke color, width, fill; arrowhead size; font size/color;
  effect strength. Tool defaults persist in localStorage and malformed stored
  values fall back to safe defaults.
- Interaction: click-drag to draw, Shift constrains aspect, Esc cancels,
  drag to move, resize handles, double-click text to edit, Delete removes.
- Zoom/pan (`+`/`-`/`0`, wheel with Ctrl, middle/Alt-drag pan).
- Undo/redo (Ctrl+Z / Ctrl+Shift+Z).
- OCR: "Copy text (OCR)" in the Quick Access bar via tesseract.js with
  bundled local assets. Results include word bounding boxes for the next OCR
  layer without requiring a native install.
- Export: PNG save, copy image to clipboard (flattened at natural res),
  copy text.
- Library: IndexedDB history with thumbnails, open → re-edit, autosave.
- Quick Access: floating action bar over the editor
  (Copy image / OCR / Save PNG / New / Close).

**Out of scope for M1**
- Native capture, global hotkeys, tray, screen overlays, disk library,
  scrolling capture, video, pinning to screen.

**Acceptance criteria**
- Capture a screenshot elsewhere → paste → annotate with every tool → copy
  image → paste into Word/Slack with alpha intact.
- OCR a code screenshot and copy the text out correctly.
- Close and reopen a capture → re-editable with history intact.
- Undo/redo works through at least 20 operations with no drift.

**Verification (2026-08-16)**

- `bun test`: 12 unit tests pass.
- `bun run build`: TypeScript and Vite production build pass.
- `node scripts/smoke-tldraw.mjs`: 32 browser checks pass, including intake,
  annotations, styles, clipboard copy, PNG export, OCR, undo/redo, autosave,
  reload persistence, and history re-editing.

### M2 — Tauri native shell (Rust) `[IMPLEMENTATION COMPLETE]`

**Implementation status (2026-08-18)**

M2 has no remaining deferred implementation work. The Windows-only runtime
matrix is documented in §9 and must be run on representative hardware before a
public release; the current workspace is not a Windows host.

1. **Implemented and covered:** native selection overlay with a pixel loupe,
   physical-pixel crop validation, and CSS-to-capture coordinate tests.
2. **Implemented:** Per-Monitor V2 process awareness and physical virtual-screen
   coordinates for mixed-DPI monitor layouts.
3. **Implemented:** window enumeration and `PrintWindow` capture with a GDI
   screen-capture fallback.
4. **Implemented:** native image, text, and file clipboard output via `arboard`.
5. **Implemented:** disk-backed capture library under `%LOCALAPPDATA%\CleanShotW`.
6. **Implemented:** persisted global capture shortcut with conflict errors and
   settings controls for remapping, cursor inclusion, and startup behavior.
7. **Implemented:** system tray with new capture, open library, and quit actions;
   tray capture requests are routed back to the main editor.
8. **Implemented:** single-instance behavior brings the existing editor window
   to the foreground.
9. **Implemented:** optional per-user `HKCU\\...\\Run` autostart launches the
   app minimized to the tray.
10. **Implemented:** optional cursor compositing for screen and window captures.
11. **Implemented:** always-on-top pin windows with a dedicated lightweight
    renderer, close action, and lifecycle cleanup.

- **Selection overlay is a transparent Tauri window**, not a separate tech:
  `transparent: true`, `decorations: false`, `alwaysOnTop`, one window per
  monitor (or spanning the virtual screen), HTML/CSS draws the dim, the
  selection rect, and a pixel-magnifier loupe. On release → open the editor
  window with the cropped region.
  - Caveat: transparent WebView2 windows on Windows have real limitations
    (input routing, per-monitor DPI, layered-window composition). The selected
    implementation is tested against the release matrix in §6.
    **Fallback** if a Windows compositor issue is found: a thin Win32/GDI
    overlay written in Rust (`windows` crate — still Tauri, no C++), or a
    WPF overlay as a last resort.
- **Editor is a normal (non-transparent) Tauri window.**
- Windows: main library window, capture overlay, Quick Access bar, pin
  windows (always-on-top thumbnails).
- Hotkeys: `tauri-plugin-global-shortcut` (user-level, no admin). Conflict →
  detected, error surfaced, remap in settings (stored in HKCU/local JSON).
- Capture in Rust: GDI `CopyFromScreen` / `xcap` crate; window capture via
  `EnumWindows` + `PrintWindow(WP_RENDERFULLCONTENT)` fallback.
- Clipboard: `arboard` for copy-image (DIB v5, alpha-safe); copy-file via
  CF_HDROP of a temp PNG.
- Library on disk:
  ```
  %LOCALAPPDATA%\CleanShotW\
    library\<uuid>\image.png
                   annotations.json
                   thumbnail.png
    settings.json
    WebView2\                (user data folder)
    logs\
  ```
  JSON index file for the library (fast startup; SQLite only if it grows).
- Tray icon (tauri tray-icon): new capture, open library, quit.
- Single instance (mutex/`tauri-plugin-single-instance`).
- `HKCU\...\Run` autostart (no admin), toggleable from settings.
- Cursor inclusion in capture, toggleable from settings.

**Build:** verified with the GNU toolchain (Rust `x86_64-pc-windows-gnu` +
MinGW GCC) — no MSVC required. First release build took ~15 min.

**Automated validation added:**

- `bun tsc -b --noEmit` validates the frontend without emitting build artifacts.
- `bun test` covers 15 browser and native-coordinate unit tests.
- Rust unit tests cover selection bounds, crop pixel/origin preservation, PNG
  encoding, settings migration/validation, autostart quoting, and pin sizing;
  the Windows CI job runs formatting, Clippy, tests, and `cargo check`.

**Out of scope for M2:** scrolling capture, in-place annotation on the overlay,
and native OCR. These remain M3 work.

### M3 — Polish and scale

- Scroll capture: SendInput-based scrolling + stitching (hard; ship as beta).
- In-process native OCR (bundled Tesseract 5 + tessdata, no system install).
- Library search by OCR text; tag/title editing.
- First public release: portable ZIP (or per-user NSIS) + SHA-256 checksums,
  release notes.
- Optional: update checker (compare version file over HTTPS, user-downloaded ZIP).

## 5. Windows engineering notes (the parts the plan didn't spell out)

### 5.1 DPI — the #1 correctness trap
- The app must run **Per-Monitor V2** DPI aware (set in the manifest/Tauri).
- Win32 metrics (`GetSystemMetrics(SM_XVIRTUALSCREEN…)`, `GetMonitorInfo`) are
  in physical pixels; WPF/`Screen` values are device-independent. For Tauri,
  the WebView2 window reports its own DPI per monitor — overlay math must be in
  physical pixels, converted once at the bridge boundary.
- `CopyFromScreen` needs physical-pixel coordinates; the returned bitmap is at
  native resolution — don't downscale the pixel data, only the display.

### 5.2 Capture pipeline
1. Hotkey → bring up overlay window(s) covering the **virtual screen** (union
   of all monitors).
2. Overlay freezes the screen content by drawing the pre-captured full-screen
   bitmap as its background (capture first, then show the frozen frame —
   avoids showing your own overlay in the shot).
3. User drags a rect → crop at native resolution from the frozen bitmap.
4. Send crop to the editor as base64 PNG over the bridge.

### 5.3 Tauri asset loading
- Production loads the Vite `dist/` via Tauri's asset protocol; dev uses the
  Vite dev server. Tesseract wasm/worker + traineddata must resolve under the
  asset protocol — verify early (spike #3).
- Tauri user-data folder lands under `%LOCALAPPDATA%` automatically.

### 5.4 Transparent overlay window
Tauri transparent windows on Windows use layered-window composition. The M2
implementation keeps the editor opaque and confines transparency to the native
capture overlay. The overlay maps its actual CSS viewport bounds back into the
pre-captured physical-pixel frame, avoiding a second DPI conversion at crop
time; the mixed-DPI hardware matrix in §9 remains the release acceptance test.

### 5.5 Clipboard gotchas
- "Copy image": `arboard` `set_image` writes a DIB with premultiplied alpha —
  correct for Slack/Word/PowerPoint. Verify a paste keeps transparency
  (release acceptance test).
- "Copy file": write PNG to `%TEMP%`, then CF_HDROP with `SHAddToRecentDocs`.

### 5.6 OCR (tesseract.js) integration
- Create worker with explicit `workerPath`, `langPath`, `corePath` pointing at
  bundled `public/tessdata` + `tessdata_fast` by default.
- Run on-demand (when the text tool activates), not on every capture — it's
  1–5s and must not block the overlay.
- Add a small benchmark harness (fixture screenshots → per-image accuracy +
  latency for `fast` vs `best`) so the default is data-driven.
- On-screen result: transparent text layer with word boxes so users can click
  a word to select/copy it.

### 5.7 Protected/secure content
DWM cannot capture DRM-protected or the UAC secure desktop. Handle gracefully
("This content can't be captured") rather than shipping a black image.

## 6. Risks and release validation

| Risk | Spike | Pass criteria |
|---|---|---|
| Transparent Tauri overlay across mixed-DPI monitors | 2-monitor (100% + 150%) test: rect must match cursor under loupe | no drift, input routed correctly, no flicker |
| GDI capture on mixed DPI | Capture both monitors and crop near each monitor edge | per-monitor physical pixels correct |
| Copy image with alpha into Word/Slack | Paste test after `arboard::set_image` | transparent background survives |
| Tesseract worker + wasm under Tauri asset protocol | Run OCR in a packaged build | no CORS/path errors |
| `PrintWindow` window capture on occluded/windowed apps | Capture VS Code and a browser | correct pixels, fallback has no black window |
| Cursor compositing | Capture with the setting on and off | pointer appears only when enabled |
| Tray/autostart/pin lifecycle | Start with `--minimized`, use tray, pin/close a capture | no duplicate process, orphaned window, or stuck tray state |
| ~~Tauri build without MSVC~~ `[DONE]` | GNU toolchain attempt | `cleanshot-w.exe` built locally 2026-08-14 (15 min) |

## 7. Toolchain (no-admin setup) — locked

```
Node 24 + bun          already present
Rust + GNU toolchain   present — builds Tauri on Windows without MSVC (verified)
TypeScript 7.x         upgraded (native compiler)
WebView2               Evergreen runtime on Win10/11 (startup check + clear error if missing)
Tesseract              none system-wide — bundled wasm (M1) / native later (M3)
```

- Editor dev loop: `bun install && bun run dev` (plain Vite in a browser).
  No native dependencies.
- Tauri shell build: works with the default GNU toolchain — `bun run tauri
  build`. Verified locally.

## 8. Distribution & release process

Tauri builds the frontend into the binary and ships `tessdata/` via the
`resources` config. Two packaging options, both no-admin for the end user:

```
Option A — portable ZIP                     Option B — per-user NSIS
CleanShotW-0.1.0-win64.zip                  CleanShotW-0.1.0-win64-setup.exe
  CleanShotW.exe                              (tauri nsis installMode: currentUser)
  resources/  (tessdata, etc.)
  CleanShotW-0.1.0-win64.zip.sha256
```

- Every release: artifact + SHA-256 sidecar + notes in `CHANGELOG.md`.
- SmartScreen will flag unsigned binaries — document "More info → Run anyway".
- No auto-update in v1.

## 9. Testing

- UI: Playwright against the Vite dev server for editor interactions
  (draw, select, undo/redo, zoom, export round-trip).
- OCR: benchmark harness from §5.6 (fixtures committed to `tests/fixtures/`).
- Native: the Windows CI job runs Rust formatting, Clippy, unit tests, and
  `cargo check`; browser-side native coordinate mapping is covered by `bun test`.
- M2 Windows release gate: on Windows 10/11, test area, window, and fullscreen
  capture; a 100% + 150% two-monitor layout; cursor on/off; transparent paste
  into Word/Slack; `PrintWindow` fallback; packaged OCR assets; tray quit/new
  capture; `--minimized` autostart; and pin/close lifecycle. Record the result
  with the release notes before publishing an artifact.
- Clipboard/WebView2 behaviors are manual acceptance checks, not browser-only
  unit-test substitutes.

## 10. Decisions

| # | Decision | Status |
|---|---|---|
| 1 | Keep Tauri as the app shell; editor stays shell-agnostic | `[DECIDED]` |
| 2 | Keep the name **CleanShot W** for now; **rename to ShutterW later** — note for follow-up before first public release (name collides with the macOS CleanShot product) | `[DECIDED — defer rename]` |
| 3 | Tauri builds with the GNU toolchain — MSVC not required | `[DECIDED]` |
| 4 | Scrolling capture: M3 beta | `[PROPOSED]` |
| 5 | .NET/Tauri channel: N/A — shell is Rust/Tauri | `[DECIDED]` |

## 11. Non-goals (v1)

- Video / GIF / screen recording
- Cloud sync, accounts, sharing URLs
- Installer auto-update, code signing
- Linux/macOS ports
- Editing capture in place on the full-screen overlay (stretch)
