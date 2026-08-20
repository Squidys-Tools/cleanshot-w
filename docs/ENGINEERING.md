# Engineering notes

This document is the stable implementation reference for contributors. It is
not a task list: planned work belongs in the [roadmap](../ROADMAP.md), completed
changes belong in the [changelog](../CHANGELOG.md), and Windows release evidence
belongs in the [M2 checklist](WINDOWS-M2-RELEASE-CHECKLIST.md).

## Stack and boundaries

- **Frontend:** React 19, Vite, TypeScript 7, tldraw, and plain CSS.
- **OCR:** tesseract.js with local assets under `public/tessdata`.
- **Native shell:** Tauri 2, Rust 2021, WebView2, `arboard`, and Windows APIs
  through `windows-sys`.
- **Browser persistence:** IndexedDB for capture images, thumbnails, annotations,
  timestamps, and titles.
- **Native persistence:** JSON and PNG files under
  `%LOCALAPPDATA%\CleanShotW\library`.

The editor talks to a typed `HostBridge` in `src/lib/bridge.ts`. The browser host
uses IndexedDB and browser APIs; the native host translates the same operations
to Tauri commands. Keep UI code independent of the storage or capture backend.

## Runtime architecture

The normal application is a React editor in the main Tauri window. Native capture
uses short-lived Tauri windows and events:

1. The global shortcut, tray action, or toolbar starts capture.
2. Rust hides the editor and captures a physical-pixel virtual-screen snapshot.
3. The transparent overlay lets the user select a crop over that frozen frame.
4. Rust validates the crop, encodes a PNG, and emits `capture:completed`.
5. The editor stores the image, creates a thumbnail, and opens the capture.

The browser path remains useful for development: it accepts pasted, dropped, or
selected image files without requiring native capture.

### Capture data

`CaptureRecord` in `src/types.ts` is the shared model:

- `id`, `title`, `createdAt`, and `updatedAt`
- natural image dimensions
- full image and thumbnail `Blob`s
- serialized tldraw annotation state

Browser history stores the record directly in IndexedDB. The native library
stores one directory per capture and an index for ordering:

```text
%LOCALAPPDATA%\CleanShotW\
  settings.json
  library\\
    index.json
    <capture-id>\\
      image.png
      thumbnail.png
      annotations.json
```

Native IDs and index entries are validated before they are used as paths. Writes
are atomic where the platform permits it. Title updates preserve the image,
thumbnail, and annotations while updating the library timestamp.

### Native command groups

The exact command registration is in `src-tauri/src/lib.rs`; the groups are:

| Capability | Native implementation |
|---|---|
| Area capture | `start_area_capture`, `complete_area_capture`, and `cancel_area_capture` |
| Full-screen/window capture | GDI capture in `capture.rs` |
| Clipboard | PNG/image, file, text, and image-read commands in `clipboard.rs` |
| Library | Save, list, get, annotation update, title update, and delete in `library.rs` |
| Settings | Hotkey, cursor, and startup settings in `hotkeys.rs` |
| Pins | Dedicated always-on-top windows in `pin.rs` |

## Native correctness notes

- The process requests Per-Monitor V2 DPI awareness and keeps native crop
  coordinates in physical pixels.
- Virtual-screen origins can be negative when a monitor is left of or above the
  primary display; do not assume `(0, 0)` is the top-left of the desktop.
- Capture the desktop before showing the overlay so the overlay cannot appear in
  the captured image.
- Window capture uses `PrintWindow` and falls back to a GDI screen copy when the
  target does not render through `PrintWindow`.
- Cursor compositing is controlled by the persisted setting and is performed in
  native capture space.
- Native failures are returned as strings and normalized by
  `nativeErrorMessage` before reaching the UI. A failed capture must restore the
  editor instead of leaving the overlay or main window hidden.
- Clipboard image and file commands validate PNG data and preserve RGBA pixels.

These behaviors need a real Windows run. The browser cannot validate DPI,
WebView2 asset loading, GDI output, tray lifecycle, clipboard interoperability,
or protected-content behavior.

## OCR assets

The browser OCR worker, core files, and `eng.traineddata` are bundled under
`public/tessdata`. Use the asset script when refreshing them:

```sh
bun run ocr:assets
```

OCR is on demand rather than running for every capture. Packaged builds must be
tested with network access disabled so the app cannot accidentally depend on a
remote worker or language file. Native OCR remains an M3 evaluation item.

## Local development and verification

Install dependencies with the lockfile:

```sh
bun install --frozen-lockfile
```

Frontend development and checks:

```sh
bun run dev
bun test
bun tsc -b --noEmit
bun run build
```

The browser smoke harness uses Playwright Chromium. Start Vite in one terminal,
then run the smoke suite in another:

```sh
bun run dev -- --host 127.0.0.1
bun run smoke
```

Native development and checks run on Windows:

```sh
bun run tauri dev
bun run tauri build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

The GitHub Actions jobs mirror these checks:

- `.github/workflows/ci.yml` — Bun tests and frontend build
- `.github/workflows/smoke.yml` — Playwright browser smoke test
- `.github/workflows/rust.yml` — Windows formatting, Clippy, tests, and check
- `.github/workflows/release.yml` — draft Windows release on `v*` tags

## Packaging and release

Tauri currently targets an NSIS installer configured for `currentUser`, so the
installer does not require elevation. The release workflow runs on Windows and
creates a draft GitHub release when a `v*` tag is pushed. A portable ZIP and
SHA-256 sidecars remain release-planning work; do not document them as available
downloads until the workflow produces them.

Before publishing, complete the [Windows M2 checklist](WINDOWS-M2-RELEASE-CHECKLIST.md)
against the packaged artifact. Record the tested build/version, Windows and
WebView2 versions, artifact, failures, limitations, and release decision there.
