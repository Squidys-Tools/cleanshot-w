//! Tray icon using the `tray-icon` crate directly (what Tauri's tray plugin
//! wraps). Windows-only for now: the Linux backend wants GTK/libappindicator
//! which the GPUI dev loop does not run; the Linux loop uses `--capture` /
//! `--pin` instead. Menu events arrive on a crossbeam channel the app polls
//! from an async task.

use crossbeam_channel::Receiver;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub const NEW_CAPTURE_ID: &str = "new-capture";
pub const OPEN_LAST_ID: &str = "open-last";
pub const QUIT_ID: &str = "quit";
const APP_NAME: &str = "Snipsy";

pub struct Tray {
    // The tray icon is removed when dropped, so the caller must keep this
    // alive for the app's lifetime.
    _icon: TrayIcon,
}

pub fn create() -> Result<Tray, String> {
    let brand = MenuItem::with_id("snipsy-title", APP_NAME, false, None);
    let new_capture = MenuItem::with_id(NEW_CAPTURE_ID, "New capture", true, None);
    let open_last = MenuItem::with_id(OPEN_LAST_ID, "Open Snipsy", true, None);
    let quit = MenuItem::with_id(QUIT_ID, "Quit", true, None);
    let separator = PredefinedMenuItem::separator();
    let menu = Menu::new();
    menu.append_items(&[&brand, &separator, &new_capture, &open_last, &quit])
        .map_err(|error| format!("Could not build the tray menu: {error}"))?;

    let icon = tray_icon::Icon::from_rgba(app_icon_rgba(32, 32), 32, 32)
        .map_err(|error| format!("Could not build the tray icon: {error}"))?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(APP_NAME)
        .with_icon(icon)
        .build()
        .map_err(|error| format!("Could not create the tray icon: {error}"))?;

    Ok(Tray { _icon: tray })
}

pub fn event_receiver() -> &'static Receiver<MenuEvent> {
    MenuEvent::receiver()
}

/// A simple procedural icon: blue square with a white border. The real app
/// icon can replace this when there is one.
fn app_icon_rgba(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let border_x = x < 2 || x >= width - 2;
            let border_y = y < 2 || y >= height - 2;
            if border_x || border_y {
                rgba.extend_from_slice(&[255, 255, 255, 255]);
            } else {
                rgba.extend_from_slice(&[59, 130, 246, 255]);
            }
        }
    }
    rgba
}
