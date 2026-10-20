//! The draw-op engine: turn a shape with its style options into a
//! deterministic list of pure-geometry draw operations that BOTH renderers
//! (the GPUI canvas and the tiny-skia flatten pipeline) consume, so the
//! preview and the export always agree.
//!
//! Everything here is seeded from the shape id: the same element rasterizes
//! identically across renders, across processes, and in the flattened PNG.
//!
//! Ports the roughjs algorithm family that Excalidraw uses:
//! - sloppiness (roughjs `roughness`): every edge is drawn as two slightly
//!   different wobbly polylines; higher levels wobble more.
//! - hachure / cross-hatch fills (roughjs fillers): parallel scanlines across
//!   the shape at a fixed angle, drawn as stroked lines.
//! - dash styles: the outline polyline is split into dash segments here, in
//!   geometry, so neither renderer needs dash support.

use super::model::{
    ArrowShape, DiamondShape, EllipseShape, FillStyle, Rect, RectShape, Shape, StrokeStyle,
};

/// One renderer-agnostic draw operation, in image pixels.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawOp {
    /// Fill a closed polygon with a solid color.
    FillPolygon {
        points: Vec<(f32, f32)>,
        color: (u8, u8, u8),
        opacity: f32,
    },
    /// Stroke an (already dash-split) polyline or closed polygon outline.
    StrokePolyline {
        points: Vec<(f32, f32)>,
        closed: bool,
        color: (u8, u8, u8),
        width: f32,
        opacity: f32,
        /// Round caps/joins (dotted strokes).
        round_cap: bool,
    },
    /// Composite a bitmap into a destination rect (image elements).
    Image {
        asset_id: String,
        bounds: Rect,
        opacity: f32,
    },
    /// A shape the ops engine renders opaquely by another mechanism
    /// (text divs, counters, effects) — the renderer falls back to its own
    /// painter. Never emitted for shapes with style options.
    Opaque,
}

/// A seedable 64-bit PRNG (SplitMix64): small, deterministic, and good enough
/// for wobble. The same seed always yields the same sequence.
pub struct Rng(u64);

