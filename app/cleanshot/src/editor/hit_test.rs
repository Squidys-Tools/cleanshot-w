//! Hit-testing: which shape is under a point, which resize handle is under
//! the cursor. All coordinates are image pixels; `tolerance` is how far (in
//! image pixels) a stroke may be from the cursor and still count — the editor
//! passes `screen_tolerance / zoom` so the physical tolerance is constant.

use super::model::{Document, PenShape, Rect, Shape};

/// One of the 8 resize handles on a selected shape's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

impl Handle {
    /// The 8 handles in canonical order.
    pub const ALL: [Handle; 8] = [
        Handle::NorthWest,
        Handle::North,
        Handle::NorthEast,
        Handle::East,
        Handle::SouthEast,
        Handle::South,
        Handle::SouthWest,
        Handle::West,
    ];

    /// The handle's center for `bounds`.
    pub fn position(self, bounds: &Rect) -> (f32, f32) {
        let right = bounds.x + bounds.w;
        let bottom = bounds.y + bounds.h;
        match self {
            Handle::NorthWest => (bounds.x, bounds.y),
            Handle::North => ((bounds.x + right) / 2.0, bounds.y),
            Handle::NorthEast => (right, bounds.y),
            Handle::East => (right, (bounds.y + bottom) / 2.0),
            Handle::SouthEast => (right, bottom),
            Handle::South => ((bounds.x + right) / 2.0, bottom),
            Handle::SouthWest => (bounds.x, bottom),
            Handle::West => (bounds.x, (bounds.y + bottom) / 2.0),
        }
    }

    /// Apply a handle drag that moves this handle to `position`, producing
    /// the resized bounds. Opposite corner/edge stays anchored.
    pub fn resize_bounds(self, bounds: &Rect, px: f32, py: f32) -> Rect {
        let right = bounds.x + bounds.w;
        let bottom = bounds.y + bounds.h;
        let (mut x1, mut y1, mut x2, mut y2) = (bounds.x, bounds.y, right, bottom);
        match self {
            Handle::NorthWest | Handle::West | Handle::SouthWest => x1 = px,
            Handle::NorthEast | Handle::East | Handle::SouthEast => x2 = px,
            Handle::North | Handle::South => {}
        }
        match self {
            Handle::NorthWest | Handle::North | Handle::NorthEast => y1 = py,
            Handle::SouthWest | Handle::South | Handle::SouthEast => y2 = py,
            Handle::West | Handle::East => {}
        }
        let x = x1.min(x2);
        let y = y1.min(y2);
        Rect::new(x, y, (x2 - x1).abs().max(1.0), (y2 - y1).abs().max(1.0))
    }
}

/// Which shape (index into `document.shapes`) is under the point, if any.
/// Topmost shape wins — the document draws later shapes on top.
pub fn hit_test(document: &Document, px: f32, py: f32, tolerance: f32) -> Option<usize> {
    let tolerance = tolerance.max(0.0);
    document
        .shapes
        .iter()
        .enumerate()
        .rev()
        .find(|(_, shape)| shape_contains(shape, px, py, tolerance))
        .map(|(index, _)| index)
}

