//! Platform helpers shared by the GPUI windows: virtual-screen geometry,
//! window-native drag and click-through, and the RGBA→BGRA swizzle the
//! renderer's image upload path requires.
//!
//! Ported from the spike (`spikes/gpui-overlay/src/lib.rs`); Windows-only raw
//! Win32 fallbacks live behind `cfg(windows)` so the Linux x11 dev loop keeps
//! compiling.

use gpui::{App, Bounds, Pixels, Point, Window, px};

pub fn make_bounds(x: i32, y: i32, w: i32, h: i32) -> Bounds<Pixels> {
    Bounds {
        origin: gpui::point(px(x as f32), px(y as f32)),
        size: gpui::Size {
            width: px(w as f32),
            height: px(h as f32),
        },
    }
}

/// The union of every connected display's bounds, in GPUI logical pixels —
/// the coordinate space mouse events and `Window::bounds()` speak. This is
/// the right geometry for *windows*; GDI capture works in physical pixels
/// (see `capture`).
///
/// Unlike the spike, this deliberately does NOT union raw Win32 virtual-screen
/// metrics in: those are physical pixels and mixing the two units is exactly
/// the bug class this port has to avoid.
pub fn gpui_virtual_screen_bounds(cx: &App) -> Bounds<Pixels> {
    let mut origin_x = i32::MAX;
    let mut origin_y = i32::MAX;
    let mut max_right = i32::MIN;
    let mut max_bottom = i32::MIN;

    let displays = cx.displays();
    for display in &displays {
        let bounds = display.bounds();
        let (x, y, w, h) = (
            f32::from(bounds.origin.x) as i32,
            f32::from(bounds.origin.y) as i32,
            f32::from(bounds.size.width) as i32,
            f32::from(bounds.size.height) as i32,
        );
        origin_x = origin_x.min(x);
        origin_y = origin_y.min(y);
        max_right = max_right.max(x + w);
        max_bottom = max_bottom.max(y + h);
    }

    if origin_x == i32::MAX || max_right <= origin_x {
        return make_bounds(0, 0, 1920, 1080);
    }

    make_bounds(
        origin_x,
        origin_y,
        max_right - origin_x,
        max_bottom - origin_y,
    )
}

/// Toggle mouse pass-through on the window.
///
/// GPUI has no surface for this; it needs raw window styles: WS_EX_TRANSPARENT
/// (+ WS_EX_LAYERED so DWM honors it) on Windows, an empty X11 input shape on
/// Linux x11.
pub fn set_click_through(window: &Window, enabled: bool) -> bool {
    #[cfg(windows)]
    {
        win32::hwnd_of(window)
            .map(|hwnd| win32::set_click_through(hwnd, enabled))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        x11_click_through(window, enabled)
    }
}

/// Hand the window over to the OS for a modal drag loop: WM_NCLBUTTONDOWN /
/// HTCAPTION on Windows, the compositor equivalent (`start_window_move`) on
/// Linux.
pub fn begin_drag(window: &Window) {
    #[cfg(windows)]
    {
        if let Some(hwnd) = win32::hwnd_of(window) {
            win32::begin_os_drag(hwnd);
        }
    }
    #[cfg(not(windows))]
    {
        window.start_window_move();
    }
}

/// Convert a selection rect from GPUI logical global coordinates into the
/// physical Win32 coordinates a GDI crop needs.
///
/// GPUI logical px map to physical px as `physical = logical × scale`, where
/// `scale` is the window's DPI scale factor (`GetDpiForWindow/96`, e.g. 1.5 at
/// 150% scaling). Both coordinate spaces share one origin only when the
/// virtual screen starts at (0,0) — in general we translate through the
/// virtual-screen origin of each space first.
pub fn selection_to_physical(
    selection: Bounds<Pixels>,
    gpui_virtual_origin: Point<Pixels>,
    win32_virtual_origin: (i32, i32),
    scale: f32,
) -> (i32, i32, u32, u32) {
    let rel_left = f32::from(selection.origin.x) - f32::from(gpui_virtual_origin.x);
    let rel_top = f32::from(selection.origin.y) - f32::from(gpui_virtual_origin.y);
    let x = win32_virtual_origin.0 + (rel_left * scale).round() as i32;
    let y = win32_virtual_origin.1 + (rel_top * scale).round() as i32;
    let w = ((f32::from(selection.size.width)) * scale).round().max(1.0) as u32;
    let h = ((f32::from(selection.size.height)) * scale)
        .round()
        .max(1.0) as u32;
    (x, y, w, h)
}

/// The DPI scale factor of the primary display, for windows opened outside
/// the overlay flow (tray "Open", `--pin`), where no window exists yet to ask
/// for its `scale_factor`. Windows: system DPI (primary monitor), which is
/// exact for the single-display case phase 1 targets; mixed-DPI multi-monitor
/// is a documented extension point.
pub fn primary_scale_factor() -> f32 {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
        unsafe { GetDpiForSystem() as f32 / 96.0 }
    }
    #[cfg(not(windows))]
    {
        1.0
    }
}

/// Swap RGBA byte order to BGRA in place.
///
/// `RenderImage` is documented as BGRA ("A cached and processed image, in
/// BGRA format", gpui assets.rs) and the Windows atlas uploads the bytes
/// verbatim into a `DXGI_FORMAT_B8G8R8A8_UNORM` texture; GPUI's own decode
/// path does this same swap (`platform.rs decode_static_image`). The spike
/// skipped it and orange rendered blue on hardware — never feed RGBA here.
pub fn rgba_to_bgra(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
}

