//! The quick-access card (design-spec §5.4): a small floating card showing
//! the most recent capture — thumbnail from the library store, Copy and Save
//! as prominent actions, annotate / pin / delete / close as icons. It is a
//! real window (not a tray menu item), toggled from the tray menu, the
//! `--quick-access` dev-loop flag, or Escape to dismiss.

use std::sync::Arc;

use gpui::{
    App, Bounds, Context, Corners, Global, KeyBinding, MouseButton, MouseDownEvent, Render,
    RenderImage, Window, WindowBounds, WindowHandle, WindowKind, WindowOptions, actions, canvas,
    div, prelude::*, px, size,
};

use crate::editor::window::EditorSource;
use crate::theme::{R_BTN, R_INPUT, R_PANEL, R_THUMB, Theme};
use crate::{editor, library, pin, platform, save_dialog};

actions!(
    quick_access,
    [
        DismissQuickAccess,
        QuickAccessCopy,
        QuickAccessSave,
        QuickAccessAnnotate,
        QuickAccessPin,
        QuickAccessDelete
    ]
);

const CARD_WIDTH: f32 = 320.0;
const THUMBNAIL_HEIGHT: f32 = 170.0;
const SCREEN_MARGIN: f32 = 26.0;

/// Tracks the card's open window so the tray toggle can close it again.
#[derive(Default)]
pub struct QuickAccessSession {
    window: Option<WindowHandle<QuickAccessCard>>,
}

impl Global for QuickAccessSession {}

/// Toggle the quick-access card for the latest library capture.
pub fn open(cx: &mut App) -> Result<(), String> {
    if !cx.has_global::<QuickAccessSession>() {
        cx.set_global(QuickAccessSession::default());
    }
    if let Some(handle) = cx.global::<QuickAccessSession>().window {
        // Already open: the tray item toggles it closed.
        cx.global_mut::<QuickAccessSession>().window = None;
        let _ = handle.update(cx, |_, window, _| window.remove_window());
        return Ok(());
    }

    let latest = library::list_captures()?.into_iter().next();
    let content = match latest {
        Some(metadata) => {
            let png = library::read_image_png(&metadata.id)?;
            CardContent {
                id: Some(metadata.id),
                title: metadata.title,
                width: metadata.image.width,
                height: metadata.image.height,
                png,
            }
        }
        None => CardContent {
            id: None,
            title: "No captures yet".to_string(),
            width: 0,
            height: 0,
            png: Vec::new(),
        },
    };

    // 26 px margin from the bottom-right of the (virtual) screen.
    let screen = platform::gpui_virtual_screen_bounds(cx);
    let card_height = THUMBNAIL_HEIGHT + 108.0;
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: gpui::point(
                screen.origin.x + screen.size.width - px(CARD_WIDTH + SCREEN_MARGIN),
                screen.origin.y + screen.size.height - px(card_height + SCREEN_MARGIN),
            ),
            size: size(px(CARD_WIDTH), px(card_height)),
        })),
        titlebar: None,
        app_id: Some("com.snipsy.desktop".to_string()),
        focus: true,
        show: true,
        #[cfg(windows)]
        kind: WindowKind::PopUp,
        #[cfg(not(windows))]
        kind: WindowKind::Normal,
        is_movable: false,
        is_resizable: false,
        ..Default::default()
    };

    let handle = cx
        .open_window(options, |window, cx| {
            let view = cx.new(|cx| QuickAccessCard::new(content, window, cx));
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            view
        })
        .map_err(|error| format!("Could not open the quick-access card: {error}"))?;
    cx.global_mut::<QuickAccessSession>().window = Some(handle);
    Ok(())
}

struct CardContent {
    id: Option<String>,
    title: String,
    width: u32,
    height: u32,
    png: Vec<u8>,
}

pub struct QuickAccessCard {
    focus: gpui::FocusHandle,
    image: Option<Arc<RenderImage>>,
    content: CardContent,
    pending_delete: bool,
    notice: Option<String>,
}

impl QuickAccessCard {
    fn new(content: CardContent, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let image = if content.png.is_empty() {
            None
        } else {
            pin::decode_to_render_image(&content.png).ok()
        };
        Self {
            focus: cx.focus_handle(),
            image,
            content,
            pending_delete: false,
            notice: None,
        }
    }

    fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(handle) = cx
            .try_global::<QuickAccessSession>()
            .and_then(|session| session.window)
        {
            cx.global_mut::<QuickAccessSession>().window = None;
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
        let _ = window;
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        match crate::clipboard::copy_png_image(&self.content.png) {
            Ok(()) => {
                self.notice = Some("Copied to clipboard.".to_string());
            }
            Err(error) => self.notice = Some(error),
        }
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if self.content.png.is_empty() {
            self.notice = Some("There is nothing to save.".to_string());
            cx.notify();
            return;
        }
        match save_dialog::save_png_via_dialog(&self.content.png, &self.content.title) {
            Ok(true) => {
                self.notice = Some("Saved.".to_string());
            }
            Ok(false) => {}
            Err(error) => self.notice = Some(error),
        }
        cx.notify();
    }

    fn annotate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.content.png.is_empty() {
            return;
        }
        let annotations = self.content.id.as_deref().map_or(
            library::AnnotationSource::Transient,
            library::read_annotations,
        );
        let result = editor::window::open(
            cx,
            EditorSource {
                png: self.content.png.clone(),
                title: self.content.title.clone(),
                annotations,
            },
        );
        match result {
            Ok(()) => self.close(window, cx),
            Err(error) => {
                self.notice = Some(error);
                cx.notify();
            }
        }
    }

    fn pin_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.content.png.is_empty() {
            return;
        }
        let scale = platform::primary_scale_factor();
        let virtual_origin = platform::gpui_virtual_screen_bounds(cx).origin;
        let result = pin::open_for_capture(
            cx,
            self.content.png.clone(),
            &self.content.title,
            self.content.width,
            self.content.height,
            scale,
            virtual_origin,
        );
        match result {
            Ok(()) => self.close(window, cx),
            Err(error) => {
                self.notice = Some(error);
                cx.notify();
            }
        }
    }

    fn delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.content.id.clone() else {
            return;
        };
        if !self.pending_delete {
            self.pending_delete = true;
            cx.notify();
            return;
        }
        self.notice = Some(match library::delete_capture(&id) {
            Ok(()) => {
                self.content.png.clear();
                self.image = None;
                "Capture deleted.".to_string()
            }
            Err(error) => error,
        });
        self.pending_delete = false;
        let _ = window;
        cx.notify();
    }

    fn icon_button(
        &self,
        slug: &'static str,
        label: &'static str,
        theme: &Theme,
        icon_color: gpui::Hsla,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
        _cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let button = div()
            .id(gpui::ElementId::Name(format!("qa-{slug}").into()))
            .w(px(32.0))
            .h(px(32.0))
            .rounded(px(R_BTN))
            .flex()
            .items_center()
            .justify_center()
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(theme.bg_elevated))
            .on_mouse_down(MouseButton::Left, handler)
            .child(crate::ui::chrome_icon(slug, 18.0, icon_color));
        crate::ui::with_tooltip(button, label)
    }
}

impl Render for QuickAccessCard {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let danger = theme.danger;
        let image = self.image.clone();
        let image_dims = self.content.image_dimensions();

        let mut card = div()
            .id("quick-access-card")
            .size_full()
            .key_context("QuickAccessCard")
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(px(R_PANEL))
            .bg(theme.bg_toolbar)
            .border_1()
            .border_color(theme.hairline)
            .shadow(theme.shadow_floating.clone())
            .on_action(cx.listener(|this, _: &DismissQuickAccess, window, cx| {
                this.close(window, cx);
            }))
            .on_action(cx.listener(|this, _: &QuickAccessCopy, _, cx| this.copy(cx)))
            .on_action(cx.listener(|this, _: &QuickAccessSave, _, cx| this.save(cx)))
            .on_action(cx.listener(|this, _: &QuickAccessAnnotate, window, cx| {
                this.annotate(window, cx);
            }))
            .on_action(cx.listener(|this, _: &QuickAccessPin, window, cx| {
                this.pin_capture(window, cx);
            }))
            .on_action(cx.listener(|this, _: &QuickAccessDelete, window, cx| {
                this.delete(window, cx);
            }));