impl Rng {
    /// Seed derived from a shape id, so rough geometry is stable per element
    /// without storing a seed in the document.
    pub fn from_shape_id(id: &str) -> Self {
        // FNV-1a 64, on the raw bytes of the id.
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in id.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Rng(hash ^ 0x9e37_79b9_7f4a_7c15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_8ce4_e659);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in [-1, 1).
    pub fn unit(&mut self) -> f32 {
        let value = (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
        (2.0 * value - 1.0) as f32
    }
}

/// How strongly edges wobble: roughjs `roughness` 0/1/2, Excalidraw's three
/// sloppiness levels.
const ROUGHNESS_OFFSET: [f32; 3] = [0.0, 1.0, 2.0];

/// roughjs `offsetOpt`: a random offset bounded by `roughness * max`.
fn offset_opt(rng: &mut Rng, level: usize, max: f32) -> f32 {
    let scale = ROUGHNESS_OFFSET[level.min(2)] * max;
    rng.unit() * scale
}

/// The number of segments a rough curve edge is sampled into.
const CURVE_SEGMENTS: usize = 12;

/// roughjs `_line`-style edge: a wobbly polyline between two points. When
/// `double` (roughjs draws every edge twice at roughness > 0), the caller
/// invokes this twice with independent randomness.
fn rough_edge(rng: &mut Rng, start: (f32, f32), end: (f32, f32), level: usize) -> Vec<(f32, f32)> {
    if level == 0 {
        return vec![start, end];
    }
    let (x1, y1) = start;
    let (x2, y2) = end;
    let length = f32::hypot(x2 - x1, y2 - y1);
    if length <= f32::EPSILON {
        return vec![start, end];
    }
    // Endpoints wobble along both axes, control point lifts perpendicular.
    let offset = |rng: &mut Rng| offset_opt(rng, level, 2.0);
    let p1 = (x1 + offset(rng), y1 + offset(rng));
    let p2 = (x2 + offset(rng), y2 + offset(rng));
    // A quadratic control point near the middle, pushed sideways.
    let control = (
        0.5 * (p1.0 + p2.0) + offset(rng) * length * 0.05,
        0.5 * (p1.1 + p2.1) + offset(rng) * length * 0.05,
    );
    let mut points = Vec::with_capacity(CURVE_SEGMENTS + 1);
    for step in 0..=CURVE_SEGMENTS {
        let t = step as f32 / CURVE_SEGMENTS as f32;
        let mt = 1.0 - t;
        points.push((
            mt * mt * p1.0 + 2.0 * mt * t * control.0 + t * t * p2.0,
            mt * mt * p1.1 + 2.0 * mt * t * control.1 + t * t * p2.1,
        ));
    }
    points
}

/// A wobbly closed polygon: every edge drawn twice (two passes) at roughness
/// > 0, like roughjs.
fn rough_polygon(rng: &mut Rng, vertices: &[(f32, f32)], level: usize) -> Vec<Vec<(f32, f32)>> {
    if level == 0 || vertices.len() < 2 {
        return vec![vertices.to_vec()];
    }
    let mut passes = Vec::new();
    for _ in 0..2 {
        let mut points = Vec::new();
        for index in 0..vertices.len() {
            let start = vertices[index];
            let end = vertices[(index + 1) % vertices.len()];
            let edge = rough_edge(rng, start, end, level);
            points.extend(edge.into_iter().skip(usize::from(!points.is_empty())));
        }
        points.pop(); // the closing edge returns to the first vertex
        passes.push(points);
    }
    passes
}

/// A wobbly ellipse: perturb the radius per sample, two passes at roughness.
fn rough_ellipse(
    rng: &mut Rng,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    level: usize,
) -> Vec<Vec<(f32, f32)>> {
    const SEGMENTS: usize = 48;
    if level == 0 {
        let mut points = Vec::with_capacity(SEGMENTS);
        for step in 0..SEGMENTS {
            let angle = step as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            points.push((cx + angle.cos() * rx, cy + angle.sin() * ry));
        }
        return vec![points];
    }
    let mut passes = Vec::new();
    for _ in 0..2 {
        let mut points = Vec::with_capacity(SEGMENTS);
        for step in 0..SEGMENTS {
            let angle = step as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            let wobble = 1.0 + offset_opt(rng, level, 0.03);
            points.push((
                cx + angle.cos() * rx * wobble,
                cy + angle.sin() * ry * wobble,
            ));
        }
        passes.push(points);
    }
    passes
}

/// Split a polyline into dash segments (geometry-level dashing). `dash` is
/// (mark, gap) in image pixels; a closed outline loops back to the start.
fn dash_polyline(points: &[(f32, f32)], closed: bool, mark: f32, gap: f32) -> Vec<Vec<(f32, f32)>> {
    if points.len() < 2 || mark <= f32::EPSILON {
        return vec![points.to_vec()];
    }
    let mut segments = Vec::new();
    let mut current: Vec<(f32, f32)> = Vec::new();
    let mut remaining = mark;
    let mut drawing = true;
    let mut previous = points[0];
    let mut count = points.len() - 1;
    if closed {
        count += 1;
    }
    for index in 0..count {
        let next = points[(index + 1) % points.len()];
        let mut segment_start = previous;
        loop {
            let dx = next.0 - segment_start.0;
            let dy = next.1 - segment_start.1;
            let length = f32::hypot(dx, dy);
            if length <= remaining {
                if drawing {
                    if current.is_empty() {
                        current.push(segment_start);
                    }
                    current.push(next);
                }
                remaining -= length;
                break;
            }
            let t = remaining / length;
            let split = (segment_start.0 + dx * t, segment_start.1 + dy * t);
            if drawing {
                if current.is_empty() {
                    current.push(segment_start);
                }
                current.push(split);
                if current.len() > 1 {
                    segments.push(std::mem::take(&mut current));
                }
                current = Vec::new();
            }
            segment_start = split;
            drawing = !drawing;
            remaining = if drawing { mark } else { gap };
        }
        previous = next;
    }
    if current.len() > 1 {
        segments.push(current);
    }
    segments
}

fn dash_pattern(style: StrokeStyle, stroke_width: f32) -> Option<(f32, f32)> {
    let width = stroke_width.max(1.0);
    match style {
        StrokeStyle::Solid => None,
        StrokeStyle::Dashed => Some((width * 3.0, width * 2.0)),
        StrokeStyle::Dotted => Some((0.1, width * 1.6)),
    }
}

/// Hatch fill lines across a closed polygon: roughjs's HachureFiller.
/// Lines run at `angle` degrees, spaced `gap` apart, with per-line random
/// jitter seeded by the element. Cross-hatch adds a second set at +90°.
fn hachure_lines(vertices: &[(f32, f32)], angle_degrees: f32, gap: f32) -> Vec<Vec<(f32, f32)>> {
    if vertices.len() < 3 || gap <= f32::EPSILON {
        return Vec::new();
    }
    let bounds = polygon_bounds(vertices);
    let cx = (bounds.0 + bounds.2) / 2.0;
    let cy = (bounds.1 + bounds.3) / 2.0;
    let theta = -angle_degrees.to_radians();
    let (sin, cos) = theta.sin_cos();
    let rotate = |point: (f32, f32)| {
        let dx = point.0 - cx;
        let dy = point.1 - cy;
        (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
    };
    let rotated: Vec<(f32, f32)> = vertices.iter().map(|point| rotate(*point)).collect();
    let rotated_bounds = polygon_bounds(&rotated);

    let mut lines = Vec::new();
    let mut y = rotated_bounds.1 - gap;
    while y <= rotated_bounds.3 + gap {
        let mut crossings: Vec<f32> = Vec::new();
        for index in 0..rotated.len() {
            let (a, b) = (rotated[index], rotated[(index + 1) % rotated.len()]);
            if (a.1 <= y && b.1 > y) || (a.1 > y && b.1 <= y) {
                let t = (y - a.1) / (b.1 - a.1);
                crossings.push(a.0 + t * (b.0 - a.0));
            }
        }
        crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in crossings.chunks(2) {
            if pair.len() == 2 {
                // Un-rotate the segment back to image space.
                let left = (pair[0], y);
                let right = (pair[1], y);
                let unrotate = |point: (f32, f32)| {
                    let dx = point.0 - cx;
                    let dy = point.1 - cy;
                    (cx + dx * cos + dy * sin, cy - dx * sin + dy * cos)
                };
                lines.push(vec![unrotate(left), unrotate(right)]);
            }
        }
        y += gap;
    }
    lines
}

fn polygon_bounds(vertices: &[(f32, f32)]) -> (f32, f32, f32, f32) {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for (x, y) in vertices {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x);
        max_y = max_y.max(*y);
    }
    (min_x, min_y, max_x, max_y)
}

/// Hatch parameters for a shape: Excalidraw's defaults are a -41° angle and a
/// gap that scales with the stroke weight.
fn hatch_style(fill: FillStyle, stroke_width: f32) -> Option<(f32, f32)> {
    match fill {
        FillStyle::Hachure => Some((-41.0, (stroke_width * 2.0).max(4.0))),
        FillStyle::CrossHatch => Some((-41.0, (stroke_width * 2.0).max(4.0))),
        FillStyle::None | FillStyle::Solid => None,
    }
}

fn hex_color(hex: &str) -> (u8, u8, u8) {
    let bytes = hex.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return (0, 0, 0);
    }
    let channel = |pair: &[u8]| {
        u8::from_str_radix(std::str::from_utf8(pair).unwrap_or("00"), 16).unwrap_or(0)
    };
    (
        channel(&bytes[1..3]),
        channel(&bytes[3..5]),
        channel(&bytes[5..7]),
    )
}

/// Rectangle vertices. Rounded corners are approximated with quarter-circle
/// point fans (cheap, deterministic, and both renderers draw the same
/// geometry). Sharp corners produce exactly four vertices.
fn rectangle_vertices(rect: &RectShape, radius: f32) -> Vec<(f32, f32)> {
    let radius = radius.clamp(0.0, rect.w.min(rect.h) / 2.0);
    if radius <= 0.1 {
        return vec![
            (rect.x, rect.y),
            (rect.x + rect.w, rect.y),
            (rect.x + rect.w, rect.y + rect.h),
            (rect.x, rect.y + rect.h),
        ];
    }
    // (corner-circle center, sweep start) per corner, tracing clockwise from
    // after the top edge: top-right, bottom-right, bottom-left, top-left.
    const CORNER_POINTS: usize = 4;
    let corners = [
        (
            (rect.x + rect.w - radius, rect.y + radius),
            -std::f32::consts::FRAC_PI_2,
        ),
        ((rect.x + rect.w - radius, rect.y + rect.h - radius), 0.0),
        (
            (rect.x + radius, rect.y + rect.h - radius),
            std::f32::consts::FRAC_PI_2,
        ),
        ((rect.x + radius, rect.y + radius), std::f32::consts::PI),
    ];
    let mut points = Vec::with_capacity(4 * (CORNER_POINTS + 1));
    for (center, start) in corners {
        for step in 0..=CORNER_POINTS {
            let t = step as f32 / CORNER_POINTS as f32;
            let angle = start + t * std::f32::consts::FRAC_PI_2;
            points.push((
                center.0 + angle.cos() * radius,
                center.1 + angle.sin() * radius,
            ));
        }
    }
    points.pop(); // each fan ends where the next begins
    points
}

/// Diamond vertices (with optional rounded corners via the same chamfer
/// trick, applied to the diamond's four corners).
fn diamond_vertices(shape: &DiamondShape, round: bool) -> Vec<(f32, f32)> {
    let cx = shape.x + shape.w / 2.0;
    let cy = shape.y + shape.h / 2.0;
    let mut vertices = vec![
        (cx, shape.y),
        (shape.x + shape.w, cy),
        (cx, shape.y + shape.h),
        (shape.x, cy),
    ];
    if round {
        // Cut each corner toward its neighbors by 20% of the edge length,
        // emulating Excalidraw's rounded diamond.
        let mut rounded = Vec::with_capacity(8);
        for index in 0..vertices.len() {
            let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
            let current = vertices[index];
            let next = vertices[(index + 1) % vertices.len()];
            let toward = |from: (f32, f32), to: (f32, f32), t: f32| {
                (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
            };
            rounded.push(toward(current, previous, 0.2));
            rounded.push(toward(current, next, 0.2));
        }
        vertices = rounded;
    }
    vertices
}

/// Compile a shape into renderer-agnostic draw ops. `shape` is any shape; the
/// `id` seeds roughness. Emitted in paint order: fill first, outline second.
pub fn shape_ops(shape: &Shape) -> Vec<DrawOp> {
    match shape {
        Shape::Rectangle(shape) => polygon_shape_ops(
            shape.id.as_str(),
            &rectangle_vertices(
                shape,
                if shape.style.edges.is_round() {
                    shape.corner_radius()
                } else {
                    0.0
                },
            ),
            &shape.color,
            shape.style.fill_color.as_deref(),
            shape.style.fill_style,
            shape.style.stroke_style,
            shape.stroke_width,
            shape.style.sloppiness,
            shape.style.opacity,
        ),
        Shape::Diamond(shape) => polygon_shape_ops(
            shape.id.as_str(),
            &diamond_vertices(shape, shape.style.edges.is_round()),
            &shape.color,
            shape.style.fill_color.as_deref(),
            shape.style.fill_style,
            shape.style.stroke_style,
            shape.stroke_width,
            shape.style.sloppiness,
            shape.style.opacity,
        ),
        Shape::Ellipse(shape) => ellipse_shape_ops(shape),
        Shape::Line(shape) => {
            let stroke = hex_color(&shape.color);
            let opacity = shape.style.opacity.alpha();
            let level = shape.style.sloppiness.level();
            let mut rng = Rng::from_shape_id(&shape.id);
            let mut ops = Vec::new();
            let passes = if level == 0 { 1 } else { 2 };
            for _ in 0..passes {
                let edge = rough_edge(&mut rng, (shape.x1, shape.y1), (shape.x2, shape.y2), level);
                for segment in
                    dashed_or_solid(&edge, false, shape.style.stroke_style, shape.stroke_width)
                {
                    ops.push(DrawOp::StrokePolyline {
                        points: segment,
                        closed: false,
                        color: stroke,
                        width: shape.stroke_width,
                        opacity,
                        round_cap: shape.style.stroke_style == StrokeStyle::Dotted,
                    });
                }
            }
            ops
        }
        Shape::Arrow(shape) => arrow_ops(shape),
        Shape::Pen(pen) => {
            vec![DrawOp::StrokePolyline {
                points: pen.points.clone(),
                closed: false,
                color: hex_color(&pen.color),
                width: pen.stroke_width,
                opacity: pen.style.opacity.alpha(),
                round_cap: true,
            }]
        }
        Shape::Image(shape) => vec![DrawOp::Image {
            asset_id: shape.asset_id.clone(),
            bounds: shape.bounds,
            opacity: shape.style.opacity.alpha(),
        }],
        _ => vec![DrawOp::Opaque],
    }
}

fn dashed_or_solid(
    points: &[(f32, f32)],
    closed: bool,
    style: StrokeStyle,
    stroke_width: f32,
) -> Vec<Vec<(f32, f32)>> {
    match dash_pattern(style, stroke_width) {
        Some((mark, gap)) => dash_polyline(points, closed, mark, gap),
        None => vec![points.to_vec()],
    }
}

#[allow(clippy::too_many_arguments)]
fn polygon_shape_ops(
    id: &str,
    vertices: &[(f32, f32)],
    stroke_color: &str,
    fill_color: Option<&str>,
    fill_style: FillStyle,
    stroke_style: StrokeStyle,
    stroke_width: f32,
    sloppiness: super::model::Sloppiness,
    opacity: super::model::Opacity,
) -> Vec<DrawOp> {
    let mut ops = Vec::new();
    let level = sloppiness.level();
    let mut rng = Rng::from_shape_id(id);

    // Fill.
    if let Some(fill_hex) = fill_color {
        match fill_style {
            FillStyle::Solid => ops.push(DrawOp::FillPolygon {
                points: vertices.to_vec(),
                color: hex_color(fill_hex),
                opacity: opacity.alpha(),
            }),
            FillStyle::Hachure | FillStyle::CrossHatch => {
                if let Some((angle, gap)) = hatch_style(fill_style, stroke_width) {
                    let color = hex_color(fill_hex);
                    for line in hachure_lines(vertices, angle, gap) {
                        ops.push(DrawOp::StrokePolyline {
                            points: line,
                            closed: false,
                            color,
                            width: stroke_width.max(1.0) * 0.75,
                            opacity: opacity.alpha(),
                            round_cap: false,
                        });
                    }
                    if fill_style == FillStyle::CrossHatch {
                        for line in hachure_lines(vertices, angle + 90.0, gap) {
                            ops.push(DrawOp::StrokePolyline {
                                points: line,
                                closed: false,
                                color,
                                width: stroke_width.max(1.0) * 0.75,
                                opacity: opacity.alpha(),
                                round_cap: false,
                            });
                        }
                    }
                }
            }
            FillStyle::None => {}
        }
    }

    // Outline: two wobbly passes at roughness > 0.
    let passes = rough_polygon(&mut rng, vertices, level);
    for pass in passes {
        for segment in dashed_or_solid(&pass, true, stroke_style, stroke_width) {
            ops.push(DrawOp::StrokePolyline {
                points: segment,
                closed: false,
                color: hex_color(stroke_color),
                width: stroke_width,
                opacity: opacity.alpha(),
                round_cap: stroke_style == StrokeStyle::Dotted,
            });
        }
    }
    ops
}

fn ellipse_shape_ops(shape: &EllipseShape) -> Vec<DrawOp> {
    let mut ops = Vec::new();
    let level = shape.style.sloppiness.level();
    let mut rng = Rng::from_shape_id(&shape.id);
    let cx = shape.x + shape.w / 2.0;
    let cy = shape.y + shape.h / 2.0;
    let rx = (shape.w / 2.0).max(0.5);
    let ry = (shape.h / 2.0).max(0.5);

    if let Some(fill_hex) = &shape.style.fill_color
        && shape.style.fill_style != FillStyle::None
    {
        let fill_color = hex_color(fill_hex);
        let hatch = shape.style.fill_style == FillStyle::Hachure
            || shape.style.fill_style == FillStyle::CrossHatch;
        if shape.style.fill_style == FillStyle::Solid {
            ops.push(DrawOp::FillPolygon {
                points: ellipse_vertices(cx, cy, rx, ry),
                color: fill_color,
                opacity: shape.style.opacity.alpha(),
            });
        } else if hatch
            && let Some((angle, gap)) = hatch_style(shape.style.fill_style, shape.stroke_width)
        {
            {
                let outline = ellipse_vertices(cx, cy, rx, ry);
                for line in hachure_lines(&outline, angle, gap) {
                    ops.push(DrawOp::StrokePolyline {
                        points: line,
                        closed: false,
                        color: fill_color,
                        width: shape.stroke_width.max(1.0) * 0.75,
                        opacity: shape.style.opacity.alpha(),
                        round_cap: false,
                    });
                }
                if shape.style.fill_style == FillStyle::CrossHatch {
                    for line in hachure_lines(&outline, angle + 90.0, gap) {
                        ops.push(DrawOp::StrokePolyline {
                            points: line,
                            closed: false,
                            color: fill_color,
                            width: shape.stroke_width.max(1.0) * 0.75,
                            opacity: shape.style.opacity.alpha(),
                            round_cap: false,
                        });
                    }
                }
            }
        }
    }

    let passes = rough_ellipse(&mut rng, cx, cy, rx, ry, level);
    for pass in passes {
        for segment in dashed_or_solid(&pass, true, shape.style.stroke_style, shape.stroke_width) {
            ops.push(DrawOp::StrokePolyline {
                points: segment,
                closed: false,
                color: hex_color(&shape.color),
                width: shape.stroke_width,
                opacity: shape.style.opacity.alpha(),
                round_cap: shape.style.stroke_style == StrokeStyle::Dotted,
            });
        }
    }
    ops
}

fn ellipse_vertices(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    const SEGMENTS: usize = 48;
    (0..SEGMENTS)
        .map(|step| {
            let angle = step as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            (cx + angle.cos() * rx, cy + angle.sin() * ry)
        })
        .collect()
}

fn arrow_ops(shape: &ArrowShape) -> Vec<DrawOp> {
    let stroke = hex_color(&shape.color);
    let opacity = shape.style.opacity.alpha();
    let level = shape.style.sloppiness.level();
    let mut rng = Rng::from_shape_id(&shape.id);
    let mut ops = Vec::new();
    let passes = if level == 0 { 1 } else { 2 };
    for _ in 0..passes {
        let edge = rough_edge(&mut rng, (shape.x1, shape.y1), (shape.x2, shape.y2), level);
        for segment in dashed_or_solid(&edge, false, shape.style.stroke_style, shape.stroke_width) {
            ops.push(DrawOp::StrokePolyline {
                points: segment,
                closed: false,
                color: stroke,
                width: shape.stroke_width,
                opacity,
                round_cap: shape.style.stroke_style == StrokeStyle::Dotted,
            });
        }
    }
    // Head stays a crisp filled triangle: roughening a tiny triangle reads as
    // noise. It inherits opacity.
    ops.push(DrawOp::FillPolygon {
        points: shape
            .head_triangle((shape.stroke_width * 4.0).max(12.0))
            .to_vec(),
        color: stroke,
        opacity,
    });
    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::model::{
        DiamondShape, EdgeStyle, EllipseShape, ImageShape, LineShape, Opacity, PenShape, RectShape,
        STROKE_WIDTHS, ShapeStyle, Sloppiness, TextAlign, TextShape,
    };

    fn rect(id: &str, sloppiness: Sloppiness) -> RectShape {
        RectShape {
            id: id.to_string(),
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 60.0,
            color: "#1a1a1a".to_string(),
            stroke_width: 2.0,
            style: ShapeStyle {
                fill_style: FillStyle::None,
                sloppiness,
                ..ShapeStyle::default()
            },
        }
    }

    fn stroke_ops(ops: &[DrawOp]) -> Vec<Vec<(f32, f32)>> {
        ops.iter()
            .filter_map(|op| match op {
                DrawOp::StrokePolyline { points, .. } => Some(points.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn rough_edges_are_deterministic_per_shape_id() {
        let shape = Shape::Rectangle(rect("shape-a", Sloppiness::Rough));
        let first = shape_ops(&shape);
        let second = shape_ops(&shape);
        assert_eq!(first, second);
        let other = Shape::Rectangle(rect("shape-b", Sloppiness::Rough));
        assert_ne!(
            stroke_ops(&first)[0],
            stroke_ops(&shape_ops(&other))[0],
            "different ids must wobble differently"
        );
    }

    #[test]
    fn sloppiness_off_keeps_clean_geometry_and_one_pass() {
        let shape = Shape::Rectangle(rect("r", Sloppiness::Off));
        let ops = stroke_ops(&shape_ops(&shape));
        assert_eq!(ops.len(), 1, "one clean pass at sloppiness off");
        assert_eq!(
            ops[0].len(),
            4,
            "four closed corners; the renderer closes the ring"
        );
    }

    #[test]
    fn rough_sloppiness_draws_two_passes_and_stays_inside_the_wobble_budget() {
        let shape = Shape::Rectangle(rect("r", Sloppiness::Rough));
        let ops = stroke_ops(&shape_ops(&shape));
        assert_eq!(ops.len(), 2, "roughjs draws every edge twice");
        // Every sampled point stays within a small band of the true edge
        // (the wobble budget: endpoint offsets + curve bulge).
        for point in &ops[0] {
            assert!(
                point.0 > 0.0 && point.0 < 120.0,
                "x wobble out of band: {point:?}"
            );
            assert!(
                point.1 > 0.0 && point.1 < 80.0,
                "y wobble out of band: {point:?}"
            );
        }
    }

    #[test]
    fn very_rough_wobbles_more_than_rough() {
        let deviation = |ops: &[DrawOp]| -> f32 {
            stroke_ops(ops)
                .first()
                .and_then(|points| points.first())
                .map(|point| (point.0 - 10.0).abs() + (point.1 - 10.0).abs())
                .unwrap_or(0.0)
        };
        let mut total_rough = 0.0;
        let mut total_very = 0.0;
        for seed in ["a", "b", "c", "d", "e"] {
            let mut r = rect(seed, Sloppiness::Rough);
            r.id = format!("{seed}-rough");
            total_rough += deviation(&shape_ops(&Shape::Rectangle(r)));
            let mut v = rect(seed, Sloppiness::VeryRough);
            v.id = format!("{seed}-very");
            total_very += deviation(&shape_ops(&Shape::Rectangle(v)));
        }
        assert!(
            total_very > total_rough,
            "very-rough ({total_very}) should deviate more than rough ({total_rough})"
        );
    }

    #[test]
    fn dashed_outlines_are_split_into_segments() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.stroke_style = StrokeStyle::Dashed;
        let ops = stroke_ops(&shape_ops(&Shape::Rectangle(shape.clone())));
        assert!(
            ops.len() > 2,
            "a dashed rectangle needs several dash segments, got {}",
            ops.len()
        );
    }

    #[test]
    fn solid_outlines_are_one_polyline() {
        let shape = rect("r", Sloppiness::Off);
        let ops = stroke_ops(&shape_ops(&Shape::Rectangle(shape.clone())));
        assert_eq!(ops.len(), 1);
    }

    #[test]
    fn hachure_fill_produces_parallel_lines_inside_the_shape() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.fill_color = Some("#1971c2".to_string());
        shape.style.fill_style = FillStyle::Hachure;
        let ops = shape_ops(&Shape::Rectangle(shape.clone()));
        let fills = ops
            .iter()
            .filter(|op| matches!(op, DrawOp::FillPolygon { .. }))
            .count();
        assert_eq!(
            fills, 0,
            "hachure fills are stroked lines, not a solid fill"
        );
        let strokes = stroke_ops(&ops);
        assert!(
            strokes.len() > 4,
            "a 100x60 rect at gap 4 needs ~15 hatch lines, got {}",
            strokes.len()
        );
        for line in strokes.iter().take(6) {
            for (x, y) in line.iter() {
                assert!((7.0..=113.0).contains(x), "hatch x {x} out of bounds");
                assert!((7.0..=73.0).contains(y), "hatch y {y} out of bounds");
            }
        }
    }

    #[test]
    fn cross_hatch_fill_doubles_the_line_count() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.fill_color = Some("#1971c2".to_string());
        shape.style.fill_style = FillStyle::Hachure;
        let hachure = stroke_ops(&shape_ops(&Shape::Rectangle(shape.clone()))).len();
        shape.style.fill_style = FillStyle::CrossHatch;
        let cross = stroke_ops(&shape_ops(&Shape::Rectangle(shape.clone()))).len();
        assert_eq!(cross, hachure * 2, "cross-hatch adds the perpendicular set");
    }

    #[test]
    fn hatch_generation_is_deterministic_for_the_same_seed() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.fill_color = Some("#1971c2".to_string());
        shape.style.fill_style = FillStyle::CrossHatch;
        let first = shape_ops(&Shape::Rectangle(shape.clone()));
        let second = shape_ops(&Shape::Rectangle(shape.clone()));
        assert_eq!(first, second);
    }

    #[test]
    fn solid_fill_is_a_single_fill_polygon() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.fill_color = Some("#1971c2".to_string());
        shape.style.fill_style = FillStyle::Solid;
        let ops = shape_ops(&Shape::Rectangle(shape.clone()));
        assert!(matches!(&ops[0], DrawOp::FillPolygon { points, .. } if points.len() == 4));
    }

    #[test]
    fn opacity_is_carried_into_every_op() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.fill_color = Some("#1971c2".to_string());
        shape.style.fill_style = FillStyle::CrossHatch;
        shape.style.opacity = Opacity(50);
        for op in shape_ops(&Shape::Rectangle(shape.clone())) {
            match op {
                DrawOp::FillPolygon { opacity, .. } | DrawOp::StrokePolyline { opacity, .. } => {
                    assert!((opacity - 0.5).abs() < 1e-6);
                }
                DrawOp::Image { .. } | DrawOp::Opaque => panic!("unexpected op"),
            }
        }
    }

