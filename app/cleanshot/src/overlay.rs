//! Capture overlay, ported from the spike (`spikes/gpui-overlay/src/bin/overlay.rs`).
//!
//! A full-virtual-screen, borderless, transparent, always-on-top window with
//! rubber-band selection. Mouse-up hands the selection (GPUI logical global
//! coordinates) to `flow::complete_selection`, which converts it to physical
//! pixels for the GDI crop; Escape cancels.

use gpui::{
    Bounds, Context, CursorStyle, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PathBuilder, Pixels, Point, Render, Window, WindowBackgroundAppearance,
    WindowBounds, WindowId, WindowKind, WindowOptions, actions, canvas, div, prelude::*, px, size,
};

use crate::flow;
use crate::theme::Theme;

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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = self.selection_bounds();
        let theme_accent = Theme::current(cx).accent;
        let dims_badge = selection.map(|sel| {
            let scale = window.scale_factor();
            let w = (f32::from(sel.size.width) * scale).round() as i64;
            let h = (f32::from(sel.size.height) * scale).round() as i64;
            div()
                .absolute()
                .left(sel.origin.x + sel.size.width / 2.0)
                .top(sel.bottom() + px(12.0))
                .px_2()
                .py(px(2.0))
                .rounded(px(6.0))
                .bg(Theme::current(cx).bg_toolbar)
                .text_size(px(11.0))
                .text_color(Theme::current(cx).text_1)
                .font_family(crate::ui::font_family())
                .child(format!("{w} × {h}"))
        });

        div()
            .id("overlay")
            .size_full()
            .cursor(CursorStyle::Crosshair)
            .key_context("Overlay")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CancelSelection, window, cx| {
                flow::cancel_capture(window, cx);
            }))
            // No root scrim: the canvas paints four quads around the
            // selection so its interior shows the live, un-dimmed screen.
            .children(dims_badge)
            .child(
                canvas(
                    move |_, _, _| theme_accent,
                    move |bounds, accent, window, _| {
                        let Some(sel) = selection else {
                            // No selection yet: dim the whole screen.
                            paint_scrim(window, bounds, gpui::black().alpha(0.45));
                            return;
                        };

                        // Dim everything except the selection so its interior
                        // shows the live, un-dimmed screen through the
                        // transparent window.
                        for side in scrim_sides(bounds, &sel) {
                            paint_scrim(window, side, gpui::black().alpha(0.45));
                        }

                        // 4 px accent glow behind the 1 px accent frame.
                        if let Some(path) = rect_outline(sel, px(4.0)) {
                            window.paint_path(path, accent.alpha(0.35));
                        }
                        if let Some(path) = rect_outline(sel, px(1.0)) {
                            window.paint_path(path, accent);
                        }

                        // Corner handles: 8x8, white with an accent border.
                        for corner in handle_points(&sel) {
                            let handle_bounds = Bounds {
                                origin: gpui::point(corner.x - px(4.0), corner.y - px(4.0)),
                                size: size(px(8.0), px(8.0)),
                            };
                            window.paint_quad(gpui::quad(
                                handle_bounds,
                                px(1.0),
                                gpui::white(),
                                px(1.0),
                                accent,
                                Default::default(),
                            ));
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

/// The four screen regions around the selection, for the see-through scrim.
fn scrim_sides(full: Bounds<Pixels>, sel: &Bounds<Pixels>) -> Vec<Bounds<Pixels>> {
    vec![
        Bounds {
            origin: full.origin,
            size: size(full.size.width, sel.origin.y - full.origin.y),
        },
        Bounds {
            origin: gpui::point(full.origin.x, sel.bottom()),
            size: size(full.size.width, full.bottom() - sel.bottom()),
        },
        Bounds {
            origin: gpui::point(full.origin.x, sel.origin.y),
            size: size(sel.origin.x - full.origin.x, sel.size.height),
        },
        Bounds {
            origin: sel.top_right(),
            size: size(full.right() - sel.right(), sel.size.height),
        },
    ]
}

fn paint_scrim(window: &mut Window, region: Bounds<Pixels>, color: gpui::Hsla) {
    if f32::from(region.size.width) <= 0.0 || f32::from(region.size.height) <= 0.0 {
        return;
    }
    window.paint_quad(gpui::quad(
        region,
        px(0.),
        color,
        px(0.),
        gpui::transparent_black(),
        Default::default(),
    ));
}

/// A closed rectangular outline path centered on the given bounds.
fn rect_outline(bounds: Bounds<Pixels>, stroke: Pixels) -> Option<gpui::Path<Pixels>> {
    let mut builder = PathBuilder::stroke(stroke);
    builder.move_to(Point {
        x: bounds.origin.x,
        y: bounds.origin.y,
    });
    builder.line_to(Point {
        x: bounds.right(),
        y: bounds.origin.y,
    });
    builder.line_to(Point {
        x: bounds.right(),
        y: bounds.bottom(),
    });
    builder.line_to(Point {
        x: bounds.origin.x,
        y: bounds.bottom(),
    });
    builder.close();
    builder.build().ok()
}

fn handle_points(sel: &Bounds<Pixels>) -> Vec<Point<Pixels>> {
    vec![
        gpui::point(sel.origin.x, sel.origin.y),
        gpui::point(sel.right(), sel.origin.y),
        gpui::point(sel.origin.x, sel.bottom()),
        gpui::point(sel.right(), sel.bottom()),
    ]
}
