# GPUI Windows spike — capture overlay + pin window

Go/no-go spike for porting cleanshot-w's UI from Tauri 2 to pure-native Rust on
GPUI. Two binaries prove the two hardest window types the app needs:

- **`spike-overlay`** (Spike A): a full-virtual-screen, borderless, transparent,
  always-on-top window that rubber-band-selects a region across every monitor.
  Mouse-up logs the selected rect in global screen coordinates and exits;
  Escape cancels.
- **`spike-pin`** (Spike B): a frameless, always-on-top window that displays a
  PNG from the command line. Drag to move, `+`/`-` (or `=`) to change opacity,
  `t` toggles click-through, Escape or `q` quits.

The existing Tauri app (`src-tauri/`) is untouched; `spikes/gpui-overlay` is its
own Cargo workspace.

> Note: the CI workflow's `push` trigger is path-filtered, so artifacts build
> for pushes touching `spikes/gpui-overlay/**` or the workflow file (and for
> PRs touching them); unrelated pushes don't re-upload an artifact.

## Which pin worked, and why

**`gpui-pre = "0.3.9"` + `gpui-pre-platform = "0.3.9"`, both exact-relevant
weekly snapshots of Zed `main` (`zed@93d7456`, published 2026-10-12).**

- The official `gpui` crate is 0.2.2 and ~315 days stale; the community
  `gpui-pre` family (Longbridge's weekly snapshot pipeline, renamed crates,
  `[lib] name = "gpui"` preserved so `use gpui::*` still works) carries recent
  `main`, including the Windows per-pixel transparency and the non-focus-stealing
  popup work we depend on.
- The entry point is `gpui_platform::application()` from the **separate**
  `gpui-pre-platform` crate — `gpui-pre` alone does not expose a platform
  constructor (`Application::with_platform` needs a `Platform`). This is easy to
  miss; the crate's own examples import `gpui_platform`, which fails to resolve
  unless you also depend on `gpui-pre-platform`.
- Compiles cleanly on Linux x11 with no patches on the first try (`cargo check`
  ~2 min cold). Windows compilation is exercised by CI (below); we had no
  Windows machine, so the Windows-specific code paths were reviewed against the
  vendored `gpui-pre-windows` source, not run locally.
- Fallbacks (unused): official `gpui` 0.2.2 or a git pin of `zed` main. Not
  needed — no reason to trade freshness for either.

## What worked (verified on Linux x11, by source inspection for Windows)

