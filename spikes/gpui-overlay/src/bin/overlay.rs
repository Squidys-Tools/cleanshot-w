//! Spike A — full-virtual-screen capture overlay.
//!
//! Opens a borderless, transparent, always-on-top window spanning every
//! monitor and lets you drag a rubber-band selection rectangle with the mouse.
//! Mouse-up logs the selected rect (in global screen coordinates) and quits;
//! Escape cancels and quits.
//!
//! Run: `spike-overlay`

use gpui::{
    App, Bounds, Context, CursorStyle, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PathBuilder, Point, Render, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions, actions, canvas, div, prelude::*, px, rgb, size,
};
use gpui_overlay_spike::virtual_screen_bounds;
use gpui_platform::application;

actions!(spike, [CancelSelection]);

struct Overlay {
    focus: gpui::FocusHandle,
    drag_start: Option<Point<gpui::Pixels>>,
    drag_current: Option<Point<gpui::Pixels>>,
}

impl Overlay {
    fn selection_bounds(&self) -> Option<Bounds<gpui::Pixels>> {
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

impl Render for Overlay {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = self.selection_bounds();

        div()
            .id("overlay")
            .size_full()
            .cursor(CursorStyle::Crosshair)
            .key_context("Overlay")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CancelSelection, _, cx| {
                println!("cancelled");
                cx.quit();
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
                    if let Some(start) = this.drag_start {
                        let end = ev.position;
                        // ev.position is window-relative, in logical px. Add
                        // the window's global logical origin (the virtual
                        // screen origin, which can be negative on
                        // multi-monitor setups) to get global logical
                        // coordinates. GDI capture works in physical px. The
                        // window scale factor gives a correct conversion only
                        // for a uniform-DPI screen; mixed-DPI mapping is not
                        // implemented by this spike (see SPIKE-NOTES.md).
                        let origin = window.bounds().origin;
                        let scale = window.scale_factor();
                        let (x, y) = (origin.x + start.x.min(end.x), origin.y + start.y.min(end.y));
                        let (w, h) = ((start.x - end.x).abs(), (start.y - end.y).abs());
                        println!(
                            "selection (logical): x={} y={} w={} h={}",
                            f32::from(x) as i32,
                            f32::from(y) as i32,
                            f32::from(w) as i32,
                            f32::from(h) as i32
                        );
                        println!(
                            "selection (uniform-DPI physical estimate, x{scale}): x={} y={} w={} h={}",
                            (f32::from(x) * scale) as i32,
                            (f32::from(y) * scale) as i32,
                            (f32::from(w) * scale) as i32,
                            (f32::from(h) * scale) as i32
                        );
                    }
                    cx.quit();
                }),
            )
    }
}

fn main() {
    application().run(|cx: &mut App| {
        cx.bind_keys([KeyBinding::new("escape", CancelSelection, None)]);

        let screen = virtual_screen_bounds(cx);
        println!("virtual screen (logical): {screen:?}");
        for display in cx.displays() {
            println!("display {:#?}: {:#?}", display.id(), display.bounds());
        }
        #[cfg(windows)]
        println!(
            "virtual screen (physical, GetSystemMetrics): {:?}",
            gpui_overlay_spike::win32::raw_virtual_screen_metrics()
        );

        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(screen)),
            titlebar: None,
            window_background: WindowBackgroundAppearance::Transparent,
            focus: true,
            show: true,
            // Windows: WS_EX_TOOLWINDOW | WS_EX_TOPMOST, style 0 — frameless
            // and always-on-top, exactly what the capture overlay needs.
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
            let view = cx.new(|cx| Overlay {
                focus: cx.focus_handle(),
                drag_start: None,
                drag_current: None,
            });
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            view
        })
        .unwrap();

        cx.activate(true);
    });
}
