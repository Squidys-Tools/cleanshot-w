use image::{RgbaImage, imageops};
use serde::{Deserialize, Serialize};

pub(super) const MIN_ICON_SIZE: f32 = 8.0;
pub(super) const MAX_ICON_SIZE: f32 = 1024.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconPack {
    #[default]
    Lucide,
    Phosphor,
    Tabler,
    Heroicons,
    Remix,
}

impl IconPack {
    pub const ALL: [Self; 5] = [
        Self::Lucide,
        Self::Phosphor,
        Self::Tabler,
        Self::Heroicons,
        Self::Remix,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Lucide => "Lucide",
            Self::Phosphor => "Phosphor",
            Self::Tabler => "Tabler",
            Self::Heroicons => "Heroicons",
            Self::Remix => "Remix",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconGlyph {
    ArrowRight,
    Check,
    Close,
    Info,
    Warning,
    Question,
    Star,
    Heart,
    Lock,
    Eye,
    Camera,
    User,
}

impl IconGlyph {
    pub const ALL: [Self; 12] = [
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
        Self::Camera,
        Self::User,
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
            Self::Camera => "Camera",
            Self::User => "Person",
        }
    }
}

include!("icon_assets.rs");

pub fn rasterize(
    pack: IconPack,
    glyph: IconGlyph,
    color: &str,
    size: f32,
) -> Result<RgbaImage, String> {
    if !size.is_finite() || !(MIN_ICON_SIZE..=MAX_ICON_SIZE).contains(&size) {
        return Err("The icon size is invalid.".to_string());
    }
    let color = parse_color(color)?;
    let bytes = ICON_ASSETS
        .iter()
        .find_map(|(asset_pack, asset_glyph, bytes)| {
            (*asset_pack == pack && *asset_glyph == glyph).then_some(*bytes)
        })
        .ok_or_else(|| format!("The {pack:?} icon {glyph:?} is not bundled."))?;
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
    use super::{ICON_ASSETS, IconGlyph, IconPack, rasterize};

    #[test]
    fn every_pack_contains_every_glyph_and_tints_transparent_pngs() {
        assert_eq!(
            ICON_ASSETS.len(),
            IconPack::ALL.len() * IconGlyph::ALL.len()
        );
        for pack in IconPack::ALL {
            for glyph in IconGlyph::ALL {
                let image = rasterize(pack, glyph, "#e03131", 32.0).unwrap();
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
    }

    #[test]
    fn icon_pack_json_is_stable_and_defaults_to_lucide() {
        assert_eq!(
            serde_json::to_string(&IconPack::default()).unwrap(),
            "\"lucide\""
        );
        assert_eq!(
            serde_json::from_str::<IconPack>("\"heroicons\"").unwrap(),
            IconPack::Heroicons
        );
    }
}