        card = card.child(
            div()
                .id("qa-thumbnail")
                .w_full()
                .h(px(THUMBNAIL_HEIGHT))
                .rounded(px(R_THUMB))
                .bg(theme.bg_elevated)
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .child(match image {
                    Some(image) => canvas(
                        move |_, _, _| (image, image_dims),
                        |bounds, (image, dims), window, _| {
                            // Contain-fit inside the thumbnail area.
                            let (w, h) = match dims {
                                Some((w, h)) => (w.max(1) as f32, h.max(1) as f32),
                                None => return,
                            };
                            let fit = (bounds.size.width / px(w))
                                .min(bounds.size.height / px(h))
                                .min(1.0);
                            let draw = size(px(w * fit), px(h * fit));
                            let image_bounds = Bounds {
                                origin: gpui::point(
                                    bounds.origin.x + (bounds.size.width - draw.width) / 2.0,
                                    bounds.origin.y + (bounds.size.height - draw.height) / 2.0,
                                ),
                                size: draw,
                            };
                            window
                                .paint_image(
                                    bounds,
                                    image_bounds,
                                    Corners::all(px(R_THUMB)),
                                    image.clone(),
                                    0,
                                    false,
                                )
                                .ok();
                        },
                    )
                    .w_full()
                    .h_full()
                    .into_any_element(),
                    None => div()
                        .text_size(px(11.0))
                        .text_color(theme.text_3)
                        .font_family(crate::ui::font_family())
                        .child("No capture")
                        .into_any_element(),
                }),
        );

        card = card.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.text_2)
                        .font_family(crate::ui::font_family())
                        .overflow_hidden()
                        .text_ellipsis()
                        .flex_1()
                        .child(self.content.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.text_3)
                        .font_family(crate::ui::font_family())
                        .child(self.content.dimensions_label()),
                ),
        );

        if let Some(notice) = &self.notice {
            card = card.child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.text_2)
                    .font_family(crate::ui::font_family())
                    .child(notice.clone()),
            );
        }
        if self.pending_delete {
            card = card.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(danger)
                            .font_family(crate::ui::font_family())
                            .child("Delete this capture?"),
                    )
                    .child(
                        div()
                            .id("qa-delete-confirm")
                            .px_2()
                            .py_1()
                            .rounded(px(R_INPUT))
                            .bg(theme.bg_elevated)
                            .text_size(px(11.0))
                            .text_color(danger)
                            .font_family(crate::ui::font_family())
                            .cursor(gpui::CursorStyle::PointingHand)
                            .child("Delete")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                                    this.delete(window, cx);
                                }),
                            ),
                    ),
            );
        }

        // Action row: Copy and Save as equal buttons, then the icon actions.
        let icon_color = theme.text_1;
        let actions = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .w_full()
            .child(
                div()
                    .id("qa-copy")
                    .flex_1()
                    .h(px(34.0))
                    .rounded(px(R_BTN))
                    .bg(theme.bg_elevated)
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .text_size(px(12.0))
                    .text_color(theme.text_1)
                    .font_family(crate::ui::font_family())
                    .cursor(gpui::CursorStyle::PointingHand)
                    .hover(move |style| style.bg(theme.hairline_strong))
                    .child("Copy")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| this.copy(cx)),
                    ),
            )
            .child(
                div()
                    .id("qa-save")
                    .flex_1()
                    .h(px(34.0))
                    .rounded(px(R_BTN))
                    .bg(theme.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .text_size(px(12.0))
                    .text_color(gpui::white())
                    .font_family(crate::ui::font_family())
                    .cursor(gpui::CursorStyle::PointingHand)
                    .child("Save")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| this.save(cx)),
                    ),
            )
            .child(div().w(px(2.0)))
            .child(self.icon_button(
                "pen",
                "Annotate",
                &theme,
                icon_color,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.annotate(window, cx)),
                cx,
            ))
            .child(self.icon_button(
                "pin",
                "Pin",
                &theme,
                icon_color,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.pin_capture(window, cx)),
                cx,
            ))
            .child(self.icon_button(
                "delete",
                "Delete",
                &theme,
                danger,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.delete(window, cx)),
                cx,
            ))
            .child(self.icon_button(
                "close",
                "Close",
                &theme,
                icon_color,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.close(window, cx)),
                cx,
            ));
        card.child(actions)
    }
}

impl CardContent {
    fn image_dimensions(&self) -> Option<(u32, u32)> {
        (self.width > 0).then_some((self.width, self.height))
    }

    fn dimensions_label(&self) -> String {
        match self.image_dimensions() {
            Some((w, h)) => format!("{w} × {h}"),
            None => String::new(),
        }
    }
}

/// Keybindings for the quick-access card.
pub fn keybindings() -> Vec<KeyBinding> {
    vec![KeyBinding::new("escape", DismissQuickAccess, None)]
}
