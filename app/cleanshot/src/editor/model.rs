//! The annotation document model: versioned JSON (shapes + image size) stored
//! next to a library capture as `annotations.json`.
//!
//! All shape geometry is in **image pixels** (the capture's original
//! resolution), never screen pixels — the editor canvas multiplies by the zoom
//! factor for display, and export/flatten rasterizes 1:1 at the original
//! resolution. This is a new format; the old tldraw `annotations.json` is
//! deliberately NOT read. Old library entries simply open as un-annotated
//! images (no document file → empty document).

use serde::{Deserialize, Serialize};

use super::icons::{IconGlyph, MAX_ICON_SIZE, MIN_ICON_SIZE};

pub const DOCUMENT_VERSION: u32 = 1;

/// The per-shape color palette, in the old app's order.
pub const PALETTE: [&str; 8] = [
    "#e03131", // red
    "#f08c00", // orange
    "#f8d12c", // yellow
    "#2f9e44", // green
    "#1971c2", // blue
    "#9c36b5", // violet
    "#1a1a1a", // black
    "#868e96", // grey
];

/// Stroke widths in image pixels, old-app S/M/L/XL.
pub const STROKE_WIDTHS: [f32; 4] = [2.0, 4.0, 8.0, 14.0];

/// Default text size in image pixels.
pub const DEFAULT_TEXT_SIZE: f32 = 24.0;

/// Counter badge diameter in image pixels.
pub const COUNTER_DIAMETER: f32 = 32.0;

fn default_counter_diameter() -> f32 {
    COUNTER_DIAMETER
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub version: u32,
    /// The capture-library id whose `image.png` this annotation document
    /// edits. None is allowed for transient/test documents, but persisted
    /// library documents always set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_id: Option<String>,
    pub image_width: u32,
    pub image_height: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<Shape>,
}

impl Document {
    /// An empty document over an image of the given size.
    pub fn new(image_width: u32, image_height: u32) -> Self {
        Document {
            version: DOCUMENT_VERSION,
            image_id: None,
            image_width,
            image_height,
            shapes: Vec::new(),
        }
    }

    /// Parse document bytes. Only version 1 is read — anything else (including
    /// the old tldraw format) is an error, and callers fall back to an empty
    /// document over the image.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let document: Document = serde_json::from_slice(bytes)
            .map_err(|error| format!("Could not parse the annotation document: {error}"))?;
        document.validate()?;
        Ok(document)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        serde_json::to_vec_pretty(self)
            .map_err(|error| format!("Could not encode the annotation document: {error}"))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != DOCUMENT_VERSION {
            return Err(format!(
                "Unsupported annotation document version {} (expected {})",
                self.version, DOCUMENT_VERSION
            ));
        }
        if self.image_width == 0 || self.image_height == 0 {
            return Err("The annotation image size is invalid.".to_string());
        }
        if self.shapes.len() > 10_000 {
            return Err("The annotation document has too many shapes.".to_string());
        }
        if let Some(image_id) = &self.image_id
            && (image_id.is_empty()
                || image_id.len() > 128
                || !image_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
        {
            return Err("The annotation image reference is invalid.".to_string());
        }
        let mut ids = std::collections::HashSet::new();
        for shape in &self.shapes {
            shape.validate()?;
            if !ids.insert(shape.id().clone()) {
                return Err(format!("Duplicate shape id {}.", shape.id()));
            }
        }
        Ok(())
    }

    /// Index of the shape with the given id.
    pub fn index_of(&self, id: &ShapeId) -> Option<usize> {
        self.shapes.iter().position(|shape| shape.id() == id)
    }

    /// Move the shape at `index` one step toward the top (later = drawn on top).
    /// Returns false when it is already topmost or the index is out of range.
    pub fn bring_forward(&mut self, index: usize) -> bool {
        if index + 1 < self.shapes.len() {
            self.shapes.swap(index, index + 1);
            true
        } else {
            false
        }
    }

