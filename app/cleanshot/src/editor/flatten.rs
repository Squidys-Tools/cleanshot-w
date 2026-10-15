//! Flatten: rasterize the document's shapes over the capture image at the
//! ORIGINAL image resolution, CPU-side, via tiny-skia. This is what "Save
//! PNG" and "copy to clipboard" produce; the GPU never reads back.
//!
//! Text (the text shape and counter digits) is rasterized with fontdue
//! against a system font, funneled through the [`TextPainter`] trait so tests
//! can substitute a deterministic painter and still exercise every blending
//! and geometry path.

use tiny_skia::{Color, Pixmap, PremultipliedColorU8, Stroke};

use super::model::{COUNTER_DIAMETER, Document, Rect, Shape};

/// Draws text into a tiny-skia pixmap. `paint` places the text block's
/// top-left at (x, y) and returns the laid-out height; `measure` returns
/// (width, height) of the laid-out block. Text may contain `\n` for line
/// breaks.
pub trait TextPainter {
    fn paint(&self, pixmap: &mut Pixmap, text: &str, x: f32, y: f32, font_px: f32, color: Color);
    fn measure(&self, text: &str, font_px: f32) -> (f32, f32);
}

/// The production painter: fontdue glyph rasterization over a system font.
pub struct SystemTextPainter {
    font: Option<fontdue::Font>,
}

const FONT_CANDIDATES: &[&str] = &[
    // Windows
    "C:\\Windows\\Fonts\\segoeui.ttf",
    "C:\\Windows\\Fonts\\arial.ttf",
    "C:\\Windows\\Fonts\\calibri.ttf",
    // Linux
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
];

impl SystemTextPainter {
    pub fn new() -> Self {
        let font = FONT_CANDIDATES
            .iter()
            .find_map(|path| {
                let bytes = std::fs::read(path).ok()?;
                fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
            })
            .or_else(|| {
                // Last resort: any font the system font directories offer.
                font_files_from_dirs()
                    .into_iter()
                    .find_map(|path| {
                        let bytes = std::fs::read(&path).ok()?;
                        fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
                    })
            });
        if font.is_none() {
            eprintln!("cleanshot-w: no system font found; text shapes will not flatten");
        }
        SystemTextPainter { font }
    }

    /// Line height for a font size (1.3 line gap, matching the canvas view).
    fn line_height(font_px: f32) -> f32 {
        font_px * 1.3
    }
}

fn font_files_from_dirs() -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let dirs = ["/usr/share/fonts", "C:\\Windows\\Fonts"];
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "ttf" || ext == "otf") {
                found.push(path);
            }
        }
    }
    found
}

impl Default for SystemTextPainter {
    fn default() -> Self {
        Self::new()
    }
}

impl TextPainter for SystemTextPainter {
    fn paint(&self, pixmap: &mut Pixmap, text: &str, x: f32, y: f32, font_px: f32, color: Color) {
        let Some(font) = &self.font else {
            return;
        };
        for (line_index, line) in text.split('\n').enumerate() {
            let baseline = y + Self::line_height(font_px) * line_index as f32 + font_px;
            let mut cursor = x;
            for ch in line.chars() {
                let (metrics, bitmap) = font.rasterize(ch, font_px);
                if metrics.width > 0 && metrics.height > 0 {
                    // Position the glyph box: baseline minus the glyph's
                    // ascent span, plus its bearing.
                    let gx = cursor + metrics.xmin as f32;
                    let gy = baseline - (metrics.height as f32 + metrics.ymin as f32);
                    blit_glyph(pixmap, &bitmap, metrics.width, gx, gy, color);
                }
                cursor += metrics.advance_width;
            }
        }
    }

    fn measure(&self, text: &str, font_px: f32) -> (f32, f32) {
        let Some(font) = &self.font else {
            return (0.0, Self::line_height(font_px) * text.split('\n').count() as f32);
        };
        let mut width = 0.0f32;
        for line in text.split('\n') {
            let line_width: f32 = line
                .chars()
                .map(|ch| font.rasterize(ch, font_px).0.advance_width)
                .sum();
            width = width.max(line_width);
        }
        let lines = text.split('\n').count() as f32;
        (width, Self::line_height(font_px) * lines)
    }
}

