//! Minimal native window picker for PrintWindow captures.

use gpui::{
    App, Context, KeyBinding, MouseButton, MouseUpEvent, Render, Window, WindowBounds, WindowKind,
    WindowOptions, actions, div, prelude::*, px, rgb, size,
};

use crate::{capture::WindowInfo, flow, platform};

actions!(window_picker, [CloseWindowPicker]);

pub struct WindowPicker {
    focus: gpui::FocusHandle,
    windows: Vec<WindowInfo>,
}

pub fn open(cx: &mut App, windows: Vec<WindowInfo>) -> Result<(), String> {
    let screen = platform::gpui_virtual_screen_bounds(cx);
    let window_size = size(px(520.), px(620.));
    let bounds = gpui::Bounds {
        origin: gpui::point(
            screen.origin.x + (screen.size.width - window_size.width) / 2.,
            screen.origin.y + (screen.size.height - window_size.height) / 2.,
        ),
        size: window_size,
    };
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        window_decorations: None,
        is_resizable: true,
        ..Default::default()
    };

    cx.open_window(options, |window, cx| {
        let view = cx.new(|cx| WindowPicker {
            focus: cx.focus_handle(),
            windows,
        });
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        view
    })
    .map_err(|error| format!("Could not open the window picker: {error}"))?;
    Ok(())
}

impl Render for WindowPicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("window-picker")
            .size_full()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(16.))
            .bg(rgb(0x18181B))
            .text_color(rgb(0xF4F4F5))
            .key_context("WindowPicker")
            .track_focus(&self.focus)
            .on_action(cx.listener(|_, _: &CloseWindowPicker, window, _| {
                window.remove_window();
            }))
            .child(div().text_xl().child("Capture a window"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0xA1A1AA))
                    .child("Select a visible window. The capture is rendered without hiding it."),
            )
            .child(if self.windows.is_empty() {
                div()
                    .id("window-picker-empty")
                    .child("No titled visible windows were found.")
            } else {
                div()
                    .id("window-picker-list")
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .max_h(px(460.))
                    .overflow_y_scroll()
                    .children(self.windows.iter().cloned().map(|entry| {
                        let title = entry.title.clone();
                        div()
                            .id(format!("window-{}", entry.id))
                            .w_full()
                            .rounded(px(6.))
                            .p(px(10.))
                            .bg(rgb(0x27272A))
                            .hover(|style| style.bg(rgb(0x3F3F46)))
                            .child(div().child(title))
                            .child(div().text_sm().text_color(rgb(0xA1A1AA)).child(format!(
                                "{} × {}  ·  {}, {}",
                                entry.width, entry.height, entry.x, entry.y
                            )))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(move |_, _: &MouseUpEvent, window, cx| {
                                    flow::capture_window(window, cx, entry.id, entry.title.clone());
                                }),
                            )
                    }))
            })
            .child(
                div()
                    .id("window-picker-cancel")
                    .w_full()
                    .p(px(10.))
                    .bg(rgb(0x27272A))
                    .child("Cancel")
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|_, _: &MouseUpEvent, window, _| window.remove_window()),
                    ),
            )
    }
}

pub fn keybindings() -> Vec<KeyBinding> {
    vec![KeyBinding::new("escape", CloseWindowPicker, None)]
}