#[cfg(windows)]
pub mod win32 {
    use gpui::Window;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging as winmsg;

    pub fn hwnd_of(window: &Window) -> Option<HWND> {
        let raw = HasWindowHandle::window_handle(window).ok()?.as_raw();
        match raw {
            RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as HWND),
            _ => None,
        }
    }

    pub fn set_visible(window: &Window, visible: bool) -> bool {
        let Some(hwnd) = hwnd_of(window) else {
            return false;
        };
        unsafe {
            winmsg::ShowWindow(
                hwnd,
                if visible {
                    winmsg::SW_SHOWNOACTIVATE
                } else {
                    winmsg::SW_HIDE
                },
            ) != 0
        }
    }

    /// WS_EX_TRANSPARENT + WS_EX_LAYERED makes the window invisible to hit
    /// testing. Note: pairing WS_EX_LAYERED with a DirectX swapchain window is
    /// the historically fragile part on NVIDIA + DirectComposition (zed issue
    /// #55654); if the pin window's content vanishes on toggle, that is the
    /// first suspect.
    pub fn set_click_through(hwnd: HWND, enabled: bool) -> bool {
        unsafe {
            let current = winmsg::GetWindowLongPtrW(hwnd, winmsg::GWL_EXSTYLE) as u32;
            let next = if enabled {
                current | winmsg::WS_EX_TRANSPARENT | winmsg::WS_EX_LAYERED
            } else {
                current & !winmsg::WS_EX_TRANSPARENT & !winmsg::WS_EX_LAYERED
            };
            winmsg::SetWindowLongPtrW(hwnd, winmsg::GWL_EXSTYLE, next as isize);
            next != current
        }
    }

    /// The classic frameless-window drag: release mouse capture, then ask the
    /// non-client handler to treat the press as a titlebar drag.
    pub fn begin_os_drag(hwnd: HWND) {
        use windows_sys::Win32::Foundation::WPARAM;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
        unsafe {
            ReleaseCapture();
            winmsg::PostMessageW(
                hwnd,
                winmsg::WM_NCLBUTTONDOWN,
                winmsg::HTCAPTION as WPARAM,
                0,
            );
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn x11_click_through(window: &Window, enabled: bool) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };
    let xid = match handle.as_raw() {
        RawWindowHandle::Xcb(handle) => handle.window.get(),
        _ => return false,
    };

    let Ok((conn, _screen)) = x11rb::connect(None) else {
        return false;
    };
    use x11rb::protocol::shape::{ConnectionExt as _, SK, SO};
    use x11rb::protocol::xproto::{ClipOrdering, ConnectionExt as XProtoExt, Rectangle};

    if enabled {
        // An empty input region makes every pixel pass clicks through.
        conn.shape_rectangles(SO::SET, SK::INPUT, ClipOrdering::UNSORTED, xid, 0, 0, &[])
            .ok()
            // check() flushes the buffered request and inspects the server's
            // verdict; without it the queued change can be lost when the
            // connection drops.
            .and_then(|cookie| cookie.check().ok())
            .is_some()
    } else {
        // Restore the input region to the full window geometry.
        let geometry = match conn.get_geometry(xid) {
            Ok(cookie) => match cookie.reply() {
                Ok(reply) => reply,
                Err(_) => return false,
            },
            Err(_) => return false,
        };
        conn.shape_rectangles(
            SO::SET,
            SK::INPUT,
            ClipOrdering::UNSORTED,
            xid,
            0,
            0,
            &[Rectangle {
                x: 0,
                y: 0,
                width: geometry.width,
                height: geometry.height,
            }],
        )
        .ok()
        .and_then(|cookie| cookie.check().ok())
        .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{rgba_to_bgra, selection_to_physical};
    use gpui::{Bounds, point, px, size};

    fn selection(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui::Pixels> {
        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        }
    }

    #[test]
    fn rgba_to_bgra_swaps_only_red_and_blue() {
        let mut pixels = vec![1, 2, 3, 4, 5, 6, 7, 8];
        rgba_to_bgra(&mut pixels);
        assert_eq!(pixels, vec![3, 2, 1, 4, 7, 6, 5, 8]);
    }

    #[test]
    fn selection_converts_logical_to_physical_at_150_percent() {
        // Single display at 150% scaling: both origins are (0,0); the spike
        // hardware case (1920×1200 physical = 1280×800 logical).
        let (x, y, w, h) = selection_to_physical(
            selection(220.0, 261.0, 826.0, 248.0),
            point(px(0.), px(0.)),
            (0, 0),
            1.5,
        );
        assert_eq!((x, y, w, h), (330, 392, 1239, 372));
    }

    #[test]
    fn selection_converts_through_both_virtual_screen_origins() {
        // A virtual screen whose Win32 origin is negative (monitors left of
        // the primary) and whose GPUI logical origin differs.
        let (x, y, w, h) = selection_to_physical(
            selection(60.0, 40.0, 100.0, 50.0),
            point(px(-50.), px(-25.)),
            (-100, -75),
            1.5,
        );
        // x = -100 + (60 - (-50)) * 1.5 = 65; y = -75 + (40 - (-25)) * 1.5 = 22.5 → 23
        assert_eq!((x, y, w, h), (65, 23, 150, 75));
    }

    #[test]
    fn sub_pixel_selections_round_to_at_least_one_physical_pixel() {
        let (x, y, w, h) = selection_to_physical(
            selection(0.2, 0.2, 0.1, 0.1),
            point(px(0.), px(0.)),
            (0, 0),
            1.5,
        );
        assert_eq!((x, y), (0, 0));
        assert_eq!((w, h), (1, 1));
    }
}