/// Alpha-over blend one glyph bitmap (8-bit coverage per pixel, `glyph_width`
/// wide) with its top-left at (x, y).
fn blit_glyph(pixmap: &mut Pixmap, bitmap: &[u8], glyph_width: usize, x: f32, y: f32, color: Color) {
    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;
    let data = pixmap.data_mut();
    let base_r = (color.red() * 255.0) as u32;
    let base_g = (color.green() * 255.0) as u32;
    let base_b = (color.blue() * 255.0) as u32;
    let base_a = (color.alpha() * 255.0) as u32;
    for (index, &coverage) in bitmap.iter().enumerate() {
        if coverage == 0 {
            continue;
        }
        let gx = index as i32 % glyph_width as i32;
        let gy = index as i32 / glyph_width as i32;
        let px = x as i32 + gx;
        let py = y as i32 + gy;
        if px < 0 || py < 0 || px >= width || py >= height {
            continue;
        }
        let alpha = coverage as u32 * base_a / 255;
        let offset = ((py * width + px) * 4) as usize;
        let dst = [data[offset] as u32, data[offset + 1] as u32, data[offset + 2] as u32, data[offset + 3] as u32];
        let inv = 255 - alpha;
        data[offset] = ((base_r * alpha + dst[0] * inv) / 255) as u8;
        data[offset + 1] = ((base_g * alpha + dst[1] * inv) / 255) as u8;
        data[offset + 2] = ((base_b * alpha + dst[2] * inv) / 255) as u8;
        data[offset + 3] = ((alpha * 255 + dst[3] * inv) / 255) as u8;
    }
}

/// Flatten the document over `image_rgba` (RGBA8, document image size) and
/// return the composed raster.
pub fn flatten(document: &Document, image_rgba: &[u8], painter: &dyn TextPainter) -> Result<Pixmap, String> {
    let width = document.image_width;
    let height = document.image_height;
    if (image_rgba.len()) != (width as usize * height as usize * 4) {
        return Err(format!(
            "The image buffer is {} bytes but the document says {}×{}.",
            image_rgba.len(),
            width,
            height
        ));
    }
    let mut pixmap = Pixmap::new(width, height).ok_or("The image is too large to rasterize.")?;
    // The capture is the opaque background.
    for (pixel, chunk) in pixmap.pixels_mut().iter_mut().zip(image_rgba.chunks_exact(4)) {
        let color = PremultipliedColorU8::from_rgba(chunk[0], chunk[1], chunk[2], chunk[3])
            .ok_or("The image buffer holds an invalid pixel.")?;
        *pixel = color;
    }
    for shape in &document.shapes {
        draw_shape(&mut pixmap, shape, painter);
    }
    Ok(pixmap)
}

/// Flatten and encode straight to PNG bytes.
pub fn flatten_to_png(document: &Document, image_rgba: &[u8]) -> Result<Vec<u8>, String> {
    let pixmap = flatten(document, image_rgba, &SystemTextPainter::new())?;
    pixmap
        .encode_png()
        .map_err(|error| format!("Could not encode the flattened PNG: {error}"))
}

fn stroke_of(width: f32) -> Stroke {
    Stroke {
        width: width.max(1.0),
        ..Stroke::default()
    }
}

fn paint(color: Color) -> tiny_skia::Paint<'static> {
    tiny_skia::Paint {
        shader: tiny_skia::Shader::SolidColor(color),
        ..tiny_skia::Paint::default()
    }
}

