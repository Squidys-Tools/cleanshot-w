# Snipsy GPUI app — phase 2 notes

Phase 1 added the tray shell, global hotkey, capture overlay, GDI region
capture, library, and pin window. Phase 2 adds the native annotation editor:
image-space document model, shapes, undo/redo, crop, and full-resolution
flatten/export. OCR, blur/pixelate/redact, window capture, autostart, icon
library, and preferences remain later phases.

The stack is `gpui-overlay-spike` (PR #35) → `gpui-port-1` (PR #36) →
`gpui-editor`. The spike's overlay/pin/window code was copied (not imported)
into this app tree. The phase-2 changes stay in `app/` and the Windows build
workflow.

## Layout

`app/` is its own Cargo workspace with two binaries: `cleanshot-w`
(`app/target/release/cleanshot-w.exe`) and `cleanshot-editor`
(`app/target/release/cleanshot-editor.exe`, a standalone image-path test
entry point for the same editor window):

| Module | Ported from | Notes |
| --- | --- | --- |
| `platform.rs` | `spikes/gpui-overlay/src/lib.rs` | Virtual-screen geometry, OS drag, click-through, plus the two fixes below. |
| `capture.rs` | `src-tauri/src/capture.rs` | GDI BitBlt/GetDIBits virtual-screen snapshot + crop. Window capture (`PrintWindow`) deliberately left out of phase 1. |
| `library.rs` | `src-tauri/src/library.rs` | Same on-disk format (shared with the Tauri app); list returns metadata, images read on demand — no base64 wire payloads. |
| `settings.rs` | `src-tauri/src/hotkeys.rs` | Same `settings.json` schema and canonical Snipsy location; one-time migration from the old `CleanShotW` directory; read-only until preferences UI. |
| `hotkey.rs` | `src-tauri/src/hotkeys.rs` | `RegisterHotKey` on a dedicated thread with its own `GetMessageW` pump; events cross to GPUI over a channel. |
| `overlay.rs` / `pin.rs` | the spike's `overlay.rs` / `pin.rs` | Same window shapes; wired into the app flow instead of exiting after one use. |
| `clipboard.rs` | `src-tauri/src/clipboard.rs` | arboard image copy (RGBA in, platform format handled by arboard). |
| `save_dialog.rs` | — | `rfd` file dialog replaces the Tauri anchor-download hack. |
| `tray.rs` | — | `tray-icon` directly (what Tauri's tray plugin wraps). Windows-only until the Linux loop has a use for it; `--capture` / `--pin <file>` drive the flow there. |
| `editor/model.rs` | new | Version-1 JSON document; shape geometry is in original image pixels and the document carries its capture id. Old tldraw `annotations.json` files are not parsed; those captures open un-annotated. |
| `editor/history.rs` / `hit_test.rs` | new | Snapshot undo/redo with interaction stopping points, hit-testing, and resize handles. Crop snapshots include the base image so crop undo restores pixels. |
| `editor/flatten.rs` | new | tiny-skia CPU rasterization at original image dimensions, with fontdue text; save and clipboard use the flattened PNG. |
| `editor/window.rs` | new | GPUI canvas, zoom/pan, selection, shape tools, text editing, crop, palette/width controls, and export actions. |

## The two hardware bugs, fixed at the source

**Color channel order.** The spike rendered orange lanterns blue on hardware.
Root cause, from the pinned `gpui-pre` 0.3.9 source: `RenderImage` is
documented as **BGRA** (`gpui-pre/src/assets.rs`: "A cached and processed
image, in BGRA format"), GPUI's own decode path swizzles RGBA→BGRA
(`gpui-pre/src/platform.rs` `decode_static_image_from_decoder`), and the
Windows atlas uploads frame bytes verbatim into a
`DXGI_FORMAT_B8G8R8A8_UNORM` texture (`gpui-pre-windows/src/directx_atlas.rs`
`upload` — no swizzle there). The spike fed raw RGBA into `Frame::new`. The
app swizzles decoded PNGs to BGRA once in `platform::rgba_to_bgra` before
`RenderImage::new`, on every platform (GPUI's shared decode path does the
same everywhere). `capture.rs` stores RGBA (GetDIBits BGRA is swapped at
read time, as in the Tauri app), so pin display swizzles once at
`RenderImage` construction.

**DPI: GPUI-logical vs Win32-physical.** At 150% scaling the spike observed
GPUI display bounds 1280×800 in a 1920×1200 virtual screen. The rule:
`physical = logical × scale`, where `scale = window.scale_factor()` is
`GetDpiForWindow/96`. Two details matter:

1. GPUI does **not** set process DPI awareness; `GetDpiForWindow` only
   reports true per-monitor DPI in a DPI-aware process. The app calls
   `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` at startup
   (`capture::configure_dpi_awareness`, ported from the Tauri app), which is
   also what makes `GetSystemMetrics`/`BitBlt` speak true physical pixels.
2. GPUI logical and Win32 physical coordinate spaces do not share an origin
   in general (monitors left of the primary give a negative Win32 virtual
   origin). `platform::selection_to_physical` translates through both
   virtual-screen origins, then scales and rounds. Covered by unit tests,
   including the 150% case (220,261,826,248 logical → 330,392,1239,372
   physical).

Known limit (documented extension point): mixed-DPI multi-monitor. GPUI
logical coordinates across mixed-DPI monitors don't scale by one factor; the
conversion uses the overlay window's scale for the whole selection. The
validated hardware case is single-display 150%.

## Capture flow

Hotkey or tray "New capture" → full-virtual-screen GDI snapshot *before*
the overlay opens (so the overlay scrim never lands in the PNG, same order
as the Tauri app) → overlay rubber-band selection (logical px) → convert to
physical, crop, encode PNG → save into the library (atomic index update) →
open the editor. Tray "Open last" reloads the latest library image and its
version-1 document. The editor stores `annotations.json` alongside
`image.png`; crop replaces the library image and repositions/intersects shapes.
`Ctrl+S` saves a full-resolution flattened PNG; `Ctrl+C` copies that flattened
image. The standalone `cleanshot-editor <image.png>` binary opens a transient
image without library persistence.

## Verified vs not

Phase 1 passes Linux formatting, strict Clippy, all-target checks, 20 unit
tests, and a release build. The Windows GNU-target check and Clippy also pass.
The `windows-2025` job runs Clippy and all-target tests before building both
binaries. Phase 1 passed on Windows hardware at 150% scaling.

The Linux cloud desktop did not display a GPUI window, so the editor UI and
export are not visually verified. Phase-two behavior still needs a Windows
hardware run.

## How to run on Windows

Download the `cleanshot-w-app-windows` artifact (Actions → "App Windows
build" → your commit → Artifacts) and run `cleanshot-w.exe`. It lives in the
tray, and the hotkey (default `ctrl+shift+4`, read from
`%LOCALAPPDATA%/Snipsy/settings.json`) starts a capture. If `Snipsy` does not
exist but `%LOCALAPPDATA%/CleanShotW` does, the first launch migrates the old
settings and capture library; an existing `Snipsy` directory wins. A second
launch exits (single-instance mutex), and a successful capture opens the
editor. To test it separately, run `cleanshot-editor.exe <image.png>`.

## Extension points for phase 3+

- `editor/model.rs` is independent of GPUI and stores only app-owned
  version-1 documents; no tldraw migration is attempted.
- `editor/flatten.rs` is CPU-only so export remains independent of display
  zoom and GPU readback.
- `capture.rs` keeps the full-screen-then-crop shape, so a future
  "window capture" path can hand a different `CaptureSurface` to the same
  crop/library/editor pipeline. Cursor drawing is ported and driven by the
  `includeCursor` setting.
- Hotkey settings are read at startup; re-registration on settings change
  slots into `hotkey.rs`.