fn shape_contains(shape: &Shape, px: f32, py: f32, tolerance: f32) -> bool {
    match shape {
        Shape::Rectangle(rect) => stroke_contains_rect(
            Rect::new(rect.x, rect.y, rect.w, rect.h),
            px,
            py,
            rect.stroke_width,
            tolerance,
        ),
        Shape::Ellipse(ellipse) => {
            // Distance from the ellipse edge: normalize both the point and a
            // tolerance ring into the unit circle and compare radii.
            let cx = ellipse.x + ellipse.w / 2.0;
            let cy = ellipse.y + ellipse.h / 2.0;
            let rx = (ellipse.w / 2.0).max(f32::EPSILON);
            let ry = (ellipse.h / 2.0).max(f32::EPSILON);
            let nx = (px - cx) / rx;
            let ny = (py - cy) / ry;
            let r = (nx * nx + ny * ny).sqrt();
            let stroke = ellipse.stroke_width;
            let tol = tolerance + stroke / 2.0;
            let tol_nx = tol / rx;
            let tol_ny = tol / ry;
            let tol_r = (tol_nx * tol_nx + tol_ny * tol_ny).sqrt();
            r <= 1.0 + tol_r && r >= 1.0 - tol_r.min(1.0)
        }
        Shape::Arrow(arrow) => {
            let on_line = distance_to_segment(px, py, arrow.x1, arrow.y1, arrow.x2, arrow.y2)
                <= arrow.stroke_width / 2.0 + tolerance;
            if on_line {
                return true;
            }
            // The head is part of the arrow: also accept a hit inside the
            // head triangle (padded by the tolerance).
            let head = arrow.head_triangle(arrow.stroke_width * 4.0 + tolerance * 2.0);
            point_in_triangle(px, py, &head)
        }
        Shape::Pen(pen) => pen_contains(pen, px, py, tolerance),
        Shape::Text(_) => {
            // A text block's bounds are its rendered extent; keep the padding
            // modest so overlapping shapes stay reachable.
            let bounds = shape.bounds();
            bounds.contains(px, py) || distance_to_rect_edge(&bounds, px, py) <= tolerance
        }
        Shape::Counter(counter) => {
            let cx = counter.x + counter.diameter / 2.0;
            let cy = counter.y + counter.diameter / 2.0;
            let r = counter.diameter / 2.0;
            let dx = px - cx;
            let dy = py - cy;
            (dx * dx + dy * dy).sqrt() <= r + tolerance
        }
    }
}

/// Stroke hit for a rectangle: near the border, or inside when the stroke is
/// thick enough to cover the middle (small shapes).
fn stroke_contains_rect(rect: Rect, px: f32, py: f32, stroke: f32, tolerance: f32) -> bool {
    let Rect { x, y, w, h } = rect;
    let outer = tolerance + stroke / 2.0;
    let near_x = px >= x - outer && px <= x + w + outer;
    let near_y = py >= y - outer && py <= y + h + outer;
    if !(near_x && near_y) {
        return false;
    }
    let inner_x = px >= x + stroke / 2.0 + tolerance && px <= x + w - stroke / 2.0 - tolerance;
    let inner_y = py >= y + stroke / 2.0 + tolerance && py <= y + h - stroke / 2.0 - tolerance;
    !(inner_x && inner_y && w > stroke + 2.0 * tolerance && h > stroke + 2.0 * tolerance)
}

fn pen_contains(pen: &PenShape, px: f32, py: f32, tolerance: f32) -> bool {
    if pen.points.len() < 2 {
        return false;
    }
    pen.points.windows(2).any(|pair| {
        distance_to_segment(px, py, pair[0].0, pair[0].1, pair[1].0, pair[1].1)
            <= pen.stroke_width / 2.0 + tolerance
    })
}