fn draw_shape(pixmap: &mut Pixmap, shape: &Shape, painter: &dyn TextPainter) {
    match shape {
        Shape::Rectangle(rect) => {
            let paint = paint(parse_color(&rect.color));
            if let Some(path) = closed_polyline(&[
                (rect.x, rect.y),
                (rect.x + rect.w, rect.y),
                (rect.x + rect.w, rect.y + rect.h),
                (rect.x, rect.y + rect.h),
            ]) {
                pixmap.stroke_path(&path, &paint, &stroke_of(rect.stroke_width), tiny_skia::Transform::default(), None);
            }
        }
        Shape::Ellipse(ellipse) => {
            let paint = paint(parse_color(&ellipse.color));
            if let Some(path) = ellipse_path(ellipse.x, ellipse.y, ellipse.w, ellipse.h) {
                pixmap.stroke_path(&path, &paint, &stroke_of(ellipse.stroke_width), tiny_skia::Transform::default(), None);
            }
        }
        Shape::Arrow(arrow) => {
            let paint = paint(parse_color(&arrow.color));
            let head_size = (arrow.stroke_width * 4.0).max(12.0);
            if let Some(line) = open_polyline(&[(arrow.x1, arrow.y1), (arrow.x2, arrow.y2)]) {
                pixmap.stroke_path(&line, &paint, &stroke_of(arrow.stroke_width), tiny_skia::Transform::default(), None);
            }
            if let Some(head) = closed_polyline(&arrow.head_triangle(head_size)) {
                pixmap.fill_path(&head, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::default(), None);
            }
        }
        Shape::Pen(pen) => {
            if pen.points.len() >= 2 {
                let paint = paint(parse_color(&pen.color));
                if let Some(path) = open_polyline(&pen.points) {
                    pixmap.stroke_path(&path, &paint, &stroke_of(pen.stroke_width), tiny_skia::Transform::default(), None);
                }
            }
        }
        Shape::Text(text) => {
            let color = parse_color(&text.color);
            painter.paint(pixmap, &text.text, text.x, text.y, text.size.max(1.0), color);
        }
        Shape::Counter(counter) => {
            let white = paint(Color::WHITE);
            let color = parse_color(&counter.color);
            let cx = counter.x + COUNTER_DIAMETER / 2.0;
            let cy = counter.y + COUNTER_DIAMETER / 2.0;
            let radius = COUNTER_DIAMETER / 2.0;
            if let Some(badge) = ellipse_path(cx - radius, cy - radius, radius * 2.0, radius * 2.0) {
                pixmap.fill_path(&badge, &paint(color), tiny_skia::FillRule::Winding, tiny_skia::Transform::default(), None);
            }
            // White ring so the badge reads on any background, then the digit.
            if let Some(ring) = ellipse_path(cx - radius + 2.0, cy - radius + 2.0, (radius - 2.0) * 2.0, (radius - 2.0) * 2.0) {
                pixmap.stroke_path(&ring, &white, &stroke_of(2.0), tiny_skia::Transform::default(), None);
            }
            let label = counter.n.to_string();
            let digit_height = COUNTER_DIAMETER * 0.55;
            let (digit_width, digit_box_height) = painter.measure(&label, digit_height);
            painter.paint(pixmap, &label, cx - digit_width / 2.0, cy - digit_box_height / 2.0, digit_height, Color::WHITE);
        }
    }
}

fn parse_color(hex: &str) -> Color {
    let bytes = hex.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return Color::BLACK;
    }
    let value = |pair: &[u8]| -> f32 {
        u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("00"), 16)
            .map(|channel| channel as f32 / 255.0)
            .unwrap_or(0.0)
    };
    Color::from_rgba(value(&bytes[1..3]), value(&bytes[3..5]), value(&bytes[5..7]), 1.0).unwrap_or(Color::BLACK)
}

fn closed_polyline(points: &[(f32, f32)]) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();
    let first = points.first()?;
    builder.move_to(first.0, first.1);
    for point in &points[1..] {
        builder.line_to(point.0, point.1);
    }
    builder.close();
    builder.finish()
}

fn open_polyline(points: &[(f32, f32)]) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();
    let first = points.first()?;
    builder.move_to(first.0, first.1);
    for point in &points[1..] {
        builder.line_to(point.0, point.1);
    }
    builder.finish()
}

/// An ellipse approximated with 64 segments (imperceptible at capture sizes).
fn ellipse_path(x: f32, y: f32, w: f32, h: f32) -> Option<tiny_skia::Path> {
    const SEGMENTS: usize = 64;
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rx = (w / 2.0).max(0.5);
    let ry = (h / 2.0).max(0.5);
    let mut builder = tiny_skia::PathBuilder::new();
    for index in 0..SEGMENTS {
        let angle = index as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let px = cx + angle.cos() * rx;
        let py = cy + angle.sin() * ry;
        if index == 0 {
            builder.move_to(px, py);
        } else {
            builder.line_to(px, py);
        }
    }
    builder.close();
    builder.finish()
}

/// A test-double text painter with deterministic metrics: every character is
/// `font_px/2` wide and `font_px` tall, drawn as a solid block. Geometry and
/// blending tests use this; the fontdue painter needs a real font file and
/// would make tests machine-dependent.
pub struct BlockTextPainter;

impl TextPainter for BlockTextPainter {
    fn paint(&self, pixmap: &mut Pixmap, text: &str, x: f32, y: f32, font_px: f32, color: Color) {
        for (line_index, line) in text.split('\n').enumerate() {
            let line_y = y + (font_px * 1.3 * line_index as f32) + font_px * 0.15;
            let mut cursor = x;
            for ch in line.chars() {
                let advance = font_px / 2.0;
                let width = (advance * 0.8).max(1.0);
                let height = (font_px * 0.7).max(1.0);
                if ch != ' ' {
                    if let Some(rect) = tiny_skia::Rect::from_xywh(cursor, line_y, width, height) {
                    let path = tiny_skia::PathBuilder::from_rect(rect);
                    pixmap.fill_path(&path, &paint(color), tiny_skia::FillRule::Winding, tiny_skia::Transform::default(), None);
                }
                }
                cursor += advance;
            }
        }
    }

