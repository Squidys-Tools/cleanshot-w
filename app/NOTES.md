# Snipsy GPUI app — phase 1 notes

Phase 1 vertical slice of the Tauri → pure-native GPUI port: tray shell,
global hotkey, capture overlay, GDI region capture, capture library, pin
window, clipboard copy, save dialog. The editor, OCR, window capture,
autostart, and preferences UI are later phases.

Branches off `gpui-overlay-spike` (PR #35); the spike's overlay/pin/window
code was copied (not imported) into this app tree. Nothing outside `app/`
and the new workflow was changed.

## Layout

`app/` is its own Cargo workspace with one binary crate, `cleanshot-w`
(`app/target/release/cleanshot-w.exe`):

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
pin window shows the capture → `ctrl+c` copies as image, `ctrl+s` opens a
save dialog, Esc closes, `+`/`-` opacity, `t` click-through, drag to move.

## Verified vs not

Verified on Linux x11: `cargo build` (full link), `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test` (20 tests:
selection validation, crop, PNG round-trip, DPI conversion, swizzle,
settings shortcut validation and hotkey parsing, library validation).
Windows-only code paths
(`cfg(windows)`: tray, RegisterHotKey thread, GDI capture, single-instance
mutex) do not compile on Linux and were written against the vendored
gpui-pre-windows source and the Tauri app's proven capture code. The
`windows-2025` CI job runs Clippy, unit tests, and the release build, but
**runtime behavior on Windows hardware is not yet verified** — that is what
the artifact is for.
In particular unverified-on-hardware: the color fix (logic verified against
gpui-pre source), the exact DPI conversion at 150% (unit-tested, not run),
tray menu event delivery through GPUI's message loop, and `WM_HOTKEY`
delivery from the hotkey thread.

## How to run on Windows

Download the `cleanshot-w-app-windows` artifact (Actions → "App Windows
build" → your commit → Artifacts) and run `cleanshot-w.exe`. It lives in the
tray; the hotkey (default `ctrl+shift+4`, read from
`%LOCALAPPDATA%/Snipsy/settings.json`) starts a capture. If the Snipsy
directory does not exist but `%LOCALAPPDATA%/CleanShotW` does, the first
launch renames that legacy directory to `Snipsy`, keeping settings and
capture-library data together; an existing `Snipsy` directory always wins.
A second launch
exits (single-instance mutex).

## Extension points for phase 2+

- `flow::AppState` is the app-wide state root; the editor window plugs in
  next to the pin window.
- `library.rs` annotations support is already in the index format; the
  editor writes them.
- `capture.rs` keeps the full-screen-then-crop shape, so a future
  "window capture" path can hand a different `CaptureSurface` to the same
  crop/library/pin pipeline. Cursor drawing is ported and driven by the
  `includeCursor` setting.
- Hotkey settings are read at startup; re-registration on settings change
  slots into `hotkey.rs`.