    #[test]
    fn dotted_strokes_use_round_caps() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.stroke_style = StrokeStyle::Dotted;
        let ops = shape_ops(&Shape::Rectangle(shape.clone()));
        assert!(ops.iter().any(|op| matches!(
            op,
            DrawOp::StrokePolyline {
                round_cap: true,
                ..
            }
        )));
    }

    #[test]
    fn rounded_rectangle_corners_are_chamfered() {
        let mut shape = rect("r", Sloppiness::Off);
        shape.style.edges = EdgeStyle::Round;
        let ops = shape_ops(&Shape::Rectangle(shape.clone()));
        let points = match &ops[0] {
            DrawOp::StrokePolyline { points, .. } => points.clone(),
            other => panic!("expected a stroke, got {other:?}"),
        };
        assert!(
            points.len() >= 12,
            "rounded corners add quarter-circle points, got {}",
            points.len()
        );
        // Every point stays inside the rectangle.
        for (x, y) in &points {
            assert!((9.0..=111.0).contains(x), "x {x} out of bounds");
            assert!((9.0..=71.0).contains(y), "y {y} out of bounds");
        }
    }

    #[test]
    fn diamond_emits_four_clean_vertices_when_edges_are_sharp() {
        let shape = DiamondShape {
            id: "d".to_string(),
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 80.0,
            color: "#1a1a1a".to_string(),
            stroke_width: 2.0,
            style: ShapeStyle::default(),
        };
        let ops = shape_ops(&Shape::Diamond(shape));
        let points = match &ops[0] {
            DrawOp::StrokePolyline { points, .. } => points.clone(),
            other => panic!("expected a stroke, got {other:?}"),
        };
        assert_eq!(points.len(), 4);
    }

    #[test]
    fn line_shape_is_a_bare_stroke_without_a_head() {
        let shape = LineShape {
            id: "l".to_string(),
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 100.0,
            color: "#1a1a1a".to_string(),
            stroke_width: 2.0,
            style: ShapeStyle::default(),
        };
        let ops = shape_ops(&Shape::Line(shape));
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], DrawOp::StrokePolyline { points, .. } if points.len() == 2));
    }

    #[test]
    fn image_shape_becomes_one_image_op() {
        let shape = ImageShape {
            id: "img".to_string(),
            asset_id: "asset-1".to_string(),
            bounds: Rect::new(5.0, 5.0, 50.0, 40.0),
            style: ShapeStyle {
                opacity: Opacity(80),
                ..ShapeStyle::default()
            },
        };
        let ops = shape_ops(&Shape::Image(shape));
        assert_eq!(
            ops,
            vec![DrawOp::Image {
                asset_id: "asset-1".to_string(),
                bounds: Rect::new(5.0, 5.0, 50.0, 40.0),
                opacity: 0.8,
            }]
        );
    }

    #[test]
    fn pen_strokes_and_text_stays_opaque() {
        let pen = PenShape {
            id: "p".to_string(),
            points: vec![(0.0, 0.0), (10.0, 10.0)],
            color: "#000000".to_string(),
            stroke_width: 2.0,
            style: ShapeStyle::default(),
        };
        assert!(matches!(
            shape_ops(&Shape::Pen(pen))[0],
            DrawOp::StrokePolyline { .. }
        ));
        let text = TextShape {
            id: "t".to_string(),
            x: 0.0,
            y: 0.0,
            text: "hi".to_string(),
            size: 20.0,
            color: "#000000".to_string(),
            align: TextAlign::Left,
            style: ShapeStyle::default(),
        };
        assert!(matches!(shape_ops(&Shape::Text(text))[0], DrawOp::Opaque));
    }

    #[test]
    fn stroke_widths_map_to_excalidraws_four_steps() {
        assert_eq!(STROKE_WIDTHS, [1.0, 2.0, 4.0, 8.0]);
    }

    #[test]
    fn ellipse_fill_and_outline_both_render() {
        let mut shape = EllipseShape {
            id: "e".to_string(),
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 80.0,
            color: "#1a1a1a".to_string(),
            stroke_width: 2.0,
            style: ShapeStyle {
                fill_color: Some("#f8d12c".to_string()),
                fill_style: FillStyle::Solid,
                ..ShapeStyle::default()
            },
        };
        let ops = shape_ops(&Shape::Ellipse(shape.clone()));
        assert!(matches!(&ops[0], DrawOp::FillPolygon { .. }));
        assert!(matches!(&ops[1], DrawOp::StrokePolyline { .. }));
        shape.style.fill_style = FillStyle::Hachure;
        let hachured = shape_ops(&Shape::Ellipse(shape.clone()));
        assert!(hachured.len() > 8, "hachured ellipse has many hatch lines");
    }
}
