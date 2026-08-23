# Engineering

This is the stable implementation reference for contributors. Planned work
belongs in [the roadmap](ROADMAP.md), completed changes belong in [the
changelog](CHANGELOG.md), product decisions belong in [the product brief](PRODUCT.md),
visual rules belong in [the design system](DESIGN.md), and current release
evidence belongs in [the roadmap and status](ROADMAP.md).

## Stack and boundaries

- **Frontend:** React 19, Vite 8, TypeScript 7, tldraw, and plain CSS.
- **OCR:** tesseract.js with local assets under `public/tessdata`.
- **Native shell:** Tauri 2, Rust 2021, WebView2, `arboard`, and Windows APIs
  through `windows-sys`.
- **Browser persistence:** IndexedDB for capture images, thumbnails, annotations,
  timestamps, and titles.
- **Native persistence:** JSON and PNG files under
  `%LOCALAPPDATA%\CleanShotW\library`.

The editor talks to a typed `HostBridge` in `src/lib/bridge.ts`. The browser
host uses IndexedDB and browser APIs. The native host translates the same
operations to Tauri commands. Keep UI code independent of storage and capture
backends.

TypeScript 7 uses `moduleResolution: "bundler"` with
`resolvePackageJsonExports: false` in `tsconfig.json`. tldraw 5.3.1 ships its
declaration files beside its `main` entry but does not publish a `types`
condition in its package export map. The setting keeps the compiler on
TypeScript 7 while allowing it to resolve those declarations.

## Runtime architecture

CleanShot W uses one React editor in two host environments:

```text
React editor
  ├─ browser host: IndexedDB, browser clipboard, paste/drop/file intake
  └─ native host: Tauri commands, Windows capture, disk library, clipboard
```

The main window and capture overlay are separate surfaces. The main window owns
annotation, export, OCR, history, and settings. The overlay owns screen
selection and returns a captured crop. A capture never stays in the overlay for
annotation.

The browser host remains useful for development. It accepts pasted, dropped, or
selected image files without requiring native capture.

## Capture lifecycle

1. A toolbar action, tray action, or global shortcut starts capture.
2. Rust hides the editor and records a physical-pixel snapshot of the virtual
   desktop.
3. A transparent overlay displays that frozen frame and accepts a selection.
4. Rust validates the crop, encodes a PNG, and emits `capture:completed`.
5. The editor stores the image, creates a thumbnail, and opens the capture.
6. Escape or Cancel restores the editor if the capture is abandoned.

Window capture enumerates titled windows and uses `PrintWindow` with a GDI
fallback. Full-screen capture uses the virtual desktop. The overlay appears
after the source frame is captured so it cannot appear in the result.

## Editor document and export

The shared capture record lives in `src/types.ts` and contains:

- capture id, title, creation time, and update time;
- natural image dimensions;
- the full image and thumbnail as `Blob`s;
- serialized tldraw document state.

The editor adds the screenshot as a locked image shape at the document origin.
Markup is stored as tldraw shapes, including the custom counter, blur, pixelate,
and redact shapes. The screenshot remains the visual base while the annotation
dock and selection actions operate above it.

Export asks tldraw to render the current page at the image's natural dimensions.
The flattened PNG path feeds Save PNG, Copy image, Copy file, and pin windows.

## Persistence

The browser host stores capture records in IndexedDB. The native host stores the
library under `%LOCALAPPDATA%\CleanShotW`:

```text
settings.json
library\
  index.json
  <capture-id>\
    image.png
    thumbnail.png
    annotations.json
```

Native ids and index entries are validated before they become paths. Annotation
autosaves are debounced, and native writes use temporary files before replacing
the destination where the platform permits it.

## Native command groups

The exact command registration is in `src-tauri/src/lib.rs`:

| Capability | Native implementation |
|---|---|
| Area capture | `start_area_capture`, `complete_area_capture`, `cancel_area_capture` |
| Window/full-screen capture | GDI capture in `capture.rs` |
| Clipboard | PNG/image, file, text, and image-read commands in `clipboard.rs` |
| Library | Save, list, open, annotation update, title update, and delete in `library.rs` |
| Settings | Hotkey, cursor, and startup settings in `hotkeys.rs` |
| Pins | Dedicated always-on-top windows in `pin.rs` |

## Windows constraints

- The process uses Per-Monitor V2 DPI awareness.
- Native crop coordinates stay in physical pixels. Convert CSS coordinates only
  at the bridge boundary.
- Virtual-screen origins may be negative when a monitor sits left of or above
  the primary display.
- Capture the desktop before showing the overlay so the overlay cannot appear in
  the captured image.
- GDI and `PrintWindow` behavior varies by application and must be checked on
  representative Windows hardware.
- The UAC secure desktop and DRM-protected content cannot be captured. Return a
  clear error instead of treating a black image as a valid result.
- Clipboard image output must preserve RGBA data when pasted into common
  Windows applications.
- Native failures are returned as strings and normalized by
  `nativeErrorMessage` before reaching the UI. A failed capture must restore
  the editor instead of leaving the overlay or main window hidden.

These behaviors need a real Windows run. The browser cannot validate DPI,
WebView2 asset loading, GDI output, tray lifecycle, clipboard interoperability,
or protected-content behavior.

## OCR and packaged assets

The browser OCR worker, core files, and `eng.traineddata` are bundled under
`public/tessdata`. Use the asset script when refreshing them:

```sh
bun run ocr:assets
```

OCR runs on demand rather than for every capture. Packaged builds must be tested
with network access disabled so the app cannot accidentally depend on a remote
worker or language file. Native OCR remains an M3 evaluation item.

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

If the local Bun install does not expose the package binary, run the checker
directly with `bunx --package typescript@7.0.2 tsc --noEmit`.

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

- `.github/workflows/ci.yml` - Bun tests and frontend build
- `.github/workflows/smoke.yml` - Playwright browser smoke test
- `.github/workflows/rust.yml` - Windows formatting, Clippy, tests, and check
- `.github/workflows/release.yml` - draft Windows release on `v*` tags

## Packaging and release

Tauri currently targets an NSIS installer configured for `currentUser`, so the
installer does not require elevation. The release workflow runs on Windows and
creates a draft GitHub release when a `v*` tag is pushed. A portable ZIP and
SHA-256 sidecars remain release-planning work. Do not document them as available
downloads until the workflow produces them.

The remaining Windows acceptance work is recorded in [the roadmap and status](ROADMAP.md).
Run it against the packaged artifact and record the tested build, Windows and
WebView2 versions, artifact, failures, limitations, and release decision there.