    fn measure(&self, text: &str, font_px: f32) -> (f32, f32) {
        let max_line = text
            .split('\n')
            .map(|line| line.chars().count() as f32 * font_px / 2.0)
            .fold(0.0f32, f32::max);
        (max_line, font_px * 1.3 * text.split('\n').count() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::model::{
        ArrowShape, CounterShape, Document, EllipseShape, PenShape, RectShape, TextShape,
    };

    fn solid_image(width: u32, height: u32) -> Vec<u8> {
        vec![255; (width * height * 4) as usize]
    }

    #[test]
    fn flatten_rejects_a_buffer_that_does_not_match_the_document_size() {
        let document = Document::new(10, 10);
        let result = flatten(&document, &[0; 100], &BlockTextPainter);
        assert!(result.is_err());
    }

    #[test]
    fn flatten_output_keeps_the_original_resolution() {
        let document = Document::new(64, 48);
        let pixmap = flatten(&document, &solid_image(64, 48), &BlockTextPainter).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (64, 48));
    }

    #[test]
    fn image_background_survives_flatten() {
        // A non-white background must still be the background after flattening
        // an empty document.
        let mut image = vec![255u8; 8 * 8 * 4];
        for chunk in image.chunks_exact_mut(4) {
            chunk.copy_from_slice(&[10, 20, 30, 255]);
        }
        let document = Document::new(8, 8);
        let pixmap = flatten(&document, &image, &BlockTextPainter).unwrap();
        let pixel = pixmap.pixel(0, 0).unwrap();
        assert_eq!(
            [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()],
            [10, 20, 30, 255]
        );
    }

    #[test]
    fn rectangle_stroke_lands_on_the_border_and_not_the_center() {
        let mut document = Document::new(100, 100);
        document.shapes.push(Shape::Rectangle(RectShape {
            id: "r".to_string(),
            x: 20.0,
            y: 20.0,
            w: 60.0,
            h: 40.0,
            color: "#ff0000".to_string(),
            stroke_width: 4.0,
        }));
        let pixmap = flatten(&document, &solid_image(100, 100), &BlockTextPainter).unwrap();
        let pixel = pixmap.pixel(20, 40).unwrap(); // on the left border
        assert!(pixel.red() > 200, "border should be red, got {pixel:?}");
        let center = pixmap.pixel(50, 40).unwrap();
        assert_eq!(center.red(), 255, "center should stay background");
        let outside = pixmap.pixel(10, 10).unwrap();
        assert_eq!(outside.red(), 255, "outside should stay background");
    }

    #[test]
    fn ellipse_fill_and_stroke_are_distinguishable() {
        let mut document = Document::new(100, 100);
        document.shapes.push(Shape::Ellipse(EllipseShape {
            id: "e".to_string(),
            x: 10.0,
            y: 10.0,
            w: 80.0,
            h: 80.0,
            color: "#00ff00".to_string(),
            stroke_width: 6.0,
        }));
        let pixmap = flatten(&document, &solid_image(100, 100), &BlockTextPainter).unwrap();
        let vertex = pixmap.pixel(50, 10).unwrap(); // top vertex on the stroke
        assert!(vertex.green() > 200, "stroke should be green, got {vertex:?}");
        let center = pixmap.pixel(50, 50).unwrap();
        assert_eq!(center.green(), 255, "unfilled center stays background");
    }

    #[test]
    fn arrow_draws_line_and_head_beyond_the_endpoint() {
        let mut document = Document::new(100, 100);
        document.shapes.push(Shape::Arrow(ArrowShape {
            id: "a".to_string(),
            x1: 10.0,
            y1: 50.0,
            x2: 60.0,
            y2: 50.0,
            color: "#0000ff".to_string(),
            stroke_width: 4.0,
        }));
        let pixmap = flatten(&document, &solid_image(100, 100), &BlockTextPainter).unwrap();
        let mid_line = pixmap.pixel(35, 50).unwrap();
        assert!(mid_line.blue() > 200, "line body should be blue, got {mid_line:?}");
        // The head tip overshoots x2 by 40% of the head size (head = 16 → tip ≈ 66.4).
        let tip = pixmap.pixel(66, 50).unwrap();
        assert!(tip.blue() > 200, "head tip should be blue, got {tip:?}");
    }

    #[test]
    fn pen_draws_along_its_polyline() {
        let mut document = Document::new(100, 100);
        document.shapes.push(Shape::Pen(PenShape {
            id: "p".to_string(),
            points: vec![(10.0, 10.0), (90.0, 90.0)],
            color: "#1a1a1a".to_string(),
            stroke_width: 6.0,
        }));
        let pixmap = flatten(&document, &solid_image(100, 100), &BlockTextPainter).unwrap();
        let midpoint = pixmap.pixel(50, 50).unwrap();
        assert!(midpoint.red() < 100, "pen stroke should be dark, got {midpoint:?}");
        let off_stroke = pixmap.pixel(50, 20).unwrap();
        assert_eq!(off_stroke.red(), 255, "off the polyline stays background");
    }

    #[test]
    fn counter_badge_is_filled_and_white_ringed() {
        // Mid-gray background so the white digit is distinguishable from it.
        let image = vec![128u8; 100 * 100 * 4];
        let mut document = Document::new(100, 100);
        document.shapes.push(Shape::Counter(CounterShape {
            id: "c".to_string(),
            x: 20.0,
            y: 20.0,
            n: 7,
            color: "#e03131".to_string(),
        }));
        let pixmap = flatten(&document, &image, &BlockTextPainter).unwrap();
        let cx = 20.0 + COUNTER_DIAMETER / 2.0;
        let badge_left = pixmap.pixel(26, cx as u32).unwrap();
        assert!(badge_left.red() > 150 && badge_left.green() < 120, "badge should be red, got {badge_left:?}");
        // The digit block (BlockTextPainter, white) covers the badge center.
        let center = pixmap.pixel(cx as u32, cx as u32).unwrap();
        assert!(center.red() > 200 && center.green() > 200, "digit should be white on the badge, got {center:?}");
        // The white ring sits just inside the badge edge.
        let ring = pixmap.pixel(cx as u32, 22).unwrap(); // the 2px band is centered at radius 14
        assert!(ring.red() > 200 && ring.green() > 200, "ring should be white, got {ring:?}");
    }

    #[test]
    fn text_shape_rasterizes_through_the_painter() {
        let mut document = Document::new(100, 100);
        document.shapes.push(Shape::Text(TextShape {
            id: "t".to_string(),
            x: 10.0,
            y: 10.0,
            text: "hi".to_string(),
            size: 20.0,
            color: "#0000ff".to_string(),
        }));
        let pixmap = flatten(&document, &solid_image(100, 100), &BlockTextPainter).unwrap();
        // BlockTextPainter draws each char as a block at font_px/2 advance:
        // 'h' block covers x ≈ 10..18, y ≈ 13..27.
        let block = pixmap.pixel(12, 18).unwrap();
        assert!(block.blue() > 200, "text block should be blue, got {block:?}");
        let after = pixmap.pixel(30, 18).unwrap();
        assert_eq!(after.blue(), 255, "beyond the text stays background");
    }

    #[test]
    fn shapes_stack_in_document_order() {
        // The later shape paints on top.
        let mut document = Document::new(100, 100);
        document.shapes.push(Shape::Rectangle(RectShape {
            id: "below".to_string(),
            x: 0.0,
            y: 40.0,
            w: 100.0,
            h: 8.0,
            color: "#ff0000".to_string(),
            stroke_width: 8.0,
        }));
        document.shapes.push(Shape::Pen(PenShape {
            id: "above".to_string(),
            points: vec![(0.0, 44.0), (100.0, 44.0)],
            color: "#00ff00".to_string(),
            stroke_width: 2.0,
        }));
        let pixmap = flatten(&document, &solid_image(100, 100), &BlockTextPainter).unwrap();
        let pixel = pixmap.pixel(50, 44).unwrap();
        assert!(pixel.green() > 200, "the pen (later) must win, got {pixel:?}");
    }

    #[test]
    fn block_painter_metrics_are_deterministic() {
        let (width, height) = BlockTextPainter.measure("ab\ncd", 20.0);
        assert_eq!(width, 20.0, "two chars at font_px/2 per line");
        assert_eq!(height, 52.0, "two lines at font_px × 1.3");
    }

    #[test]
    fn parse_color_handles_palette_values_and_garbage() {
        let red = parse_color("#e03131");
        assert!(red.red() > 0.85);
        assert!(red.green() < 0.25);
        assert_eq!(parse_color("garbage"), parse_color("#000000"));
    }

    #[test]
    fn crop_rect_survives_round_trip_through_document_crop() {
        let rect = Rect::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(rect.right(), 4.0);
        assert_eq!(rect.bottom(), 6.0);
        assert!(rect.contains(2.0, 3.0));
        assert!(!rect.contains(5.0, 3.0));
    }
}
