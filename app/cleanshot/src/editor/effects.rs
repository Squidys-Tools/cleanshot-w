use image::{Rgba, RgbaImage, imageops};

use super::model::{EffectKind, EffectShape};

pub const REDACT_RGBA: [u8; 4] = [0x11, 0x18, 0x27, 0xff];

#[derive(Debug, Clone)]
pub struct EffectPatch {
    pub x: u32,
    pub y: u32,
    pub rgba: RgbaImage,
}

pub fn rasterize(source: &RgbaImage, shape: &EffectShape) -> Option<EffectPatch> {
    let bounds = shape.bounds;
    if ![bounds.x, bounds.y, bounds.w, bounds.h]
        .iter()
        .all(|value| value.is_finite())
        || bounds.w <= 0.0
        || bounds.h <= 0.0
    {
        return None;
    }

    let (width, height) = source.dimensions();
    let x0 = bounds.x.floor().clamp(0.0, width as f32) as u32;
    let y0 = bounds.y.floor().clamp(0.0, height as f32) as u32;
    let x1 = (bounds.x + bounds.w).ceil().clamp(0.0, width as f32) as u32;
    let y1 = (bounds.y + bounds.h).ceil().clamp(0.0, height as f32) as u32;
    if x0 >= x1 || y0 >= y1 {
        return None;
    }

    let mut rgba = RgbaImage::new(x1 - x0, y1 - y0);
    match &shape.kind {
        EffectKind::Blur { radius_px } if (0.1..=128.0).contains(radius_px) => {
            let blurred = imageops::blur(source, *radius_px);
            for y in y0..y1 {
                for x in x0..x1 {
                    rgba.put_pixel(x - x0, y - y0, *blurred.get_pixel(x, y));
                }
            }
        }
        EffectKind::Pixelate { block_px } if (2..=256).contains(block_px) => {
            for block_y in (y0 / *block_px * *block_px..y1).step_by(*block_px as usize) {
                for block_x in (x0 / *block_px * *block_px..x1).step_by(*block_px as usize) {
                    let block_right = (block_x + *block_px).min(width);
                    let block_bottom = (block_y + *block_px).min(height);
                    let mut channels = [0u64; 4];
                    let mut count = 0u64;
                    for y in block_y..block_bottom {
                        for x in block_x..block_right {
                            let pixel = source.get_pixel(x, y);
                            for (channel, value) in channels.iter_mut().zip(pixel.0) {
                                *channel += u64::from(value);
                            }
                            count += 1;
                        }
                    }
                    if count == 0 {
                        continue;
                    }
                    let average = Rgba(channels.map(|value| (value / count) as u8));
                    let patch_left = block_x.max(x0);
                    let patch_top = block_y.max(y0);
                    for y in patch_top..block_bottom.min(y1) {
                        for x in patch_left..block_right.min(x1) {
                            rgba.put_pixel(x - x0, y - y0, average);
                        }
                    }
                }
            }
        }
        EffectKind::Redact => {
            for pixel in rgba.pixels_mut() {
                *pixel = Rgba(REDACT_RGBA);
            }
        }
        _ => return None,
    }

    Some(EffectPatch { x: x0, y: y0, rgba })
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{REDACT_RGBA, rasterize};
    use crate::editor::model::{EffectKind, EffectShape, Rect};

    #[test]
    fn redact_clips_to_the_image_and_returns_only_covered_pixels() {
        let source = RgbaImage::from_pixel(4, 2, Rgba([200, 180, 160, 255]));
        let patch = rasterize(
            &source,
            &EffectShape {
                id: "redact".to_string(),
                bounds: Rect::new(-1.0, 0.25, 3.0, 1.0),
                kind: EffectKind::Redact,
            },
        )
        .unwrap();

        assert_eq!((patch.x, patch.y), (0, 0));
        assert_eq!(patch.rgba.dimensions(), (2, 2));
        assert!(patch.rgba.pixels().all(|pixel| pixel.0 == REDACT_RGBA));
        assert_eq!(source.get_pixel(2, 0), &Rgba([200, 180, 160, 255]));
    }

    #[test]
    fn pixelation_uses_image_anchored_blocks_and_clips_writes() {
        let source = RgbaImage::from_fn(4, 2, |x, _| {
            Rgba([x as u8 * 10, x as u8 * 10, x as u8 * 10, 255])
        });
        let patch = rasterize(
            &source,
            &EffectShape {
                id: "pixelate".to_string(),
                bounds: Rect::new(1.0, 0.0, 2.0, 2.0),
                kind: EffectKind::Pixelate { block_px: 2 },
            },
        )
        .unwrap();

        assert_eq!(patch.rgba.dimensions(), (2, 2));
        assert_eq!(patch.rgba.get_pixel(0, 0).0, [5, 5, 5, 255]);
        assert_eq!(patch.rgba.get_pixel(1, 0).0, [25, 25, 25, 255]);
    }
}
