//! Shared code for the GPUI Windows spikes.
//!
//! Everything here must compile on Linux x11 (the dev loop); Windows-only raw
//! Win32 fallbacks live behind `cfg(windows)`.

use gpui::{App, Bounds, Pixels, Size, Window, px};

pub fn make_bounds(x: i32, y: i32, w: i32, h: i32) -> Bounds<Pixels> {
    Bounds {
        origin: gpui::point(px(x as f32), px(y as f32)),
        size: Size {
            width: px(w as f32),
            height: px(h as f32),
        },
    }
}

/// The union of every connected display's bounds, in global screen
/// coordinates — the GPUI equivalent of the virtual-screen metrics
/// `GetSystemMetrics(SM_XVIRTUALSCREEN…)` produces in `src-tauri/src/capture.rs`.
pub fn virtual_screen_bounds(cx: &App) -> Bounds<Pixels> {
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

    // GPUI's display enumeration can disagree with the raw virtual screen on
    // Windows (DPI-mixed arrangements, hidden monitors). Cross-check against
    // raw Win32 and prefer whichever spans more, mirroring capture.rs.
    #[cfg(windows)]
    {
        let (rx, ry, rw, rh) = win32::raw_virtual_screen_metrics();
        if rw > max_right - origin_x || rh > max_bottom - origin_y {
            origin_x = origin_x.min(rx);
            origin_y = origin_y.min(ry);
            max_right = max_right.max(rx + rw);
            max_bottom = max_bottom.max(ry + rh);
        }
    }

    if origin_x == i32::MAX || max_right <= origin_x {
        // No displays found; fall back to a sane default so the spike still runs.
        return make_bounds(0, 0, 1920, 1080);
    }

    make_bounds(
        origin_x,
        origin_y,
        max_right - origin_x,
        max_bottom - origin_y,
    )
}

/// Toggle full mouse/keyboard pass-through on the window.
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

    /// EnumDisplayMonitors equivalent. Returns (x, y, width, height) of the
    /// virtual screen, mirroring `src-tauri/src/capture.rs`.
    pub fn raw_virtual_screen_metrics() -> (i32, i32, i32, i32) {
        unsafe {
            (
                winmsg::GetSystemMetrics(winmsg::SM_XVIRTUALSCREEN),
                winmsg::GetSystemMetrics(winmsg::SM_YVIRTUALSCREEN),
                winmsg::GetSystemMetrics(winmsg::SM_CXVIRTUALSCREEN),
                winmsg::GetSystemMetrics(winmsg::SM_CYVIRTUALSCREEN),
            )
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
            .is_ok()
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
        .is_ok()
    }
}
