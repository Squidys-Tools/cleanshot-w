//! The capture flow wiring the pieces together: hotkey/tray → full-screen
//! GDI capture → overlay selection → crop → library → pin window.

use gpui::{App, Bounds, Global, Pixels, Point, Window};

use crate::{capture, library, overlay, pin, platform, settings::current_settings};

#[derive(Default)]
pub struct AppState {
    /// Full-screen surface captured before the overlay opens, waiting for a
    /// selection to crop from it.
    pending: Option<PendingCapture>,
    /// The most recent finished capture, for tray "Open" / copy / save.
    pub last: Option<LastCapture>,
    /// Set while the overlay window is up.
    overlay_open: Option<gpui::WindowId>,
}

#[derive(Clone)]
#[cfg_attr(not(windows), allow(dead_code))] // consumed by the tray "Open" path
pub struct LastCapture {
    pub png: Vec<u8>,
    pub title: String,
}

pub struct PendingCapture {
    surface: capture::CaptureSurface,
    /// Virtual-screen origin in GPUI logical coordinates, for the
    /// logical → physical conversion at selection time.
    gpui_virtual_origin: Point<Pixels>,
}

impl Global for AppState {}

/// Start a capture: snapshot the whole virtual screen first (so the overlay's
/// own scrim never lands in the PNG, mirroring the Tauri app's flow), then
/// open the selection overlay on top.
pub fn start_capture(cx: &mut App) {
    if cx.global::<AppState>().pending.is_some() {
        return;
    }
    let include_cursor = current_settings().include_cursor;
    let surface = match capture::capture_surface(include_cursor) {
        Ok(surface) => surface,
        Err(error) => {
            eprintln!("Snipsy: capture failed: {error}");
            return;
        }
    };
    let bounds = platform::gpui_virtual_screen_bounds(cx);
    cx.global_mut::<AppState>().pending = Some(PendingCapture {
        surface,
        gpui_virtual_origin: bounds.origin,
    });
    match overlay::open(cx, bounds) {
        Ok(window_id) => cx.global_mut::<AppState>().overlay_open = Some(window_id),
        Err(error) => {
            eprintln!("Snipsy: {error}");
            cx.global_mut::<AppState>().pending = None;
        }
    }
}

/// Escape pressed on the overlay: drop the pending capture and close.
pub fn cancel_capture(window: &mut Window, cx: &mut App) {
    cx.global_mut::<AppState>().pending = None;
    cx.global_mut::<AppState>().overlay_open = None;
    window.remove_window();
}

/// Selection finished: convert the logical rect to physical Win32 pixels,
/// crop, save to the library, and pin the result.
pub fn complete_selection(window: &mut Window, cx: &mut App, selection: Bounds<Pixels>) {
    let scale = window.scale_factor();
    let pending = cx.global_mut::<AppState>().pending.take();
    cx.global_mut::<AppState>().overlay_open = None;

    let result = pending.map_or_else(
        || Err("There is no pending capture.".to_string()),
        |pending| {
            let (x, y, w, h) = platform::selection_to_physical(
                selection,
                pending.gpui_virtual_origin,
                (pending.surface.origin_x, pending.surface.origin_y),
                scale,
            );
            let cropped = pending.surface.crop(x, y, w, h)?;
            let png = capture::encode_png(cropped.width, cropped.height, &cropped.rgba)?;
            let title = format!("Capture {}", library::now_millis());
            let saved_title =
                match library::save_capture(&png, cropped.width, cropped.height, &title) {
                    Ok(metadata) => metadata.title,
                    Err(error) => {
                        eprintln!("Snipsy: could not save the capture: {error}");
                        title.clone()
                    }
                };
            cx.global_mut::<AppState>().last = Some(LastCapture {
                png: png.clone(),
                title: saved_title.clone(),
            });
            pin::open_for_capture(
                cx,
                png,
                &saved_title,
                cropped.width,
                cropped.height,
                scale,
                pending.gpui_virtual_origin,
            )
        },
    );
    if let Err(error) = result {
        eprintln!("Snipsy: {error}");
    }
    window.remove_window();
}

/// Tray "Open": re-pin the most recent capture.
#[cfg_attr(not(windows), allow(dead_code))] // dispatched from the tray
pub fn open_last_capture(cx: &mut App) {
    if let Some(last) = cx.global::<AppState>().last.clone() {
        return open_pin_from_png(cx, last.png, &last.title);
    }

    let latest = match library::list_captures() {
        Ok(list) if !list.is_empty() => list.into_iter().next().unwrap(),
        Ok(_) => {
            eprintln!("Snipsy: the capture library is empty");
            return;
        }
        Err(error) => {
            eprintln!("Snipsy: {error}");
            return;
        }
    };
    match library::read_image_png(&latest.id) {
        Ok(png) => open_pin_from_png(cx, png, &latest.title),
        Err(error) => eprintln!("Snipsy: {error}"),
    }
}

/// Linux dev-loop entry: pin a PNG from a path (`cleanshot-w --pin file.png`).
pub fn open_pin_from_path(cx: &mut App, path: &str) {
    match std::fs::read(path) {
        Ok(png) => {
            let title = std::path::Path::new(path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| "Capture".to_string());
            open_pin_from_png(cx, png, &title)
        }
        Err(error) => eprintln!("Snipsy: could not read {path}: {error}"),
    }
}

fn open_pin_from_png(cx: &mut App, png: Vec<u8>, title: &str) {
    let dimensions = match image::ImageReader::new(std::io::Cursor::new(&png))
        .with_guessed_format()
        .map_err(|error| format!("{error}"))
        .and_then(|reader| reader.into_dimensions().map_err(|error| format!("{error}")))
    {
        Ok(dimensions) => dimensions,
        Err(error) => {
            eprintln!("Snipsy: could not decode {title}: {error}");
            return;
        }
    };
    let (width, height) = dimensions;
    let scale = platform::primary_scale_factor();
    if let Err(error) = pin::open_for_capture(
        cx,
        png,
        title,
        width,
        height,
        scale,
        platform::gpui_virtual_screen_bounds(cx).origin,
    ) {
        eprintln!("Snipsy: {error}");
    }
}

/// Tray menu dispatch.
#[cfg_attr(not(windows), allow(dead_code))] // dispatched from the tray
pub fn handle_menu_event(cx: &mut App, id: &str) {
    match id {
        "new-capture" => start_capture(cx),
        "open-last" => open_last_capture(cx),
        "quit" => cx.quit(),
        _ => {}
    }
}
