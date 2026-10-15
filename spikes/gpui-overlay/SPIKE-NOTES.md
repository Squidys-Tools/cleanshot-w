# GPUI Windows spike — capture overlay + pin window

Go/no-go spike for porting cleanshot-w's UI from Tauri 2 to pure-native Rust on
GPUI. Two binaries prove the two hardest window types the app needs:

- **`spike-overlay`** (Spike A): a full-virtual-screen, borderless, transparent,
  always-on-top window that rubber-band-selects a region across every monitor.
  Mouse-up logs the selected rect in global screen coordinates and exits;
  Escape cancels.
- **`spike-pin`** (Spike B): a frameless, always-on-top window that displays a
  PNG from the command line. Drag to move, `+`/`-` (or `=`) to change opacity,
  `t` toggles full click-through, Escape or `q` quits.

The existing Tauri app (`src-tauri/`) is untouched; `spikes/gpui-overlay` is its
own Cargo workspace.

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
| Multi-monitor geometry | `cx.displays()` → `PlatformDisplay::bounds()` gives per-monitor bounds; the spike unions them into a virtual screen, then cross-checks against raw `GetSystemMetrics(SM_XVIRTUALSCREEN / SM_YVIRTUALSCREEN / SM_CXVIRTUALSCREEN / SM_CYVIRTUALSCREEN)` (same approach as `src-tauri/src/capture.rs:557`) and takes the larger span. Raw metrics come from `windows-sys` behind `cfg(windows)`. |
| Rubber-band painting | `canvas()` element + `window.paint_quad(quad(...))` for the translucent fill and `PathBuilder::stroke(2px)` → `window.paint_path(...)` for the border — the same primitives as gpui's `painting` example. Redraw via `cx.notify()` on `on_mouse_move`. |
| Selection coords | Mouse events give window-relative positions; since the overlay window's origin is the virtual-screen origin, the logged rect is already global. |
| Keyboard (Esc) | `actions!` + `cx.bind_keys([KeyBinding::new("escape", ..., None)])` + `div().key_context(...).track_focus(...)` + `window.focus(...)`. Requires the window to hold focus; works with `focus: true`. |
| Pin image | `image` crate decode → RGBA8 → `image::Frame::new(rgba)` → `RenderImage::new(vec![frame])` → `img(Arc<RenderImage>)`. The img element converts RGBA→BGRA itself. |
| Frameless drag | `Window::start_window_move()` is implemented for X11/Wayland but is a **no-op on Windows** (default trait impl). Windows path uses the classic trick behind `cfg(windows)`: `ReleaseCapture()` then `PostMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION, 0)`. HWND obtained through gpui's `HasWindowHandle` impl (`RawWindowHandle::Win32`). |
| Click-through | Raw Win32 via `windows-sys`: `SetWindowLongPtrW(GWL_EXSTYLE)` with `WS_EX_TRANSPARENT | WS_EX_LAYERED`. GPUI has no API for this. On Linux x11 the same toggle empties the X11 input shape region via `x11rb` (SO::SET / SK::INPUT), so the toggle is testable in the dev loop too. |
| Opacity | `div().opacity(f32)` re-rendered from state; `+`/`-`/`=` bound as actions. |

## Known limitations and risks

- **Nothing here has run on real Windows yet.** The GitHub Actions workflow
  (`.github/workflows/spike-win-build.yml`) builds both exes on `windows-2025`
  and uploads them as the `gpui-spike-windows` artifact on every push; running
  those on actual hardware is the remaining test. The Windows-specific code
  (PopUp styles, `WM_NCLBUTTONDOWN` drag, `WS_EX_TRANSPARENT` toggle, raw
  virtual-screen metrics) was written against the vendored `gpui-pre-windows`
  source but never executed.
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
  UX. Multi-monitor DPI scaling is handled by GPUI (per-display scale factors),
  but mixed-DPI rect math is only verified by inspection.
- `spike-pin` decodes the PNG fully into memory at startup; no progressive
  loading, which is fine for a spike (pins are screenshots).
- `gpui-pre` re-publishes weekly and snapshots can break APIs; pin the exact
  `0.3.9` until the port starts, then bump deliberately.

## How to run on Windows

Download the `gpui-spike-windows` artifact from the latest workflow run on this
branch (Actions → "Spike Windows build" → run for your commit → Artifacts) and
unzip it. Run `spike-overlay.exe`: the screen dims across every monitor, drag a
blue rectangle, and mouse-up prints the selected rect (`selection: x=… y=… w=…
h=…`) and exits — Escape cancels instead. Run `spike-pin.exe C:\path\to\image.png`
to open a frameless always-on-top window showing the PNG; drag it to move, press
`+`/`-` for opacity in 10% steps, press `t` to toggle full click-through (mouse
and keyboard pass through the window entirely), and Escape or `q` to quit. If
the pin window goes blank after `t` on an NVIDIA machine, set
`GPUI_DISABLE_DIRECT_COMPOSITION=1` and rerun, then note it in this file.
