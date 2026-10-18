//! The sticker icon set: one curated HugeIcons library (see
//! `mockups/snipsy-ui-redesign/icon-mapping.md`). Icons render on the canvas
//! from pre-rasterized 512px PNGs, tinted at runtime by alpha replacement.

use image::{RgbaImage, imageops};
use serde::{Deserialize, Serialize};

pub(super) const MIN_ICON_SIZE: f32 = 8.0;
pub(super) const MAX_ICON_SIZE: f32 = 1024.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconGlyph {
    ArrowRight,
    Check,
    Close,
    Info,
    Warning,
    Question,
    #[default]
    Star,
    Heart,
    Lock,
    Eye,
    User,
    Camera,
    Flag,
    PinSticker,
    ThumbsUp,
    Clock,
    Mail,
    Smile,
    Bookmark,
    CheckCircle,
    CrossCircle,
    Plus,
    Minus,
    Scissors,
}

impl IconGlyph {
    pub const ALL: [Self; 24] = [
        Self::ArrowRight,
        Self::Check,
        Self::Close,
        Self::Info,
        Self::Warning,
        Self::Question,
        Self::Star,
        Self::Heart,
        Self::Lock,
        Self::Eye,
        Self::User,
        Self::Camera,
        Self::Flag,
        Self::PinSticker,
        Self::ThumbsUp,
        Self::Clock,
        Self::Mail,
        Self::Smile,
        Self::Bookmark,
        Self::CheckCircle,
        Self::CrossCircle,
        Self::Plus,
        Self::Minus,
        Self::Scissors,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ArrowRight => "Arrow right",
            Self::Check => "Check",
            Self::Close => "Close",
            Self::Info => "Information",
            Self::Warning => "Warning",
            Self::Question => "Question",
            Self::Star => "Star",
            Self::Heart => "Heart",
            Self::Lock => "Lock",
            Self::Eye => "Eye",
            Self::User => "Person",
            Self::Camera => "Camera",
            Self::Flag => "Flag",
            Self::PinSticker => "Pin",
            Self::ThumbsUp => "Thumbs up",
            Self::Clock => "Clock",
            Self::Mail => "Mail",
            Self::Smile => "Smile",
            Self::Bookmark => "Bookmark",
            Self::CheckCircle => "Check circle",
            Self::CrossCircle => "Cross circle",
            Self::Plus => "Plus",
            Self::Minus => "Minus",
            Self::Scissors => "Scissors",
        }
    }
}

include!("icon_assets.rs");

pub fn rasterize(glyph: IconGlyph, color: &str, size: f32) -> Result<RgbaImage, String> {
    if !size.is_finite() || !(MIN_ICON_SIZE..=MAX_ICON_SIZE).contains(&size) {
        return Err("The icon size is invalid.".to_string());
    }
    let color = parse_color(color)?;
    let bytes = ICON_ASSETS
        .iter()
        .find_map(|(asset_glyph, bytes)| (*asset_glyph == glyph).then_some(*bytes))
        .ok_or_else(|| format!("The {glyph:?} icon is not bundled."))?;
    let source = image::load_from_memory(bytes)
        .map_err(|error| format!("Could not load the bundled icon: {error}"))?
        .into_rgba8();
    let size = size.round() as u32;
    let mut image = imageops::resize(&source, size, size, imageops::FilterType::Lanczos3);
    for pixel in image.pixels_mut() {
        pixel.0 = [color[0], color[1], color[2], pixel.0[3]];
    }
    Ok(image)
}

fn parse_color(color: &str) -> Result<[u8; 3], String> {
    let Some(hex) = color.strip_prefix('#') else {
        return Err("The icon color is invalid.".to_string());
    };
    if hex.len() != 6 {
        return Err("The icon color is invalid.".to_string());
    }
    let channel = |start| {
        u8::from_str_radix(&hex[start..start + 2], 16)
            .map_err(|_| "The icon color is invalid.".to_string())
    };
    Ok([channel(0)?, channel(2)?, channel(4)?])
}

#[cfg(test)]
mod tests {
    use super::{ICON_ASSETS, IconGlyph, rasterize};

    #[test]
    fn every_glyph_is_bundled_and_tints_transparent_pngs() {
        assert_eq!(ICON_ASSETS.len(), IconGlyph::ALL.len());
        for glyph in IconGlyph::ALL {
            let image = rasterize(glyph, "#e03131", 32.0).unwrap();
            assert_eq!(image.dimensions(), (32, 32));
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
            assert!(
                image
                    .pixels()
                    .filter(|pixel| pixel.0[3] > 0)
                    .all(|pixel| pixel.0[..3] == [0xe0, 0x31, 0x31])
            );
        }
    }

    #[test]
    fn glyph_json_is_stable() {
        assert_eq!(
            serde_json::to_string(&IconGlyph::ArrowRight).unwrap(),
            "\"arrow-right\""
        );
        assert_eq!(
            serde_json::from_str::<IconGlyph>("\"pin-sticker\"").unwrap(),
            IconGlyph::PinSticker
        );
    }
}
