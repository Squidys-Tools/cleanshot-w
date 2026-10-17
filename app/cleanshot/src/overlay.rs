//! Capture overlay, ported from the spike (`spikes/gpui-overlay/src/bin/overlay.rs`).
//!
//! A full-virtual-screen, borderless, transparent, always-on-top window with
//! rubber-band selection. Mouse-up hands the selection (GPUI logical global
//! coordinates) to `flow::complete_selection`, which converts it to physical
//! pixels for the GDI crop; Escape cancels.

use gpui::{
    Bounds, Context, CursorStyle, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PathBuilder, Pixels, Point, Render, Window, WindowBackgroundAppearance,
    WindowBounds, WindowId, WindowKind, WindowOptions, actions, canvas, div, prelude::*, px, rgb,
    size,
};

use crate::flow;

actions!(overlay, [CancelSelection]);

pub struct Overlay {
    focus: gpui::FocusHandle,
    drag_start: Option<Point<Pixels>>,
    drag_current: Option<Point<Pixels>>,
}

impl Overlay {
    fn selection_bounds(&self) -> Option<Bounds<Pixels>> {
        let start = self.drag_start?;
        let end = self.drag_current?;
        Some(Bounds {
            origin: Point {
                x: start.x.min(end.x),
                y: start.y.min(end.y),
            },
            size: size((start.x - end.x).abs(), (start.y - end.y).abs()),
        })
    }
}

/// Open the overlay across the whole virtual screen (logical coordinates).
/// The caller must have already stored the pending capture surface.
pub fn open(cx: &mut gpui::App, screen: Bounds<Pixels>) -> Result<WindowId, String> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(screen)),
        app_id: Some("com.snipsy.desktop".to_string()),
        titlebar: None,
        window_background: WindowBackgroundAppearance::Transparent,
        focus: true,
        show: true,
        // Windows: WS_EX_TOOLWINDOW | WS_EX_TOPMOST, style 0 — frameless and
        // always-on-top, exactly what the capture overlay needs.
        // Linux x11: PopUp is override-redirect, which most WMs never give
        // keyboard focus, so the dev loop uses a normal WM-managed window
        // (decorations off) with client-side shadows instead.
        #[cfg(windows)]
        kind: WindowKind::PopUp,
        #[cfg(not(windows))]
        kind: WindowKind::Normal,
        window_decorations: if cfg!(windows) {
            None
        } else {
            Some(gpui::WindowDecorations::Client)
        },
        is_movable: false,
        is_resizable: false,
        ..Default::default()
    };

    cx.open_window(options, |window, cx| {
        window.set_window_title("Snipsy Capture");
        let view = cx.new(|cx| Overlay {
            focus: cx.focus_handle(),
            drag_start: None,
            drag_current: None,
        });
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        view
    })
    .map(|handle| handle.window_id())
    .map_err(|error| format!("Could not open the capture overlay: {error}"))
}

impl Render for Overlay {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = self.selection_bounds();

        div()
            .id("overlay")
            .size_full()
            .cursor(CursorStyle::Crosshair)
            .key_context("Overlay")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CancelSelection, window, cx| {
                flow::cancel_capture(window, cx);
            }))
            // Dim scrim so the selection area reads clearly against the screen.
            .bg(rgb(0x000000).alpha(0.15))
            .child(
                canvas(
                    move |_, _, _| {},
                    move |_, _, window, _| {
                        let Some(sel) = selection else { return };

                        // Filled interior via a quad.
                        window.paint_quad(gpui::quad(
                            sel,
                            px(0.),
                            rgb(0x3B82F6).alpha(0.08),
                            px(0.),
                            gpui::transparent_black(),
                            Default::default(),
                        ));

                        // Border via a stroked path (PathBuilder + paint_path).
                        let mut builder = PathBuilder::stroke(px(2.));
                        builder.move_to(Point {
                            x: sel.origin.x,
                            y: sel.origin.y,
                        });
                        builder.line_to(Point {
                            x: sel.right(),
                            y: sel.origin.y,
                        });
                        builder.line_to(Point {
                            x: sel.right(),
                            y: sel.bottom(),
                        });
                        builder.line_to(Point {
                            x: sel.origin.x,
                            y: sel.bottom(),
                        });
                        builder.close();
                        if let Ok(path) = builder.build() {
                            window.paint_path(path, rgb(0x3B82F6));
                        }
                    },
                )
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    this.drag_start = Some(ev.position);
                    this.drag_current = Some(ev.position);
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if this.drag_start.is_some() {
                    this.drag_current = Some(ev.position);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseUpEvent, window, cx| {
                    let Some(start) = this.drag_start else {
                        return;
                    };
                    let end = ev.position;
                    if start.x == end.x || start.y == end.y {
                        this.drag_start = None;
                        this.drag_current = None;
                        cx.notify();
                        return;
                    }
                    // ev.position is window-relative; add the window's
                    // global origin to get logical global coordinates.
                    let origin = window.bounds().origin;
                    let selection = Bounds {
                        origin: origin + gpui::point(start.x.min(end.x), start.y.min(end.y)),
                        size: size((start.x - end.x).abs(), (start.y - end.y).abs()),
                    };
                    flow::complete_selection(window, cx, selection);
                }),
            )
    }
}

/// Keystroke bindings for the overlay window.
pub fn keybindings() -> Vec<KeyBinding> {
    vec![KeyBinding::new("escape", CancelSelection, None)]
}
