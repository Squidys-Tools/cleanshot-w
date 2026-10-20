//! Chrome icon rendering and tooltips, shared by every Snipsy surface.
//!
//! Icons are HugeIcons free-set SVGs bundled by
//! `scripts/generate-hugeicons-icons.mjs` (see `assets/icons/hugeicons/`).
//! gpui's `svg()` element tints through the element's text color and paints
//! nothing until one is set — every icon element here passes a color.

use gpui::{AnyElement, Context, Hsla, Render, SharedString, Window, div, prelude::*, px};

include!("ui_chrome.rs");

/// The bytes of a bundled chrome icon SVG.
pub fn chrome_svg(slug: &str) -> &'static [u8] {
    CHROME_ICONS
        .iter()
        .find(|(candidate, _)| *candidate == slug)
        .map_or_else(
            || panic!("unknown chrome icon slug {slug:?}; add it to scripts/generate-hugeicons-icons.mjs"),
            |(_, bytes)| *bytes,
        )
}

/// A tinted chrome icon, painted by gpui's svg element.
pub fn chrome_icon(slug: &'static str, size: f32, color: Hsla) -> AnyElement {
    let bytes = chrome_svg(slug);
    div()
        .w(px(size))
        .h(px(size))
        .child(gpui::svg().data(bytes).text_color(color).size_full())
        .into_any_element()
}

/// The app-wide UI font family (Segoe UI on Windows, DejaVu Sans on the
/// Linux dev loop).
pub fn font_family() -> &'static str {
    #[cfg(windows)]
    {
        "Segoe UI"
    }
    #[cfg(not(windows))]
    {
        "DejaVu Sans"
    }
}

/// A minimal tooltip view: one caption line. `with_tooltip` attaches it.
pub struct Tooltip {
    text: SharedString,
}

impl Tooltip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded(px(6.0))
            .bg(gpui::black().alpha(0.92))
            .border_1()
            .border_color(gpui::white().alpha(0.14))
            .text_size(px(11.0))
            .text_color(gpui::white())
            .font_family(font_family())
            .child(self.text.clone())
    }
}

/// Attach a hover tooltip with `text` to a stateful div.
pub fn with_tooltip(
    element: gpui::Stateful<gpui::Div>,
    text: &'static str,
) -> gpui::Stateful<gpui::Div> {
    element.tooltip(move |_, cx| cx.new(|_| Tooltip::new(text)).into())
}

#[cfg(test)]
mod tests {
    use super::CHROME_ICONS;

    #[test]
    fn every_chrome_slug_is_bundled_as_svg() {
        assert!(!CHROME_ICONS.is_empty());
        for (slug, bytes) in CHROME_ICONS {
            assert!(!slug.is_empty());
            assert!(bytes.starts_with(b"<svg"), "{slug} is not an SVG");
        }
    }
}
