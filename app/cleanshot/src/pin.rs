//! Pin window, ported from the spike (`spikes/gpui-overlay/src/bin/pin.rs`).
//!
//! A frameless, always-on-top window showing a capture. Drag to move, `+`/`-`
//! opacity, `t` click-through, `ctrl+c` copy as image, `ctrl+s` save as PNG
//! via a file dialog, Escape closes.
//!
//! Color: PNG bytes are decoded to RGBA and swizzled to BGRA before
//! `RenderImage::new` — the renderer's atlas is `DXGI_FORMAT_B8G8R8A8_UNORM`
//! on Windows and uploads frame bytes verbatim (see `platform::rgba_to_bgra`).

use std::sync::Arc;

use gpui::{
    App, Bounds, Context, KeyBinding, MouseButton, MouseDownEvent, Render, RenderImage, Window,
    WindowBounds, WindowKind, WindowOptions, actions, div, img, prelude::*, px, size,
};

use crate::{clipboard, platform, save_dialog};

actions!(
    pin,
    [
        OpacityUp,
        OpacityDown,
        ToggleClickThrough,
        ClosePin,
        CopyPinImage,
        SavePinImage
    ]
);

pub struct PinWindow {
    focus: gpui::FocusHandle,
    image: Arc<RenderImage>,
    png: Vec<u8>,
    title: String,
    opacity: f32,
    click_through: bool,
}

/// Decode PNG bytes into a render-ready (BGRA) image.
pub fn decode_to_render_image(png: &[u8]) -> Result<Arc<RenderImage>, String> {
    let mut rgba = image::ImageReader::new(std::io::Cursor::new(png))
        .with_guessed_format()
        .map_err(|error| format!("Could not open the capture image: {error}"))?
        .decode()
        .map_err(|error| format!("Could not decode the capture image: {error}"))?
        .into_rgba8();
    platform::rgba_to_bgra(rgba.as_mut());
    let frame = image::Frame::new(rgba);
    Ok(Arc::new(RenderImage::new(vec![frame])))
}

/// Open a pin window for a freshly captured PNG. `physical_size` is the
/// capture's size in physical pixels; the window is sized to fit on screen
/// using the overlay's scale factor.
pub fn open_for_capture(
    cx: &mut App,
    png: Vec<u8>,
    title: &str,
    physical_width: u32,
    physical_height: u32,
    scale: f32,
    virtual_origin: gpui::Point<gpui::Pixels>,
) -> Result<(), String> {
    let image = decode_to_render_image(&png)?;
    let logical_width = physical_width as f32 / scale;
    let logical_height = physical_height as f32 / scale;

    // Cap the initial window size; keep the image aspect ratio.
    let max = size(px(800.), px(600.));
    let fit = (max.width / px(logical_width))
        .min(max.height / px(logical_height))
        .min(1.0);
    let window_size = size(px(logical_width * fit), px(logical_height * fit));

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: virtual_origin + gpui::point(px(120.), px(120.)),
            size: window_size,
        })),
        titlebar: None,
        app_id: Some("com.snipsy.desktop".to_string()),
        focus: true,
        show: true,
        // Same shape as the overlay: frameless topmost tool window on
        // Windows; a normal WM-managed window on the Linux dev loop.
        #[cfg(windows)]
        kind: WindowKind::PopUp,
        #[cfg(not(windows))]
        kind: WindowKind::Normal,
        is_movable: false,
        is_resizable: false,
        ..Default::default()
    };

    let window_title = format!("Snipsy — {title}");
    cx.open_window(options, move |window, cx| {
        window.set_window_title(&window_title);
        let view = cx.new(|cx| PinWindow {
            focus: cx.focus_handle(),
            image,
            png,
            title: title.to_string(),
            opacity: 1.0,
            click_through: false,
        });
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        view
    })
    .map_err(|error| format!("Could not open the pin window: {error}"))?;
    Ok(())
}

impl Render for PinWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("pin")
            .size_full()
            .opacity(self.opacity)
            .key_context("PinWindow")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &ClosePin, window, _| window.remove_window()))
            .on_action(cx.listener(|this, _: &OpacityUp, _, cx| {
                this.opacity = (this.opacity + 0.1).min(1.0);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &OpacityDown, _, cx| {
                this.opacity = (this.opacity - 0.1).max(0.1);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleClickThrough, window, _| {
                this.click_through = !this.click_through;
                platform::set_click_through(window, this.click_through);
            }))
            .on_action(cx.listener(|this, _: &CopyPinImage, _, _| {
                if let Err(error) = clipboard::copy_png_image(&this.png) {
                    eprintln!("Snipsy: {error}");
                }
            }))
            .on_action(cx.listener(|this, _: &SavePinImage, _, _| {
                if let Err(error) = save_dialog::save_png_via_dialog(&this.png, &this.title) {
                    eprintln!("Snipsy: {error}");
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, window, _| {
                    // Hand the press to the OS as a titlebar drag: a modal
                    // loop on X11, WM_NCLBUTTONDOWN/HTCAPTION on Windows.
                    platform::begin_drag(window);
                }),
            )
            .child(
                img(self.image.clone())
                    .size_full()
                    .object_fit(gpui::ObjectFit::Contain),
            )
    }
}

/// Keystroke bindings for pin windows.
pub fn keybindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("+", OpacityUp, None),
        KeyBinding::new("=", OpacityUp, None),
        KeyBinding::new("-", OpacityDown, None),
        KeyBinding::new("t", ToggleClickThrough, None),
        KeyBinding::new("ctrl-c", CopyPinImage, None),
        KeyBinding::new("ctrl-s", SavePinImage, None),
        KeyBinding::new("escape", ClosePin, None),
    ]
}