| Concern | Result |
| --- | --- |
| Transparent windows | `WindowBackgroundAppearance::Transparent` on `WindowOptions.window_background`. Upstream fixed per-pixel transparency on Windows (zed PR #26645); gpui-pre 0.3.9 includes it. |
| Borderless + always-on-top on Windows | `WindowKind::PopUp` maps to `WS_EX_TOOLWINDOW | WS_EX_TOPMOST`, style `0` in `gpui-pre-windows/src/window.rs:492` — frameless, topmost, no taskbar entry. Exactly the overlay/pin shape. |
| Multi-monitor geometry | `cx.displays()` → `PlatformDisplay::bounds()` gives per-monitor bounds in **logical** px; the spike unions them into a logical virtual screen for the window. Raw `GetSystemMetrics(SM_XVIRTUALSCREEN / …)` (same calls as `src-tauri/src/capture.rs:557`) return **physical** px and are logged separately for comparison — see "Coordinate units under DPI scaling" below. |
| Rubber-band painting | `canvas()` element + `window.paint_quad(quad(...))` for the translucent fill and `PathBuilder::stroke(2px)` → `window.paint_path(...)` for the border — the same primitives as gpui's `painting` example. Redraw via `cx.notify()` on `on_mouse_move`. |
| Selection coords | Mouse events give window-relative positions; the spike adds the window's global origin (the virtual-screen origin, which can be negative on multi-monitor setups) before logging, so the rect is what a real capture would use. |
| Keyboard (Esc) | `actions!` + `cx.bind_keys([KeyBinding::new("escape", ..., None)])` + `div().key_context(...).track_focus(...)` + `window.focus(...)`. Requires the window to hold focus; works with `focus: true`. |
| Pin image | `image` crate decode → RGBA8 → **swizzle RGBA→BGRA** (`pixel.swap(0, 2)`) → `image::Frame::new` → `RenderImage::new(vec![frame])` → `img(Arc<RenderImage>)`. The swizzle is mandatory: see "RenderImage is BGRA" below. |
| Frameless drag | `Window::start_window_move()` is implemented for X11/Wayland but is a **no-op on Windows** (default trait impl). Windows path uses the classic trick behind `cfg(windows)`: `ReleaseCapture()` then `PostMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION, 0)`. HWND obtained through gpui's `HasWindowHandle` impl (`RawWindowHandle::Win32`). |
| Click-through | Raw Win32 via `windows-sys`: `SetWindowLongPtrW(GWL_EXSTYLE)` with `WS_EX_TRANSPARENT | WS_EX_LAYERED`. GPUI has no API for this. On Linux x11 the same toggle empties the X11 input shape region via `x11rb` (checked via `cookie.check()` so the request is actually flushed), so the toggle is testable in the dev loop too. This is **mouse** pass-through: the window keeps keyboard focus until the user clicks another window, which is deliberate — it's what lets you press `t` again to undo the toggle. |
| Opacity | `div().opacity(f32)` re-rendered from state; `+`/`-`/`=` bound as actions. |

## RenderImage is BGRA — swizzle decoded pixels (fixed on hardware)

The first hardware run showed a red↔blue channel swap: warm orange lanterns
rendered blue in the pin window. Root cause, with the code path:

- `RenderImage`'s buffer is documented and used as **BGRA** ("A cached and
  processed image, in BGRA format", `gpui/src/assets.rs`). GPUI's own decode
  path swizzles: `decode_static_image_from_decoder` in
  `gpui/src/platform.rs:3240` decodes to RGBA8 and then runs
  `pixel.swap(0, 2)` over every 4-byte pixel before building `Frame::new` →
  `RenderImage` (GIF and WebP paths in `gpui/src/elements/img.rs` do the same).
- The Windows renderer uploads those bytes into a
  `DXGI_FORMAT_B8G8R8A8_UNORM` texture (`gpui-pre-windows/src/directx_atlas.rs:185`,
  render target likewise `directx_renderer.rs:27`), so RGBA bytes in a
  `RenderImage` render with red and blue exchanged.
- Our first `load_png` handed the `image` crate's raw RGBA8 output straight to
  `RenderImage`, skipping the swizzle. On a BGRA renderer that's exactly
  orange↔blue. The fix mirrors GPUI's own swizzle in `load_png`
  (`spikes/gpui-overlay/src/bin/pin.rs`).

The spike never caught this on Linux because the dev loop never rendered
(cargo check/build only); it surfaced on the first real Windows run.

## Coordinate units under DPI scaling (verified on hardware at 150%)

Hardware data point: one 1920×1200-physical display at 150% scaling. GPUI
reported display bounds 1280×800 (logical), raw `GetSystemMetrics` reported
1920×1200 (physical). The units split:

- **GPUI `Pixels` are logical px everywhere**: `PlatformDisplay::bounds()`
  (Windows impl divides physical monitor rects by the monitor's scale factor,
  `gpui-pre-windows/src/display.rs`), `WindowOptions::window_bounds`, and
  mouse-event positions (`MouseDownEvent::position` etc., window-relative).
- **Win32 is physical px**: `GetSystemMetrics(SM_*VIRTUALSCREEN)` and GDI
  (`BitBlt` capture, `CreateCompatibleBitmap`) — the process is per-monitor-v2
  DPI-aware, so real physical values come through (no 96-DPI virtualization).
- **Verified conversion for the tested single-monitor case:**
  `physical = logical × scale_factor`, where `window.scale_factor()` was 1.5.
  This reproduces the observed 1920×1200 physical screen from the 1280×800
  logical display and is correct for a uniform-DPI screen.
- **Mixed-DPI conversion is not implemented or hardware-verified.** The spike
  multiplies the entire selection by one `window.scale_factor()`; that is not
  valid for a selection crossing monitors with different scale factors. GPUI
  does not expose a per-display scale factor on `PlatformDisplay`, and this
  full-screen overlay has no per-monitor coordinate transform. The
  vendored Windows display implementation (`gpui-pre-windows/src/display.rs`)
  constructs each `PlatformDisplay::bounds()` by dividing that monitor's
  physical origin and size by its scale factor (`logical_point` in
  `gpui-pre-windows/src/util.rs`). The inverse for a point on one monitor is
  `physical = logical × that_monitor_scale_factor`. To convert a mixed-DPI
  selection, the port must map it piecewise using each monitor's physical
  origin and scale factor; a practical route is to enumerate the physical
  monitor rectangles, obtain each factor with `GetDpiForMonitor`, and map the
  corresponding GPUI logical bounds back to physical coordinates. The
  transform, full-screen overlay behavior, and a selection spanning monitors
  with different DPI remain to be implemented and tested on hardware.

Consequence for the port's capture path: the overlay selection arrives in
global **logical** px. For the tested single-display 150% case, multiply by
1.5 before calling GDI; for mixed DPI, do not use that single factor. The first
version of this spike got the window bounds wrong in a subtle way: it unioned
GPUI's logical display bounds with the raw **physical** virtual screen and
took the larger, which at 150% produced a 1920×1200 *logical* overlay window —
1.5× the actual screen. The union is now logical-only (for the window), and
`spike-overlay` logs both unit systems side by side at startup and prints the
selection rect in both (`selection (logical): …` / `selection (uniform-DPI
physical estimate, x1.5): …`). The physical output is an estimate valid only
when the selected area has one scale factor; it uses the overlay window's
single scale factor and is not a mixed-DPI conversion.

## Known limitations and risks

- **Remaining hardware unknowns**: the first Windows run (single 150% display)
  verified both spikes end to end — overlay selection, cancel, global rect
  logging, pin drag, opacity, click-through — and this is a GO. Not yet
  exercised on hardware: multi-monitor arrangements, any mixed-DPI conversion
  (not implemented; see above), and the NVIDIA alpha path below.
- **Click-through + DirectX is the known fragile spot.** `WS_EX_LAYERED` on a
  DirectX-swapchain window is exactly the configuration implicated in the
  NVIDIA + DirectComposition alpha-zeroing bug (zed issue #55654). If a
  click-through pin window blanks its content, first try
  **`GPUI_DISABLE_DIRECT_COMPOSITION=1`** — gpui-pre-windows reads that env var
  (`directx_renderer.rs:26`) and falls back off DirectComposition.
- **Linux dev-loop shape differs from the Windows shape.** `WindowKind::PopUp`
  is override-redirect on X11, and WMs never hand override-redirect windows
  keyboard focus, so Esc would be dead in the dev loop; the spikes therefore
  use `WindowKind::Normal` + client-side decorations on Linux and `PopUp` on
  Windows. Topmost-on-Linux is not wired (the dev loop doesn't need it; on
  Windows PopUp provides it). Wayland is untested entirely.
- The overlay scrim (15% black) covers the whole screen including all monitors;
  on Windows the PopUp tool-window has no taskbar button, which matches capture
  UX. GPUI reports per-display logical bounds, but the spike does not implement
  mixed-DPI selection-to-GDI coordinate conversion; its one-window logical
  bounds and single scale factor are not evidence that a mixed-DPI overlay
  spans or maps every monitor correctly.
- `spike-pin` decodes the PNG fully into memory at startup; no progressive
  loading, which is fine for a spike (pins are screenshots).
- `gpui-pre` re-publishes weekly and snapshots can break APIs; both crates are
  pinned to the exact `=0.3.9` requirement (plus a committed `Cargo.lock`) so a
  lockfile refresh can't silently bump onto a breaking snapshot. Bump
  deliberately when the port starts.

## How to run on Windows

Download the `gpui-spike-windows` artifact from the latest workflow run on this
branch (Actions → "Spike Windows build" → run for your commit → Artifacts) and
unzip it. Run `spike-overlay.exe`: the screen dims across every monitor, drag a
blue rectangle, and mouse-up prints the selected rect in logical coordinates
and a uniform-DPI physical estimate, then exits — Escape cancels instead. Run
`spike-pin.exe C:\path\to\image.png`
to open a frameless always-on-top window showing the PNG; drag it to move, press
`+`/`-` for opacity in 10% steps, press `t` to toggle full click-through (mouse
and keyboard pass through the window entirely), and Escape or `q` to quit. If
the pin window goes blank after `t` on an NVIDIA machine, set
`GPUI_DISABLE_DIRECT_COMPOSITION=1` and rerun, then note it in this file.
