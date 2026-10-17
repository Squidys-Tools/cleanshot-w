//! Spike B — pin window.
//!
//! A frameless, always-on-top window that displays the PNG passed on the
//! command line. Drag the image to move the window, +/- change opacity,
//! `t` toggles full click-through, Escape quits.
//!
//! Run: `spike-pin path/to/image.png`

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    App, Bounds, Context, KeyBinding, MouseButton, MouseDownEvent, Render, RenderImage, Window,
    WindowBounds, WindowKind, WindowOptions, actions, div, img, prelude::*, px, size,
};
use gpui_overlay_spike::{begin_drag, set_click_through, virtual_screen_bounds};
use gpui_platform::application;

actions!(
    pin,
    [OpacityUp, OpacityDown, ToggleClickThrough, QuitPinned]
);

struct PinWindow {
    focus: gpui::FocusHandle,
    image: Arc<RenderImage>,
    opacity: f32,
    click_through: bool,
}

impl Render for PinWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("pin")
            .size_full()
            .opacity(self.opacity)
            .key_context("PinWindow")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &QuitPinned, _, cx| cx.quit()))
            .on_action(cx.listener(|this, _: &OpacityUp, _, cx| {
                this.opacity = (this.opacity + 0.1).min(1.0);
                println!("opacity: {:.0}%", this.opacity * 100.0);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &OpacityDown, _, cx| {
                this.opacity = (this.opacity - 0.1).max(0.1);
                println!("opacity: {:.0}%", this.opacity * 100.0);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToggleClickThrough, window, cx| {
                this.click_through = !this.click_through;
                let applied = set_click_through(window, this.click_through);
                println!(
                    "click-through: {} (applied: {}) — mouse now passes through; \
                     keyboard stays with this window until you click another one, \
                     which is what lets you press t again to undo",
                    this.click_through, applied
                );
                let _ = cx;
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, window, _| {
                    // Hand the press to the OS as a titlebar drag: a modal
                    // loop on X11, WM_NCLBUTTONDOWN/HTCAPTION on Windows.
                    begin_drag(window);
                }),
            )
            .child(
                img(self.image.clone())
                    .size_full()
                    .object_fit(gpui::ObjectFit::Contain),
            )
    }
}

fn main() {
    let png_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: spike-pin <image.png>");
        std::process::exit(2);
    });

    application().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("+", OpacityUp, None),
            KeyBinding::new("=", OpacityUp, None),
            KeyBinding::new("-", OpacityDown, None),
            KeyBinding::new("t", ToggleClickThrough, None),
            KeyBinding::new("escape", QuitPinned, None),
            KeyBinding::new("q", QuitPinned, None),
        ]);

        let (render_image, pixel_size) = match load_png(&png_path) {
            Ok(ok) => ok,
            Err(error) => {
                eprintln!("spike-pin: {error}");
                cx.quit();
                return;
            }
        };

        // Cap the initial window size; keep the image aspect ratio.
        let max = size(px(800.), px(600.));
        let scale = (max.width / pixel_size.width)
            .min(max.height / pixel_size.height)
            .min(1.0);
        let window_size = size(pixel_size.width * scale, pixel_size.height * scale);

        let screen = virtual_screen_bounds(cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: screen.origin + gpui::point(px(120.), px(120.)),
                size: window_size,
            })),
            titlebar: None,
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

        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| PinWindow {
                focus: cx.focus_handle(),
                image: render_image.clone(),
                opacity: 1.0,
                click_through: false,
            });
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            view
        })
        .unwrap();

        println!(
            "pin window up: drag to move, +/- opacity, t click-through, Esc quit ({png_path})"
        );
        cx.activate(true);
    });
}

fn load_png(path: &str) -> Result<(Arc<RenderImage>, gpui::Size<gpui::Pixels>), String> {
    let path = PathBuf::from(path);
    let mut rgba = image::ImageReader::open(&path)
        .map_err(|e| format!("could not open {}: {e}", path.display()))?
        .decode()
        .map_err(|e| format!("could not decode {}: {e}", path.display()))?
        .into_rgba8();
    let (width, height) = rgba.dimensions();

    // `RenderImage` buffers are BGRA (the doc on the struct says so, and
    // gpui's own PNG/GIF/WebP decode paths swizzle RGBA→BGRA in
    // `gpui/src/elements/img.rs` before constructing one). The Windows
    // renderer uploads them into a `DXGI_FORMAT_B8G8R8A8_UNORM` texture
    // (`directx_atlas.rs`), so handing it raw RGBA bytes swaps red and blue.
    for [r, _, b, _] in rgba.as_chunks_mut::<4>().0 {
        std::mem::swap(r, b);
    }

    let frame = image::Frame::new(rgba);
    let render_image = Arc::new(RenderImage::new(vec![frame]));
    Ok((render_image, size(px(width as f32), px(height as f32))))
}