    /// Move the shape at `index` one step toward the bottom. Returns false
    /// when it is already at the bottom or the index is out of range.
    pub fn send_backward(&mut self, index: usize) -> bool {
        if index < self.shapes.len() && index > 0 {
            self.shapes.swap(index, index - 1);
            true
        } else {
            false
        }
    }

    pub fn delete(&mut self, id: &ShapeId) -> bool {
        let before = self.shapes.len();
        self.shapes.retain(|shape| shape.id() != id);
        self.shapes.len() != before
    }

    /// The number the next counter badge should show: one past the highest
    /// existing badge. Derived, so undoing a counter creation does not skew
    /// the sequence.
    pub fn next_counter_number(&self) -> u64 {
        self.shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Counter(counter) => Some(counter.n),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Crop the document to `rect` (image pixels). Shapes that do not
    /// intersect the rect at all are removed (the old app's crop semantics);
    /// the rest are translated with the image. Returns the number of shapes
    /// removed.
    pub fn crop(&mut self, rect: &Rect) -> usize {
        let before = self.shapes.len();
        self.shapes.retain(|shape| {
            let bounds = shape.bounds();
            bounds.x + bounds.w > rect.x
                && bounds.y + bounds.h > rect.y
                && bounds.x < rect.x + rect.w
                && bounds.y < rect.y + rect.h
        });
        let removed = before - self.shapes.len();
        for shape in &mut self.shapes {
            shape.translate(-rect.x, -rect.y);
        }
        self.image_width = rect.w.round().max(1.0) as u32;
        self.image_height = rect.h.round().max(1.0) as u32;
        removed
    }
}

/// Shape id: stable across edits, opaque to rendering.
pub type ShapeId = String;

/// A rectangle in image pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectShape {
    pub id: ShapeId,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Hex color, one of [`PALETTE`].
    pub color: String,
    /// Stroke width in image pixels.
    pub stroke_width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EllipseShape {
    pub id: ShapeId,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: String,
    pub stroke_width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArrowShape {
    pub id: ShapeId,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub color: String,
    pub stroke_width: f32,
}

impl ArrowShape {
    /// Endpoints normalized so (x1, y1) is the tail and (x2, y2) the head,
    /// with the head occupying at least its own size so it stays grabbable.
    fn normalized(&self) -> ((f32, f32), (f32, f32)) {
        ((self.x1, self.y1), (self.x2, self.y2))
    }

    /// The filled triangular head: base centered on the line, `size` long.
    pub fn head_triangle(&self, size: f32) -> [(f32, f32); 3] {
        let ((x1, y1), (x2, y2)) = self.normalized();
        let (dx, dy) = (x2 - x1, y2 - y1);
        let length = (dx * dx + dy * dy).sqrt();
        if length <= f32::EPSILON {
            return [(x2, y2); 3];
        }
        let (ux, uy) = (dx / length, dy / length); // along the line
        let (px, py) = (-uy, ux); // perpendicular
        // Head base sits back from the tip; the tip overshoots by 40% of the
        // size so the line joint is covered.
        let tip = (x2 + ux * size * 0.4, y2 + uy * size * 0.4);
        let base = (x2, y2);
        let half = size * 0.5;
        [
            tip,
            (base.0 + px * half, base.1 + py * half),
            (base.0 - px * half, base.1 - py * half),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PenShape {
    pub id: ShapeId,
    /// Polyline points in image pixels; at least 2 after creation.
    pub points: Vec<(f32, f32)>,
    pub color: String,
    pub stroke_width: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextShape {
    pub id: ShapeId,
    /// Top-left of the text block in image pixels.
    pub x: f32,
    pub y: f32,
    pub text: String,
    /// Font size in image pixels.
    pub size: f32,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterShape {
    pub id: ShapeId,
    /// Top-left of the badge bounding square in image pixels.
    pub x: f32,
    pub y: f32,
    /// The number this badge displays, 1-based.
    pub n: u64,
    pub color: String,
    #[serde(default = "default_counter_diameter")]
    pub diameter: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectShape {
    pub id: ShapeId,
    pub bounds: Rect,
    pub kind: EffectKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconShape {
    pub id: ShapeId,
    /// Docs saved before the icon-pack purge stored `pack` too; serde ignores
    /// it now, and unknown glyph ids fall back to Star (`deserialize_glyph`).
    #[serde(default, deserialize_with = "deserialize_glyph")]
    pub glyph: IconGlyph,
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: String,
}

/// Old documents may carry glyph ids that are no longer bundled; they render
/// as the closest fallback (Star) instead of failing the whole document.
fn deserialize_glyph<'de, D>(deserializer: D) -> Result<IconGlyph, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let name = String::deserialize(deserializer)?;
    Ok(IconGlyph::ALL
        .iter()
        .copied()
        .find(|glyph| serde_json::to_string(glyph).is_ok_and(|json| json.trim_matches('"') == name))
        .unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EffectKind {
    Blur { radius_px: f32 },
    Pixelate { block_px: u32 },
    Redact,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Shape {
    Rectangle(RectShape),
    Ellipse(EllipseShape),
    Arrow(ArrowShape),
    Pen(PenShape),
    Text(TextShape),
    Counter(CounterShape),
    Effect(EffectShape),
    Icon(IconShape),
}

impl Shape {
    pub fn id(&self) -> &ShapeId {
        match self {
            Shape::Rectangle(shape) => &shape.id,
            Shape::Ellipse(shape) => &shape.id,
            Shape::Arrow(shape) => &shape.id,
            Shape::Pen(shape) => &shape.id,
            Shape::Text(shape) => &shape.id,
            Shape::Counter(shape) => &shape.id,
            Shape::Effect(shape) => &shape.id,
            Shape::Icon(shape) => &shape.id,
        }
    }

    /// The bounding box used for hit-testing, resizing, and crop deletion.
    pub fn bounds(&self) -> Rect {
        match self {
            Shape::Rectangle(shape) => Rect::new(shape.x, shape.y, shape.w, shape.h),
            Shape::Ellipse(shape) => Rect::new(shape.x, shape.y, shape.w, shape.h),
            Shape::Arrow(shape) => {
                let (x1, y1, x2, y2) = (shape.x1, shape.y1, shape.x2, shape.y2);
                let x = x1.min(x2);
                let y = y1.min(y2);
                Rect::new(x, y, (x1 - x2).abs().max(1.0), (y1 - y2).abs().max(1.0))
            }
            Shape::Pen(shape) => {
                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;
                for (x, y) in &shape.points {
                    min_x = min_x.min(*x);
                    min_y = min_y.min(*y);
                    max_x = max_x.max(*x);
                    max_y = max_y.max(*y);
                }
                Rect::new(
                    min_x,
                    min_y,
                    (max_x - min_x).max(1.0),
                    (max_y - min_y).max(1.0),
                )
            }
            Shape::Text(text) => {
                // No font metrics at model level: estimate the rendered block
                // from character counts (0.6 em per char, 1.3 line height).
                let lines = text.text.split('\n').count().max(1) as f32;
                let width = text
                    .text
                    .split('\n')
                    .map(|line| line.chars().count() as f32 * text.size * 0.6)
                    .fold(0.0f32, f32::max);
                Rect::new(text.x, text.y, width.max(1.0), text.size * 1.3 * lines)
            }
            Shape::Counter(shape) => Rect::new(shape.x, shape.y, shape.diameter, shape.diameter),
            Shape::Effect(shape) => shape.bounds,
            Shape::Icon(shape) => Rect::new(shape.x, shape.y, shape.size, shape.size),
        }
    }

    /// Translate the shape by (dx, dy) in image pixels.
    pub fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            Shape::Rectangle(shape) => {
                shape.x += dx;
                shape.y += dy;
            }
            Shape::Ellipse(shape) => {
                shape.x += dx;
                shape.y += dy;
            }
            Shape::Arrow(shape) => {
                shape.x1 += dx;
                shape.y1 += dy;
                shape.x2 += dx;
                shape.y2 += dy;
            }
            Shape::Pen(shape) => {
                for point in &mut shape.points {
                    point.0 += dx;
                    point.1 += dy;
                }
            }
            Shape::Text(shape) => {
                shape.x += dx;
                shape.y += dy;
            }
            Shape::Counter(shape) => {
                shape.x += dx;
                shape.y += dy;
            }
            Shape::Effect(shape) => {
                shape.bounds.x += dx;
                shape.bounds.y += dy;
            }
            Shape::Icon(shape) => {
                shape.x += dx;
                shape.y += dy;
            }
        }
    }

    /// Resize by scaling the shape's geometry from its current bounds onto
    /// `target`. Text scales by height; counter badges retain their circular
    /// shape and scale by the smaller axis.
    pub fn scale_to(&mut self, target: &Rect) {
        let original = self.bounds();
        let sx = if original.w > f32::EPSILON {
            target.w / original.w
        } else {
            1.0
        };
        let sy = if original.h > f32::EPSILON {
            target.h / original.h
        } else {
            1.0
        };
        match self {
            Shape::Rectangle(shape) => {
                shape.x = target.x;
                shape.y = target.y;
                shape.w = target.w.max(1.0);
                shape.h = target.h.max(1.0);
            }
            Shape::Ellipse(shape) => {
                shape.x = target.x;
                shape.y = target.y;
                shape.w = target.w.max(1.0);
                shape.h = target.h.max(1.0);
            }
            Shape::Arrow(shape) => {
                shape.x1 = target.x + (shape.x1 - original.x) * sx;
                shape.y1 = target.y + (shape.y1 - original.y) * sy;
                shape.x2 = target.x + (shape.x2 - original.x) * sx;
                shape.y2 = target.y + (shape.y2 - original.y) * sy;
            }
            Shape::Pen(shape) => {
                for point in &mut shape.points {
                    point.0 = target.x + (point.0 - original.x) * sx;
                    point.1 = target.y + (point.1 - original.y) * sy;
                }
            }
            Shape::Text(shape) => {
                shape.x = target.x;
                shape.y = target.y;
                shape.size = (shape.size * sy).max(1.0);
            }
            Shape::Counter(shape) => {
                shape.x = target.x;
                shape.y = target.y;
                shape.diameter = (shape.diameter * sx.min(sy)).max(8.0);
            }
            Shape::Effect(shape) => {
                shape.bounds = Rect::new(target.x, target.y, target.w.max(1.0), target.h.max(1.0));
            }
            Shape::Icon(shape) => {
                let width_changed = (sx - 1.0).abs() > f32::EPSILON;
                let height_changed = (sy - 1.0).abs() > f32::EPSILON;
                shape.x = target.x;
                shape.y = target.y;
                shape.size = match (width_changed, height_changed) {
                    (true, false) => target.w,
                    (false, true) => target.h,
                    (true, true) => target.w.min(target.h),
                    (false, false) => shape.size,
                }
                .clamp(MIN_ICON_SIZE, MAX_ICON_SIZE);
            }
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.id().is_empty() || self.id().len() > 64 {
            return Err("The shape id is invalid.".to_string());
        }
        if let Shape::Effect(shape) = self {
            let Rect { x, y, w, h } = shape.bounds;
            let bounds_valid = finite(&[x, y, w, h]) && w > 0.0 && h > 0.0;
            let kind_valid = match &shape.kind {
                EffectKind::Blur { radius_px } => {
                    radius_px.is_finite() && (0.1..=128.0).contains(radius_px)
                }
                EffectKind::Pixelate { block_px } => (2..=256).contains(block_px),
                EffectKind::Redact => true,
            };
            return if bounds_valid && kind_valid {
                Ok(())
            } else {
                Err(format!("The geometry for shape {} is invalid.", self.id()))
            };
        }
        if let Shape::Icon(shape) = self {
            let geometry_valid = finite(&[shape.x, shape.y, shape.size])
                && (MIN_ICON_SIZE..=MAX_ICON_SIZE).contains(&shape.size);
            return if geometry_valid {
                validate_color(&shape.color)
            } else {
                Err(format!("The geometry for shape {} is invalid.", self.id()))
            };
        }
        let (color, geometry_valid) = match self {
            Shape::Rectangle(shape) => (
                &shape.color,
                finite(&[shape.x, shape.y, shape.w, shape.h, shape.stroke_width])
                    && shape.w >= 0.0
                    && shape.h >= 0.0
                    && valid_stroke(shape.stroke_width),
            ),
            Shape::Ellipse(shape) => (
                &shape.color,
                finite(&[shape.x, shape.y, shape.w, shape.h, shape.stroke_width])
                    && shape.w >= 0.0
                    && shape.h >= 0.0
                    && valid_stroke(shape.stroke_width),
            ),
            Shape::Arrow(shape) => (
                &shape.color,
                finite(&[shape.x1, shape.y1, shape.x2, shape.y2, shape.stroke_width])
                    && valid_stroke(shape.stroke_width),
            ),
            Shape::Pen(shape) => (
                &shape.color,
                shape.points.len() >= 2
                    && shape.points.len() <= 100_000
                    && shape.points.iter().all(|point| finite(&[point.0, point.1]))
                    && valid_stroke(shape.stroke_width),
            ),
            Shape::Text(shape) => (
                &shape.color,
                finite(&[shape.x, shape.y, shape.size])
                    && shape.size > 0.0
                    && shape.size <= 100_000.0
                    && shape.text.len() <= 1_000_000,
            ),
            Shape::Counter(shape) => (
                &shape.color,
                finite(&[shape.x, shape.y, shape.diameter])
                    && shape.diameter >= 8.0
                    && shape.diameter <= 100_000.0
                    && shape.n > 0,
            ),
            Shape::Effect(_) => unreachable!("effects are validated without a color"),
            Shape::Icon(_) => unreachable!("icons are validated separately"),
        };
        if !geometry_valid {
            return Err(format!("The geometry for shape {} is invalid.", self.id()));
        }
        validate_color(color)
    }
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn valid_stroke(width: f32) -> bool {
    width > 0.0 && width <= 100_000.0
}

fn validate_color(color: &str) -> Result<(), String> {
    let valid = color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(format!("The shape color {color:?} is invalid."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_shape(id: &str, x: f32, y: f32, w: f32, h: f32) -> Shape {
        Shape::Rectangle(RectShape {
            id: id.to_string(),
            x,
            y,
            w,
            h,
            color: "#e03131".to_string(),
            stroke_width: 4.0,
        })
    }

    fn sample_document() -> Document {
        let mut document = Document::new(800, 600);
        document
            .shapes
            .push(rect_shape("a", 10.0, 10.0, 100.0, 50.0));
        document.shapes.push(Shape::Arrow(ArrowShape {
            id: "b".to_string(),
            x1: 200.0,
            y1: 100.0,
            x2: 300.0,
            y2: 200.0,
            color: "#1971c2".to_string(),
            stroke_width: 4.0,
        }));
        document.shapes.push(Shape::Pen(PenShape {
            id: "c".to_string(),
            points: vec![(50.0, 50.0), (60.0, 55.0), (70.0, 50.0)],
            color: "#1a1a1a".to_string(),
            stroke_width: 2.0,
        }));
        document
    }

    #[test]
    fn document_round_trips_through_json() {
        let mut document = sample_document();
        document.image_id = Some("capture-42".to_string());
        let bytes = document.to_bytes().unwrap();
        let parsed = Document::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, document);
    }

    #[test]
    fn documents_without_shapes_skip_the_field_but_still_parse() {
        let document = Document::new(100, 100);
        let bytes = serde_json::to_vec(&document).unwrap();
        assert!(!bytes.windows(6).any(|window| window == b"shapes"));
        let parsed = Document::from_bytes(&bytes).unwrap();
        assert!(parsed.shapes.is_empty());
    }

    #[test]
    fn foreign_document_versions_are_rejected() {
        let bytes = br#"{"version": 99, "imageWidth": 10, "imageHeight": 10, "shapes": []}"#;
        assert!(Document::from_bytes(bytes).is_err());
    }

    #[test]
    fn tldraw_style_documents_are_rejected_not_partially_parsed() {
        // The old tldraw format has a completely different schema; it must
        // never silently load as an empty document.
        let bytes = br#"{"schema": {}, "records": [], "version": 1}"#;
        assert!(Document::from_bytes(bytes).is_err());
    }

    #[test]
    fn invalid_colors_are_rejected_at_parse_time() {
        let bytes = br#"{
            "version": 1,
            "imageWidth": 100,
            "imageHeight": 100,
            "shapes": [
                {"type": "rectangle", "id": "a", "x": 0, "y": 0, "w": 10, "h": 10,
                 "color": "red", "strokeWidth": 4}
            ]
        }"#;
        assert!(Document::from_bytes(bytes).is_err());
    }

    #[test]
    fn invalid_shape_geometry_is_rejected_at_parse_time() {
        let bytes = br##"{
            "version": 1,
            "imageId": "capture-1",
            "imageWidth": 100,
            "imageHeight": 100,
            "shapes": [
                {"type": "rectangle", "id": "r", "x": 0, "y": 0, "w": -10, "h": 10,
                 "color": "#e03131", "strokeWidth": 4}
            ]
        }"##;
        assert!(Document::from_bytes(bytes).is_err());
    }

    #[test]
    fn duplicate_shape_ids_are_rejected() {
        let mut document = Document::new(100, 100);
        document.shapes.push(rect_shape("a", 0.0, 0.0, 10.0, 10.0));
        document.shapes.push(rect_shape("a", 20.0, 0.0, 10.0, 10.0));
        assert!(document.validate().is_err());
    }

    #[test]
    fn z_order_moves_shapes_one_step() {
        let mut document = sample_document();
        assert!(document.bring_forward(0));
        assert_eq!(document.shapes[1].id(), "a");
        assert!(
            !document.bring_forward(document.shapes.len() - 1),
            "topmost cannot go forward"
        );
        assert!(document.send_backward(1));
        assert_eq!(document.shapes[0].id(), "a");
        assert!(!document.send_backward(0), "bottom cannot go back");
    }

    #[test]
    fn delete_removes_only_the_target_shape() {
        let mut document = sample_document();
        assert!(document.delete(&"b".to_string()));
        assert_eq!(
            document
                .shapes
                .iter()
                .map(|s| s.id().as_str())
                .collect::<Vec<_>>(),
            ["a", "c"]
        );
        assert!(!document.delete(&"b".to_string()));
    }

    #[test]
    fn counter_numbers_derive_from_the_existing_max() {
        let mut document = Document::new(100, 100);
        assert_eq!(document.next_counter_number(), 1);
        document.shapes.push(Shape::Counter(CounterShape {
            id: "c1".to_string(),
            x: 0.0,
            y: 0.0,
            n: 3,
            color: "#e03131".to_string(),
            diameter: COUNTER_DIAMETER,
        }));
        assert_eq!(document.next_counter_number(), 4);
    }

    #[test]
    fn crop_removes_shapes_outside_and_shifts_the_rest() {
        let mut document = sample_document();
        // "a" (10..110 x, 10..60 y) is inside; "b" (200..300) is outside;
        // "c" (50..70 x, 50..55 y) intersects.
        let removed = document.crop(&Rect::new(0.0, 0.0, 150.0, 100.0));
        assert_eq!(removed, 1);
        assert_eq!(document.shapes.len(), 2);
        assert_eq!(document.image_width, 150);
        assert_eq!(document.image_height, 100);
        let a = &document.shapes[0];
        assert_eq!(a.bounds(), Rect::new(10.0, 10.0, 100.0, 50.0));
        let c = &document.shapes[1];
        match c {
            Shape::Pen(pen) => assert_eq!(pen.points[0], (50.0, 50.0)),
            other => panic!("expected the pen, got {other:?}"),
        }
    }

    #[test]
    fn crop_touching_only_an_edge_counts_as_outside() {
        // The old app deletes shapes whose bounds merely touch the crop edge.
        let mut document = Document::new(200, 200);
        document
            .shapes
            .push(rect_shape("edge", 100.0, 50.0, 50.0, 50.0));
        document.crop(&Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(document.shapes.is_empty());
    }

    #[test]
    fn crop_clamps_to_at_least_one_pixel() {
        let mut document = Document::new(100, 100);
        document.crop(&Rect::new(0.0, 0.0, 0.5, 0.5));
        assert_eq!((document.image_width, document.image_height), (1, 1));
    }

    #[test]
    fn scale_to_resizes_geometry_proportionally() {
        let mut document = sample_document();
        let arrow_bounds = document.shapes[1].bounds();
        assert_eq!(arrow_bounds, Rect::new(200.0, 100.0, 100.0, 100.0));

        let target = Rect::new(0.0, 0.0, 200.0, 300.0);
        document.shapes[1].scale_to(&target);
        match &document.shapes[1] {
            Shape::Arrow(arrow) => {
                assert_eq!((arrow.x1, arrow.y1), (0.0, 0.0));
                assert_eq!((arrow.x2, arrow.y2), (200.0, 300.0));
            }
            other => panic!("expected the arrow, got {other:?}"),
        }
    }

    #[test]
    fn resizing_text_and_counters_scales_their_visible_geometry() {
        let mut text = Shape::Text(TextShape {
            id: "t".to_string(),
            x: 10.0,
            y: 10.0,
            text: "text".to_string(),
            size: 20.0,
            color: "#1a1a1a".to_string(),
        });
        text.scale_to(&Rect::new(20.0, 30.0, 100.0, 52.0));
        match text {
            Shape::Text(text) => {
                assert_eq!((text.x, text.y, text.size), (20.0, 30.0, 40.0));
            }
            other => panic!("expected text, got {other:?}"),
        }

        let mut counter = Shape::Counter(CounterShape {
            id: "counter".to_string(),
            x: 10.0,
            y: 10.0,
            n: 1,
            color: "#e03131".to_string(),
            diameter: 32.0,
        });
        counter.scale_to(&Rect::new(20.0, 30.0, 64.0, 48.0));
        match counter {
            Shape::Counter(counter) => {
                assert_eq!((counter.x, counter.y, counter.diameter), (20.0, 30.0, 48.0));
            }
            other => panic!("expected counter, got {other:?}"),
        }
    }

    #[test]
    fn arrow_head_points_back_along_the_line() {
        let arrow = ArrowShape {
            id: "arrow".to_string(),
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 0.0,
            color: "#000000".to_string(),
            stroke_width: 4.0,
        };
        let head = arrow.head_triangle(20.0);
        // Tip overshoots the endpoint along the line; the base straddles it.
        assert!(head[0].0 > 100.0);
        assert_eq!(head[1].0, 100.0);
        assert_eq!(head[2].0, 100.0);
        assert!(
            (head[1].1 - head[2].1).abs() >= 19.0,
            "base spans the head size"
        );
    }

    #[test]
    fn degenerate_arrow_head_collapses_to_a_point() {
        let arrow = ArrowShape {
            id: "arrow".to_string(),
            x1: 5.0,
            y1: 5.0,
            x2: 5.0,
            y2: 5.0,
            color: "#000000".to_string(),
            stroke_width: 4.0,
        };
        assert_eq!(arrow.head_triangle(20.0), [(5.0, 5.0); 3]);
    }

    #[test]
    fn text_bounds_are_not_zero() {
        let text = Shape::Text(TextShape {
            id: "t".to_string(),
            x: 10.0,
            y: 10.0,
            text: "hello".to_string(),
            size: 24.0,
            color: "#000000".to_string(),
        });
        let bounds = text.bounds();
        assert!(bounds.h > 0.0);
    }

    #[test]
    fn effect_shapes_round_trip_and_resize_as_boxes() {
        let mut document = Document::new(100, 80);
        document.shapes.push(Shape::Effect(EffectShape {
            id: "blur".to_string(),
            bounds: Rect::new(10.0, 20.0, 30.0, 40.0),
            kind: EffectKind::Blur { radius_px: 14.0 },
        }));
        assert_eq!(
            Document::from_bytes(&document.to_bytes().unwrap()).unwrap(),
            document
        );

        let shape = &mut document.shapes[0];
        shape.translate(5.0, -5.0);
        shape.scale_to(&Rect::new(20.0, 30.0, 60.0, 50.0));
        assert_eq!(shape.bounds(), Rect::new(20.0, 30.0, 60.0, 50.0));
    }

    #[test]
    fn effect_parameters_are_validated_at_document_load() {
        for kind in [
            EffectKind::Blur { radius_px: 0.0 },
            EffectKind::Pixelate { block_px: 1 },
        ] {
            let mut document = Document::new(100, 80);
            document.shapes.push(Shape::Effect(EffectShape {
                id: "effect".to_string(),
                bounds: Rect::new(0.0, 0.0, 10.0, 10.0),
                kind,
            }));
            assert!(document.to_bytes().is_err());
        }
    }

    #[test]
    fn icon_shapes_round_trip_and_resize_without_distortion() {
        let mut document = Document::new(100, 80);
        document.shapes.push(Shape::Icon(IconShape {
            id: "icon".to_string(),
            glyph: IconGlyph::ArrowRight,
            x: 10.0,
            y: 20.0,
            size: 32.0,
            color: "#1971c2".to_string(),
        }));
        assert_eq!(
            Document::from_bytes(&document.to_bytes().unwrap()).unwrap(),
            document
        );

        document.shapes[0].scale_to(&Rect::new(10.0, 20.0, 64.0, 32.0));
        assert_eq!(
            document.shapes[0].bounds(),
            Rect::new(10.0, 20.0, 64.0, 64.0)
        );
        document.shapes[0].scale_to(&Rect::new(10.0, 20.0, 64.0, 48.0));
        assert_eq!(
            document.shapes[0].bounds(),
            Rect::new(10.0, 20.0, 48.0, 48.0)
        );
    }

    #[test]
    fn resizing_an_icon_beyond_its_raster_limit_keeps_the_document_valid() {
        let mut document = Document::new(2400, 1600);
        document.shapes.push(Shape::Icon(IconShape {
            id: "icon".to_string(),
            glyph: IconGlyph::ArrowRight,
            x: 10.0,
            y: 20.0,
            size: 32.0,
            color: "#1971c2".to_string(),
        }));

        document.shapes[0].scale_to(&Rect::new(10.0, 20.0, 2048.0, 1600.0));

        let bytes = document
            .to_bytes()
            .expect("a resized icon should remain a valid annotation");
        let restored = Document::from_bytes(&bytes).unwrap();
        assert_eq!(
            restored.shapes[0].bounds(),
            Rect::new(10.0, 20.0, 1024.0, 1024.0)
        );
    }

    #[test]
    fn legacy_icon_shapes_parse_without_the_pack_and_fallback_to_star() {
        for icon in [
            // Old documents: pack field present, glyph still bundled.
            r##"{"type":"icon","id":"icon","pack":"legacy","glyph":"arrow-right","x":0,"y":0,"size":32,"color":"#1971c2"}"##,
            // Unknown glyph id from a newer or corrupted document: Star.
            r##"{"type":"icon","id":"icon","glyph":"not-bundled","x":0,"y":0,"size":32,"color":"#1971c2"}"##,
            // Missing glyph entirely: Star.
            r##"{"type":"icon","id":"icon","x":0,"y":0,"size":32,"color":"#1971c2"}"##,
        ] {
            let bytes =
                format!(r#"{{"version":1,"imageWidth":10,"imageHeight":10,"shapes":[{icon}]}}"#);
            let document = Document::from_bytes(bytes.as_bytes()).unwrap();
            let Shape::Icon(shape) = &document.shapes[0] else {
                panic!("expected an icon shape");
            };
            let expected = if icon.contains("arrow-right") {
                IconGlyph::ArrowRight
            } else {
                IconGlyph::Star
            };
            assert_eq!(shape.glyph, expected);
        }
    }
}