/// Distance from (px, py) to the closest point of the segment (x1, y1)-(x2, y2).
pub fn distance_to_segment(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f32::EPSILON {
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }
    let t = (((px - x1) * dx + (py - y1) * dy) / length_sq).clamp(0.0, 1.0);
    let cx = x1 + t * dx;
    let cy = y1 + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

fn distance_to_rect_edge(rect: &Rect, px: f32, py: f32) -> f32 {
    let dx = if px < rect.x {
        rect.x - px
    } else if px > rect.right() {
        px - rect.right()
    } else {
        0.0
    };
    let dy = if py < rect.y {
        rect.y - py
    } else if py > rect.bottom() {
        py - rect.bottom()
    } else {
        0.0
    };
    (dx * dx + dy * dy).sqrt()
}

fn point_in_triangle(px: f32, py: f32, triangle: &[(f32, f32); 3]) -> bool {
    fn sign(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
        (ax - cx) * (by - cy) - (bx - cx) * (ay - cy)
    }
    let (a, b, c) = (triangle[0], triangle[1], triangle[2]);
    let d1 = sign(px, py, a.0, a.1, b.0, b.1);
    let d2 = sign(px, py, b.0, b.1, c.0, c.1);
    let d3 = sign(px, py, c.0, c.1, a.0, a.1);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

/// Which resize handle is under the point, given the selection's bounds.
/// `handle_size` is the handle's full side length in image pixels.
pub fn handle_at(bounds: &Rect, px: f32, py: f32, handle_size: f32) -> Option<Handle> {
    let half = handle_size / 2.0;
    Handle::ALL.into_iter().find(|handle| {
        let (hx, hy) = handle.position(bounds);
        px >= hx - half && px <= hx + half && py >= hy - half && py <= hy + half
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::model::{
        ArrowShape, COUNTER_DIAMETER, CounterShape, Document, EllipseShape, RectShape, TextShape,
    };

    fn document_with(shape: Shape) -> Document {
        let mut document = Document::new(1000, 1000);
        document.shapes.push(shape);
        document
    }

    fn rect(x: f32, y: f32, w: f32, h: f32, stroke: f32) -> Shape {
        Shape::Rectangle(RectShape {
            id: "r".to_string(),
            x,
            y,
            w,
            h,
            color: "#e03131".to_string(),
            stroke_width: stroke,
        })
    }

    #[test]
    fn rectangle_border_hits_but_interior_misses() {
        let document = document_with(rect(100.0, 100.0, 200.0, 100.0, 4.0));
        assert_eq!(
            hit_test(&document, 100.0, 150.0, 4.0),
            Some(0),
            "left border"
        );
        assert_eq!(
            hit_test(&document, 300.0, 150.0, 4.0),
            Some(0),
            "right border"
        );
        assert_eq!(
            hit_test(&document, 200.0, 100.0, 4.0),
            Some(0),
            "top border"
        );
        assert_eq!(
            hit_test(&document, 200.0, 150.0, 4.0),
            None,
            "middle is a miss"
        );
    }

    #[test]
    fn hit_tolerance_can_shrink_with_zoom() {
        let document = document_with(rect(100.0, 100.0, 200.0, 100.0, 4.0));
        assert_eq!(hit_test(&document, 95.0, 150.0, 4.0), Some(0));
        assert_eq!(hit_test(&document, 95.0, 150.0, 0.25), None);
    }

    #[test]
    fn thin_rectangle_interior_is_a_hit_because_the_stroke_covers_it() {
        let document = document_with(rect(100.0, 100.0, 8.0, 8.0, 4.0));
        assert_eq!(hit_test(&document, 104.0, 104.0, 4.0), Some(0));
    }

    #[test]
    fn ellipse_ring_hits_but_center_misses() {
        let shape = Shape::Ellipse(EllipseShape {
            id: "e".to_string(),
            x: 100.0,
            y: 100.0,
            w: 200.0,
            h: 100.0,
            color: "#1971c2".to_string(),
            stroke_width: 4.0,
        });
        let document = document_with(shape);
        assert_eq!(
            hit_test(&document, 300.0, 150.0, 4.0),
            Some(0),
            "right vertex"
        );
        assert_eq!(
            hit_test(&document, 200.0, 100.0, 4.0),
            Some(0),
            "top vertex"
        );
        assert_eq!(hit_test(&document, 200.0, 150.0, 4.0), None, "center");
    }

    #[test]
    fn filled_ellipse_is_one_hit_everywhere() {
        let mut shape = Shape::Ellipse(EllipseShape {
            id: "e".to_string(),
            x: 100.0,
            y: 100.0,
            w: 200.0,
            h: 100.0,
            color: "#1971c2".to_string(),
            stroke_width: 4.0,
        });
        if let Shape::Ellipse(ellipse) = &mut shape {
            ellipse.stroke_width = 220.0; // thicker than the radii: filled
        }
        let document = document_with(shape);
        assert_eq!(hit_test(&document, 200.0, 150.0, 4.0), Some(0));
    }

    #[test]
    fn arrow_hits_along_the_whole_line_and_not_beside_it() {
        let shape = Shape::Arrow(ArrowShape {
            id: "a".to_string(),
            x1: 0.0,
            y1: 0.0,
            x2: 300.0,
            y2: 0.0,
            color: "#1a1a1a".to_string(),
            stroke_width: 4.0,
        });
        let document = document_with(shape);
        assert_eq!(hit_test(&document, 150.0, 0.0, 4.0), Some(0), "mid-line");
        assert_eq!(
            hit_test(&document, 290.0, 0.0, 4.0),
            Some(0),
            "near the head"
        );
        assert_eq!(hit_test(&document, 150.0, 30.0, 4.0), None, "off the line");
    }

    #[test]
    fn pen_hits_its_polyline() {
        let shape = Shape::Pen(PenShape {
            id: "p".to_string(),
            points: vec![(0.0, 0.0), (100.0, 100.0)],
            color: "#1a1a1a".to_string(),
            stroke_width: 4.0,
        });
        let document = document_with(shape);
        assert_eq!(hit_test(&document, 50.0, 50.0, 4.0), Some(0));
        assert_eq!(hit_test(&document, 50.0, 20.0, 4.0), None);
    }

    #[test]
    fn topmost_shape_wins() {
        // Two rectangles sharing the border line x=100.
        let mut document = Document::new(1000, 1000);
        document.shapes.push(rect(0.0, 0.0, 100.0, 100.0, 4.0));
        document.shapes.push(rect(100.0, 0.0, 100.0, 100.0, 4.0));
        // On the shared border both shapes are hit; the later (topmost) wins.
        assert_eq!(hit_test(&document, 100.0, 50.0, 4.0), Some(1));
        assert_eq!(hit_test(&document, 0.0, 50.0, 4.0), Some(0));
        assert_eq!(hit_test(&document, 300.0, 50.0, 4.0), None);
    }

    #[test]
    fn text_hits_its_block() {
        let shape = Shape::Text(TextShape {
            id: "t".to_string(),
            x: 100.0,
            y: 100.0,
            text: "hello".to_string(),
            size: 24.0,
            color: "#1a1a1a".to_string(),
        });
        let document = document_with(shape);
        let bounds = document.shapes[0].bounds();
        assert_eq!(hit_test(&document, 105.0, bounds.y + 5.0, 4.0), Some(0));
        assert_eq!(hit_test(&document, 0.0, 0.0, 4.0), None);
    }

    #[test]
    fn counter_hits_its_badge_circle() {
        let shape = Shape::Counter(CounterShape {
            id: "c".to_string(),
            x: 100.0,
            y: 100.0,
            n: 1,
            color: "#e03131".to_string(),
            diameter: COUNTER_DIAMETER,
        });
        let document = document_with(shape);
        let cx = 100.0 + COUNTER_DIAMETER / 2.0;
        assert_eq!(
            hit_test(&document, cx, 100.0 + COUNTER_DIAMETER, 4.0),
            Some(0)
        );
        assert_eq!(
            hit_test(
                &document,
                cx + COUNTER_DIAMETER,
                100.0 + COUNTER_DIAMETER,
                4.0
            ),
            None
        );
    }

    #[test]
    fn handles_map_to_the_right_corners() {
        let bounds = Rect::new(100.0, 100.0, 100.0, 50.0);
        let size = 12.0;
        assert_eq!(
            handle_at(&bounds, 100.0, 100.0, size),
            Some(Handle::NorthWest)
        );
        assert_eq!(
            handle_at(&bounds, 200.0, 100.0, size),
            Some(Handle::NorthEast)
        );
        assert_eq!(
            handle_at(&bounds, 200.0, 150.0, size),
            Some(Handle::SouthEast)
        );
        assert_eq!(
            handle_at(&bounds, 100.0, 150.0, size),
            Some(Handle::SouthWest)
        );
        assert_eq!(handle_at(&bounds, 150.0, 100.0, size), Some(Handle::North));
        assert_eq!(
            handle_at(&bounds, 150.0, 175.0, size),
            None,
            "outside the bounds"
        );
    }

    #[test]
    fn resizing_by_handles_anchors_the_opposite_side() {
        let bounds = Rect::new(100.0, 100.0, 100.0, 50.0);
        let resized = Handle::SouthEast.resize_bounds(&bounds, 130.0, 140.0);
        assert_eq!(resized, Rect::new(100.0, 100.0, 30.0, 40.0));

        let flipped = Handle::NorthWest.resize_bounds(&bounds, 20.0, 30.0);
        assert_eq!(flipped, Rect::new(20.0, 30.0, 180.0, 120.0));

        let edge = Handle::East.resize_bounds(&bounds, 160.0, 999.0);
        assert_eq!(
            edge,
            Rect::new(100.0, 100.0, 60.0, 50.0),
            "vertical edges ignore the y move"
        );
    }

    #[test]
    fn empty_document_never_hits() {
        let document = Document::new(100, 100);
        assert_eq!(hit_test(&document, 50.0, 50.0, 4.0), None);
    }
}
