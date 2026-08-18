# Architecture

Two-stage, one codebase. Stage 1 is 100% browser-runnable. Stage 2 wires the
same editor into the Tauri shell.

## Overview

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

The editor talks to the shell through a typed `hostBridge` that is mocked in the
browser and backed by real Tauri commands in the shell.

## The hostBridge contract

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

Envelope for custom commands:

```jsonc
{ "id": "req-123", "cmd": "capture:area", "params": {} }
{ "id": "req-123", "ok": true, "data": { "pngBase64": "...", "width": 1920, "height": 1080, "dpiScale": 1.0 } }
```

## Data model

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

## Annotation rendering

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

## Undo / history

- Command stack (snapshot of `annotations[]`), unlimited-depth, `Ctrl+Z` /
  `Ctrl+Shift+Z`.
- Capture auto-saved to the library (debounced). Re-opening a capture restores
  image + annotations + undo history.

## Windows engineering notes

### DPI — the #1 correctness trap

- The app must run **Per-Monitor V2** DPI aware (set in the manifest/Tauri).
- Win32 metrics (`GetSystemMetrics(SM_XVIRTUALSCREEN…)`, `GetMonitorInfo`) are
  in physical pixels; WPF/`Screen` values are device-independent. For Tauri,
  the WebView2 window reports its own DPI per monitor — overlay math must be in
  physical pixels, converted once at the bridge boundary.
- `CopyFromScreen` needs physical-pixel coordinates; the returned bitmap is at
  native resolution — don't downscale the pixel data, only the display.

### Capture pipeline

1. Hotkey → bring up overlay window(s) covering the **virtual screen** (union
   of all monitors).
2. Overlay freezes the screen content by drawing the pre-captured full-screen
   bitmap as its background (capture first, then show the frozen frame —
   avoids showing your own overlay in the shot).
3. User drags a rect → crop at native resolution from the frozen bitmap.
4. Send crop to the editor as base64 PNG over the bridge.

### Transparent overlay window

Tauri transparent windows on Windows use layered-window composition. It works
for simple cases but fights mixed-DPI and click-through. That's why the M2
overlay is isolated as spike #1 with an explicit native-Rust fallback, and why
the editor itself deliberately stays a normal opaque window.

### Clipboard gotchas

- "Copy image": `arboard` `set_image` writes a DIB with premultiplied alpha —
  correct for Slack/Word/PowerPoint. Verify a paste keeps transparency
  (targeted spike).
- "Copy file": write PNG to `%TEMP%`, then CF_HDROP with `SHAddToRecentDocs`.

### OCR (tesseract.js) integration

- Create worker with explicit `workerPath`, `langPath`, `corePath` pointing at
  bundled `public/tessdata` + `tessdata_fast` by default.
- Run on-demand (when the text tool activates), not on every capture — it's
  1–5s and must not block the overlay.
- Add a small benchmark harness (fixture screenshots → per-image accuracy +
  latency for `fast` vs `best`) so the default is data-driven.
- On-screen result: transparent text layer with word boxes so users can click
  a word to select/copy it.

### Protected/secure content

DWM cannot capture DRM-protected or the UAC secure desktop. Handle gracefully
("This content can't be captured") rather than shipping a black image.

### Tauri asset loading

Production loads the Vite `dist/` via Tauri's asset protocol; dev uses the
Vite dev server. Tesseract wasm/worker + traineddata must resolve under the
asset protocol — verify early (spike #3). Tauri user-data folder lands under
`%LOCALAPPDATA%` automatically.
