//! Chrome icon rendering, shared by every Snipsy surface.
//!
//! Icons are HugeIcons free-set SVGs bundled by
//! `scripts/generate-hugeicons-icons.mjs` (see `assets/icons/hugeicons/`).
//! gpui's `svg()` element tints through the element's text color and paints
//! nothing until one is set — every icon element here passes a color.

use gpui::{AnyElement, Hsla, div, prelude::*, px};

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
