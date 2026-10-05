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
`resolvePackageJsonExports: false` in `tsconfig.json`. tldraw, declared as
`^5.4.2`, ships its declaration files beside its `main` entry but does not
publish a `types` condition in its package export map. The setting keeps the
compiler on TypeScript 7 while allowing it to resolve those declarations.

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

## tldraw licensing

The editor requires a tldraw license key. This is not optional and not a
formality: **a production build without a key renders a blank editor.**

Five seconds after `<Tldraw>` mounts, `LicenseProvider` in `@tldraw/editor`
checks the license state. If it is `unlicensed-production` or `expired`, it
discards the editor and renders an invisible placeholder:

```js
function shouldHideEditorAfterDelay(licenseState) {
  return licenseState === "expired" || licenseState === "unlicensed-production";
}
```

The visible effect is that the canvas, the annotation dock, and the status bar
all disappear while the rest of the app keeps working. No exception is thrown.
The console shows only a styled banner.

Three properties of this gate make it easy to ship by accident:

- **Dev builds are exempt.** `LicenseManager.getIsDevelopment()` treats loopback
  and `*.localhost` hosts as development unless `process.env.NODE_ENV ===
  "production"`, so `bun run dev` and the browser smoke suite report
  `unlicensed` and render normally. The packaged app serves from
  `http://tauri.localhost/`, where `NODE_ENV` is inlined as `production`.
- **It is delayed by five seconds**, so a quick look after opening a capture can
  show a working editor.
- **Nothing fails loudly.** The build succeeds and every browser check passes.

Set the key in the build environment. Vite inlines `VITE_`-prefixed variables,
so that is the reliable choice:

```sh
VITE_TLDRAW_LICENSE_KEY=... bun run tauri build
```

`bun run check:license` fails when no key is present. It gates the release
workflow, so a tag cannot publish an installer with a dead editor. In the CI
workflow it runs with `--warn-only` and only reports, because that job runs on
every push to main and a missing key is a licensing decision rather than a build
regression. Pass `--warn-only` to check without failing anywhere else.

The key comes from a repository secret. Add `VITE_TLDRAW_LICENSE_KEY` under
Settings, Secrets and variables, Actions. Both workflows read
`TLDRAW_LICENSE_KEY` and `VITE_TLDRAW_LICENSE_KEY`, so either name works.

To confirm a packaged build is actually licensed, run the diagnostic harness and
check that `.cs-ui` and at least one `<canvas>` exist once a capture is open:

```sh
bun run diagnose:packaged -- --exe "src-tauri\target\release\cleanshot-w.exe"
```

## Diagnosing a packaged build

Release builds ship without devtools, so a packaged-only fault cannot be
inspected from the outside. `scripts/diagnose-packaged.mjs` re-enables the
WebView2 DevTools Protocol for the packaged process, captures console output and
uncaught exceptions, then measures the real geometry of the editor subtree.

```sh
bun run diagnose:packaged -- --exe "src-tauri\target\release\cleanshot-w.exe" \
  --open-capture "<history entry title>" --out report.json
```

It prints the ancestor chain of `.cs-tldraw` with a `ZERO-SIZED` marker, the
computed style of the tldraw nodes, backing-store size against painted size for
every canvas, and `devicePixelRatio`. Use it before theorising about a
Windows-only rendering fault. This is how release-gate Finding 1 was attributed
in minutes rather than a second manual gate pass.

## OCR and packaged assets

The browser OCR worker, core files, and `eng.traineddata` are bundled under
`public/tessdata`. Use the asset script when refreshing them:

```sh
bun run ocr:assets
```

OCR runs on demand rather than for every capture. Packaged builds must be tested
with network access disabled so the app cannot accidentally depend on a remote
worker or language file. Native OCR remains an M3 evaluation item.

The remote Google Fonts stylesheet that `index.html` used to load has been
removed for the same reason. It was render-blocking, so a packaged build on an
offline or restricted machine stalled first paint and made text metrics depend
on the network. Inter is still used when the OS provides it; the stack in
`App.css` falls through to Segoe UI otherwise.

## Local development and verification

Install dependencies with the lockfile:

```sh
bun install --frozen-lockfile
```

Frontend development and checks:

```sh
bun run dev
bun test
bun run typecheck
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
bun run rust:check
```

`bun run rust:check` runs `cargo fmt --check`, Clippy with `-D warnings`, and
`cargo test --no-default-features --lib`; there is no separate `cargo check`
step. That test scope is narrower than CI's, so a green local run is not
equivalent coverage: `.github/workflows/rust.yml` runs a bare
`cargo test --manifest-path src-tauri/Cargo.toml` across the default-feature
package and all targets. Run the full CI command before trusting the test set.

The local script also skips the tests on a windows-gnu toolchain, because those
harness executables lack the comctl32 v6 manifest Tauri embeds in the real
binary and abort at load, so Rust tests stay on CI's msvc runner.

The GitHub Actions jobs cover more than this script, and do not all mirror it:

- `.github/workflows/ci.yml` - Bun tests, typecheck, frontend build, and a
  non-blocking tldraw license report. Runs on pull requests that touch the
  frontend, tests, or scripts, and on every push to main.
- `.github/workflows/smoke.yml` - Playwright browser smoke test
- `.github/workflows/rust.yml` - Windows formatting, Clippy, and tests
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
