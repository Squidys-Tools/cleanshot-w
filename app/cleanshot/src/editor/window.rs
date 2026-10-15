//! The annotation editor window: the captured image on a zoom/pan canvas,
//! shape tools, selection with handles, undo/redo, crop, and export.
//!
//! Coordinate spaces, kept strictly apart (see `platform.rs` notes):
//! - **image px** — the capture's original resolution; all document geometry.
//! - **screen px (GPUI logical)** — window-relative; `screen = pan + image × zoom`.
//!   Export flattens 1:1 in image px via `flatten`, never screen px.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    App, Bounds, Context, Corners, FocusHandle, KeyBinding, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Point, Render, RenderImage,
    ScrollWheelEvent, Size, Window, WindowBounds, WindowOptions, actions, canvas, div, prelude::*,
    px, rgb, size,
};

use crate::clipboard;
use crate::editor::flatten::flatten_to_png;
use crate::editor::history::History;
use crate::editor::hit_test::{Handle, handle_at, hit_test};
use crate::editor::model::{
    ArrowShape, COUNTER_DIAMETER, CounterShape, DEFAULT_TEXT_SIZE, Document, EllipseShape, PALETTE,
    PenShape, Rect, RectShape, STROKE_WIDTHS, Shape, ShapeId, TextShape,
};
use crate::library;
use crate::{platform, save_dialog};

actions!(
    editor,
    [
        ToolSelect,
        ToolRectangle,
        ToolEllipse,
        ToolArrow,
        ToolPen,
        ToolText,
        ToolCounter,
        ToolCrop,
        DeleteShape,
        UndoEdit,
        RedoEdit,
        CopyFlattened,
        SaveFlattened,
        ApplyCrop,
        CancelInteraction,
        BringForwardShape,
        SendBackwardShape,
        ZoomIn,
        ZoomOut,
        ZoomFit,
        CloseEditor,
    ]
);

const TOOLBAR_HEIGHT: f32 = 48.0;
const SELECTION_BAR_HEIGHT: f32 = 44.0;
const HANDLE_SCREEN_SIZE: f32 = 10.0; // selection handles, screen px
const ZOOM_MIN: f32 = 0.05;
const ZOOM_MAX: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Rectangle,
    Ellipse,
    Arrow,
    Pen,
    Text,
    Counter,
    Crop,
}

impl Tool {
    fn cursor(self) -> gpui::CursorStyle {
        match self {
            Tool::Select => gpui::CursorStyle::Arrow,
            _ => gpui::CursorStyle::Crosshair,
        }
    }
}

/// What a mouse drag is currently doing.
#[derive(Debug, Clone)]
enum Drag {
    None,
    Pan {
        last: Point<Pixels>,
    },
    /// Rectangle/ellipse/arrow rubber-band, in image px.
    Draw {
        start: (f32, f32),
        current: (f32, f32),
    },
    /// Freehand pen, accumulating points in image px.
    PenDraft {
        points: Vec<(f32, f32)>,
    },
    /// Moving a shape from its original geometry.
    Move {
        id: ShapeId,
        original: Shape,
        start: (f32, f32),
    },
    /// Resizing a shape from its original geometry.
    Resize {
        id: ShapeId,
        original: Shape,
        handle: Handle,
    },
    /// Dragging out a fresh crop rect.
    CropNew {
        start: (f32, f32),
    },
    /// Moving the existing crop rect.
    CropMove {
        start: (f32, f32),
        original: Rect,
    },
    /// Resizing the crop rect by a handle.
    CropResize {
        original: Rect,
        handle: Handle,
    },
}

/// Everything needed to open one editor window.
pub struct EditorSource {
    pub png: Vec<u8>,
    pub title: String,
    /// Annotation document to edit (None → fresh, un-annotated image).
    pub doc: Option<Document>,
    /// Where the document persists (None → transient, e.g. the Linux
    /// `--edit` dev loop).
    pub doc_path: Option<PathBuf>,
}

pub struct EditorWindow {
    focus: FocusHandle,
    title: String,
    image: Arc<RenderImage>,
    /// The base image, RGBA, at the document's resolution.
    rgba: image::RgbaImage,
    doc: Document,
    doc_path: Option<PathBuf>,
    history: History,
    tool: Tool,
    color_index: usize,
    stroke_width_index: usize,
    selected: Option<ShapeId>,
    zoom: f32,
    pan: Point<Pixels>,
    drag: Drag,
    space_held: bool,
    /// The text shape being edited and its caret (char index).
    text_editing: Option<ShapeId>,
    caret: usize,
    /// Crop rect in image px while the crop tool is active.
    crop: Option<Rect>,
}

/// Open an editor window for a capture.
pub fn open(cx: &mut App, source: EditorSource) -> Result<(), String> {
    let rgba = image::ImageReader::new(std::io::Cursor::new(&source.png))
        .with_guessed_format()
        .map_err(|error| format!("Could not open the capture image: {error}"))?
        .decode()
        .map_err(|error| format!("Could not decode the capture image: {error}"))?
        .into_rgba8();
    let (width, height) = rgba.dimensions();

    let mut doc = match source.doc {
        Some(doc) => doc,
        None => Document::new(width, height),
    };
    doc.image_id = source
        .doc_path
        .as_ref()
        .and_then(|path| path.parent())
        .and_then(|path| path.file_name())
        .map(|id| id.to_string_lossy().into_owned());
    let image = crate::pin::decode_to_render_image(&source.png)?;
    if doc.image_width != width || doc.image_height != height {
        return Err(format!(
            "The annotation document is {}×{} but the image is {width}×{height}.",
            doc.image_width, doc.image_height
        ));
    }

    // Editor: a normal, decorated window the user can move and resize.
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: platform::gpui_virtual_screen_bounds(cx).origin + gpui::point(px(60.), px(60.)),
            size: size(px(960.), px(700.)),
        })),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some(format!("Snipsy — {}", source.title).into()),
            ..Default::default()
        }),
        focus: true,
        show: true,
        #[cfg(not(windows))]
        window_decorations: Some(gpui::WindowDecorations::Client),
        ..Default::default()
    };

    cx.open_window(options, |window, cx| {
        let view = cx.new(|cx| EditorWindow {
            focus: cx.focus_handle(),
            title: source.title.clone(),
            image,
            rgba,
            doc,
            doc_path: source.doc_path.clone(),
            history: History::new(),
            tool: Tool::Select,
            color_index: 4, // blue, the old app's default
            stroke_width_index: 1,
            selected: None,
            zoom: 0.0, // fit on first render
            pan: Point::default(),
            drag: Drag::None,
            space_held: false,
            text_editing: None,
            caret: 0,
            crop: None,
        });
        let focus = view.read(cx).focus.clone();
        window.focus(&focus, cx);
        view
    })
    .map_err(|error| format!("Could not open the editor window: {error}"))?;
    Ok(())
}

/// Keystroke bindings for the editor window. Old app letters plus the
/// 1..n tool shortcuts.
pub fn keybindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("v", ToolSelect, Some("EditorWindow")),
        KeyBinding::new("r", ToolRectangle, Some("EditorWindow")),
        KeyBinding::new("o", ToolEllipse, Some("EditorWindow")),
        KeyBinding::new("a", ToolArrow, Some("EditorWindow")),
        KeyBinding::new("d", ToolPen, Some("EditorWindow")),
        KeyBinding::new("t", ToolText, Some("EditorWindow")),
        KeyBinding::new("c", ToolCounter, Some("EditorWindow")),
        KeyBinding::new("shift-x", ToolCrop, Some("EditorWindow")),
        KeyBinding::new("1", ToolSelect, Some("EditorWindow")),
        KeyBinding::new("2", ToolRectangle, Some("EditorWindow")),
        KeyBinding::new("3", ToolEllipse, Some("EditorWindow")),
        KeyBinding::new("4", ToolArrow, Some("EditorWindow")),
        KeyBinding::new("5", ToolPen, Some("EditorWindow")),
        KeyBinding::new("6", ToolText, Some("EditorWindow")),
        KeyBinding::new("7", ToolCounter, Some("EditorWindow")),
        KeyBinding::new("8", ToolCrop, Some("EditorWindow")),
        KeyBinding::new("delete", DeleteShape, Some("EditorWindow")),
        KeyBinding::new("backspace", DeleteShape, Some("EditorWindow")),
        KeyBinding::new("ctrl-z", UndoEdit, Some("EditorWindow")),
        KeyBinding::new("ctrl-shift-z", RedoEdit, Some("EditorWindow")),
        KeyBinding::new("ctrl-y", RedoEdit, Some("EditorWindow")),
        KeyBinding::new("ctrl-c", CopyFlattened, Some("EditorWindow")),
        KeyBinding::new("ctrl-s", SaveFlattened, Some("EditorWindow")),
        KeyBinding::new("ctrl-enter", ApplyCrop, Some("EditorWindow")),
        KeyBinding::new("escape", CancelInteraction, Some("EditorWindow")),
        KeyBinding::new("ctrl-]", BringForwardShape, Some("EditorWindow")),
        KeyBinding::new("ctrl-[", SendBackwardShape, Some("EditorWindow")),
        KeyBinding::new("=", ZoomIn, Some("EditorWindow")),
        KeyBinding::new("-", ZoomOut, Some("EditorWindow")),
        KeyBinding::new("0", ZoomFit, Some("EditorWindow")),
        KeyBinding::new("ctrl-w", CloseEditor, Some("EditorWindow")),
    ]
}

impl EditorWindow {
    // ---- coordinate transforms -------------------------------------------

    fn to_image(&self, position: Point<Pixels>) -> (f32, f32) {
        (
            f32::from(position.x - self.pan.x) / self.zoom.max(f32::EPSILON),
            f32::from(position.y - self.pan.y) / self.zoom.max(f32::EPSILON),
        )
    }

    fn to_screen_x(&self, x: f32) -> Pixels {
        self.pan.x + px(x * self.zoom)
    }

    fn to_screen_y(&self, y: f32) -> Pixels {
        self.pan.y + px(y * self.zoom)
    }

    /// Fit the image in the viewport (above the toolbar) and center it.
    fn fit(&mut self, viewport: Size<Pixels>) {
        let available_w = (viewport.width - px(40.0)).max(px(100.));
        let available_h = (viewport.height - px(TOOLBAR_HEIGHT + 40.0)).max(px(100.));
        let zoom_x = f32::from(available_w) / self.doc.image_width as f32;
        let zoom_y = f32::from(available_h) / self.doc.image_height as f32;
        self.zoom = zoom_x.min(zoom_y).clamp(ZOOM_MIN, 1.0);
        self.pan = Point {
            x: (viewport.width - px(self.doc.image_width as f32 * self.zoom)) / 2.0,
            y: (viewport.height
                - px(TOOLBAR_HEIGHT)
                - px(self.doc.image_height as f32 * self.zoom))
                / 2.0,
        };
    }

    fn zoom_at(&mut self, factor: f32, anchor: Point<Pixels>) {
        let next = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
        let ratio = next / self.zoom;
        // Keep the image point under the cursor fixed.
        self.pan.x = anchor.x - (anchor.x - self.pan.x) * ratio;
        self.pan.y = anchor.y - (anchor.y - self.pan.y) * ratio;
        self.zoom = next;
    }

    // ---- model helpers ---------------------------------------------------

    fn current_color(&self) -> String {
        PALETTE[self.color_index].to_string()
    }

    fn current_stroke_width(&self) -> f32 {
        STROKE_WIDTHS[self.stroke_width_index]
    }

    fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        self.doc.index_of(selected)
    }

    /// A snapshot commit for the open interaction. Persists the document.
    fn commit(&mut self) {
        self.history.commit(&self.doc, None);
        self.persist();
    }

    fn commit_with_image(&mut self) {
        self.history.commit(&self.doc, Some(&self.rgba));
        self.persist();
    }

    fn persist(&self) {
        if let Some(path) = &self.doc_path
            && let Err(error) = library::write_annotations_idempotent(&self.doc, path)
        {
            eprintln!("Snipsy: {error}");
        }
    }

    fn begin_text_editing(&mut self, id: ShapeId) {
        self.text_editing = Some(id);
        self.caret = self
            .doc
            .index_of(self.text_editing.as_ref().unwrap())
            .and_then(|index| match &self.doc.shapes[index] {
                Shape::Text(text) => Some(text.text.chars().count()),
                _ => None,
            })
            .unwrap_or(0);
    }

    /// Finish the open text edit: commit (or drop an empty text shape).
    fn commit_text_edit(&mut self) {
        let Some(id) = self.text_editing.take() else {
            return;
        };
        let empty = self
            .doc
            .index_of(&id)
            .is_none_or(|index| match &self.doc.shapes[index] {
                Shape::Text(text) => text.text.trim().is_empty(),
                _ => true,
            });
        if empty {
            self.doc.delete(&id);
            self.selected = None;
        }
        self.commit();
        if self.selected.is_none() {
            self.selected = None;
        }
    }

    fn end_flattened_png(&self) -> Result<Vec<u8>, String> {
        flatten_to_png(&self.doc, self.rgba.as_raw())
    }

    // ---- interaction: mouse ---------------------------------------------

    fn on_canvas_mouse_down(&mut self, event: &MouseDownEvent) {
        if self.text_editing.is_some() {
            self.commit_text_edit();
            return;
        }
        if event.button == MouseButton::Middle {
            self.drag = Drag::Pan {
                last: event.position,
            };
            return;
        }
        if event.button != MouseButton::Left {
            return;
        }
        if self.space_held {
            self.drag = Drag::Pan {
                last: event.position,
            };
            return;
        }
        let (ix, iy) = self.to_image(event.position);
        match self.tool {
            Tool::Select => self.select_mouse_down(ix, iy),
            Tool::Rectangle | Tool::Ellipse | Tool::Arrow => {
                self.history.begin(&self.doc, None);
                self.drag = Drag::Draw {
                    start: (ix, iy),
                    current: (ix, iy),
                };
            }
            Tool::Pen => {
                self.history.begin(&self.doc, None);
                self.drag = Drag::PenDraft {
                    points: vec![(ix, iy)],
                };
            }
            Tool::Text => self.place_text(ix, iy),
            Tool::Counter => self.place_counter(ix, iy),
            Tool::Crop => self.crop_mouse_down(ix, iy),
        }
    }

    fn select_mouse_down(&mut self, ix: f32, iy: f32) {
        // Resize handles win over everything.
        if let Some(index) = self.selected_index()
            && let Some(handle) = handle_at(
                &self.doc.shapes[index].bounds(),
                ix,
                iy,
                HANDLE_SCREEN_SIZE / self.zoom,
            )
        {
            self.history.begin(&self.doc, None);
            self.drag = Drag::Resize {
                id: self.doc.shapes[index].id().clone(),
                original: self.doc.shapes[index].clone(),
                handle,
            };
            return;
        }
        match hit_test(&self.doc, ix, iy, 4.0 / self.zoom) {
            Some(index) => {
                let id = self.doc.shapes[index].id().clone();
                if self.selected.as_ref() != Some(&id) {
                    self.selected = Some(id.clone());
                }
                self.history.begin(&self.doc, None);
                self.drag = Drag::Move {
                    id,
                    original: self.doc.shapes[index].clone(),
                    start: (ix, iy),
                };
            }
            None => {
                self.selected = None;
            }
        }
    }

    fn place_text(&mut self, ix: f32, iy: f32) {
        if let Some(index) = hit_test(&self.doc, ix, iy, 4.0 / self.zoom)
            && matches!(&self.doc.shapes[index], Shape::Text(_))
        {
            let id = self.doc.shapes[index].id().clone();
            self.selected = Some(id.clone());
            self.history.begin(&self.doc, None);
            self.begin_text_editing(id);
            return;
        }
        self.history.begin(&self.doc, None);
        let id = next_shape_id("text");
        let shape = TextShape {
            id: id.clone(),
            x: ix,
            y: iy,
            text: String::new(),
            size: DEFAULT_TEXT_SIZE,
            color: self.current_color(),
        };
        self.doc.shapes.push(Shape::Text(shape));
        self.selected = Some(id.clone());
        self.begin_text_editing(id);
    }

    fn place_counter(&mut self, ix: f32, iy: f32) {
        self.history.begin(&self.doc, None);
        let n = self.doc.next_counter_number();
        let id = next_shape_id(&format!("counter-{n}"));
        self.doc.shapes.push(Shape::Counter(CounterShape {
            id,
            x: ix - COUNTER_DIAMETER / 2.0,
            y: iy - COUNTER_DIAMETER / 2.0,
            n,
            color: self.current_color(),
            diameter: COUNTER_DIAMETER,
        }));
        self.commit();
        // Counters stay selected so the palette applies to them.
        self.selected = self.doc.shapes.last().map(|shape| shape.id().clone());
    }

    fn crop_mouse_down(&mut self, ix: f32, iy: f32) {
        let Some(crop) = self.crop else {
            self.drag = Drag::CropNew { start: (ix, iy) };
            self.crop = Some(Rect::new(ix, iy, 0.0, 0.0));
            return;
        };
        if let Some(handle) = handle_at(&crop, ix, iy, HANDLE_SCREEN_SIZE / self.zoom) {
            self.drag = Drag::CropResize {
                original: crop,
                handle,
            };
        } else if crop.contains(ix, iy) {
            self.drag = Drag::CropMove {
                start: (ix, iy),
                original: crop,
            };
        } else {
            self.drag = Drag::CropNew { start: (ix, iy) };
        }
    }

    fn on_canvas_mouse_move(&mut self, event: &MouseMoveEvent) {
        let position = event.position;
        match self.drag.clone() {
            Drag::None => {}
            Drag::Pan { last } => {
                self.pan.x += position.x - last.x;
                self.pan.y += position.y - last.y;
                self.drag = Drag::Pan { last: position };
            }
            Drag::Draw { start, .. } => {
                self.drag = Drag::Draw {
                    start,
                    current: self.to_image(position),
                };
            }
            Drag::PenDraft { mut points } => {
                let point = self.to_image(position);
                let moved_far = points.last().is_none_or(|last| {
                    (last.0 - point.0).powi(2) + (last.1 - point.1).powi(2)
                        > (1.5 / self.zoom).powi(2)
                });
                if moved_far {
                    points.push(point);
                }
                self.drag = Drag::PenDraft { points };
            }
            Drag::Move {
                id,
                original,
                start,
            } => {
                let (ix, iy) = self.to_image(position);
                if let Some(shape) = self.doc.shapes.iter_mut().find(|shape| shape.id() == &id) {
                    *shape = original.clone();
                    shape.translate(ix - start.0, iy - start.1);
                }
            }
            Drag::Resize {
                id,
                original,
                handle,
            } => {
                let (ix, iy) = self.to_image(position);
                let original_bounds = original.bounds();
                let target = handle.resize_bounds(&original_bounds, ix, iy);
                if let Some(shape) = self.doc.shapes.iter_mut().find(|shape| shape.id() == &id) {
                    *shape = original.clone();
                    shape.scale_to(&target);
                }
            }
            Drag::CropNew { start } => {
                let (ix, iy) = self.to_image(position);
                self.crop = Some(Rect::new(
                    start.0.min(ix),
                    start.1.min(iy),
                    (start.0 - ix).abs(),
                    (start.1 - iy).abs(),
                ));
            }
            Drag::CropMove { start, original } => {
                let (ix, iy) = self.to_image(position);
                self.crop = Some(Rect::new(
                    original.x + (ix - start.0),
                    original.y + (iy - start.1),
                    original.w,
                    original.h,
                ));
            }
            Drag::CropResize { original, handle } => {
                let (ix, iy) = self.to_image(position);
                self.crop = Some(handle.resize_bounds(&original, ix, iy));
            }
        }
    }

    fn on_canvas_mouse_up(&mut self, event: &MouseUpEvent) {
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        match drag {
            Drag::None | Drag::Pan { .. } => {}
            Drag::Draw { start, current } => {
                self.finish_draw(start, current);
            }
            Drag::PenDraft { points } => {
                if points.len() >= 2 {
                    let id = next_shape_id("pen");
                    self.doc.shapes.push(Shape::Pen(PenShape {
                        id,
                        points,
                        color: self.current_color(),
                        stroke_width: self.current_stroke_width(),
                    }));
                    self.selected = self.doc.shapes.last().map(|shape| shape.id().clone());
                    self.commit();
                } else {
                    let _ = self.history.cancel();
                }
            }
            Drag::Move { .. } | Drag::Resize { .. } => self.commit(),
            Drag::CropNew { .. } | Drag::CropMove { .. } | Drag::CropResize { .. } => {
                let _ = event;
            }
        }
    }

    /// Insert the rectangle/ellipse/arrow that was rubber-banded.
    fn finish_draw(&mut self, start: (f32, f32), current: (f32, f32)) {
        let id = next_shape_id(self.tool_name());
        let created = match self.tool {
            Tool::Rectangle => {
                (start.0 - current.0).abs() >= 2.0 || (start.1 - current.1).abs() >= 2.0
            }
            Tool::Ellipse => {
                (start.0 - current.0).abs() >= 2.0 || (start.1 - current.1).abs() >= 2.0
            }
            Tool::Arrow => (start.0 - current.0).powi(2) + (start.1 - current.1).powi(2) >= 4.0,
            _ => false,
        };
        if !created {
            let _ = self.history.cancel();
            return;
        }
        let shape = match self.tool {
            Tool::Rectangle => Shape::Rectangle(RectShape {
                id,
                x: start.0.min(current.0),
                y: start.1.min(current.1),
                w: (start.0 - current.0).abs(),
                h: (start.1 - current.1).abs(),
                color: self.current_color(),
                stroke_width: self.current_stroke_width(),
            }),
            Tool::Ellipse => Shape::Ellipse(EllipseShape {
                id,
                x: start.0.min(current.0),
                y: start.1.min(current.1),
                w: (start.0 - current.0).abs(),
                h: (start.1 - current.1).abs(),
                color: self.current_color(),
                stroke_width: self.current_stroke_width(),
            }),
            Tool::Arrow => Shape::Arrow(ArrowShape {
                id,
                x1: start.0,
                y1: start.1,
                x2: current.0,
                y2: current.1,
                color: self.current_color(),
                stroke_width: self.current_stroke_width(),
            }),
            _ => unreachable!("finish_draw only runs for drawing tools"),
        };
        self.doc.shapes.push(shape);
        self.selected = self.doc.shapes.last().map(|shape| shape.id().clone());
        self.commit();
    }

    fn tool_name(&self) -> &'static str {
        match self.tool {
            Tool::Select => "select",
            Tool::Rectangle => "rect",
            Tool::Ellipse => "ellipse",
            Tool::Arrow => "arrow",
            Tool::Pen => "pen",
            Tool::Text => "text",
            Tool::Counter => "counter",
            Tool::Crop => "crop",
        }
    }

    // ---- interaction: keyboard ------------------------------------------

    fn on_key_down_raw(&mut self, event: &gpui::KeyDownEvent) {
        if self.text_editing.is_some() {
            self.handle_text_keystroke(event);
            return;
        }
        if event.keystroke.key == "space"
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.platform
        {
            self.space_held = true;
        }
    }

    fn on_key_up_raw(&mut self, event: &gpui::KeyUpEvent) {
        if event.keystroke.key == "space" {
            self.space_held = false;
        }
    }

    /// Raw text input for the text shape being edited.
    fn handle_text_keystroke(&mut self, event: &gpui::KeyDownEvent) {
        let Some(id) = self.text_editing.clone() else {
            return;
        };
        let Some(index) = self.doc.index_of(&id) else {
            self.text_editing = None;
            return;
        };
        let Shape::Text(text) = &mut self.doc.shapes[index] else {
            self.text_editing = None;
            return;
        };
        let key = event.keystroke.key.as_str();
        match key {
            "escape" => {
                self.commit_text_edit();
            }
            "enter" => {
                let caret = self.caret;
                text.text
                    .insert(char_to_byte_index(&text.text, caret), '\n');
                self.caret = caret + 1;
            }
            "backspace" => {
                if self.caret > 0 {
                    let caret = self.caret;
                    let remove_at = text
                        .text
                        .char_indices()
                        .nth(caret - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    text.text.remove(remove_at);
                    self.caret = caret - 1;
                }
            }
            "delete" => {
                let caret = self.caret;
                let remove_at = text.text.char_indices().nth(caret).map(|(i, _)| i);
                if let Some(remove_at) = remove_at {
                    text.text.remove(remove_at);
                }
            }
            "left" => {
                self.caret = self.caret.saturating_sub(1);
            }
            "right" => {
                let max = text.text.chars().count();
                self.caret = (self.caret + 1).min(max);
            }
            "up" | "home" => {
                self.caret = 0;
            }
            "down" | "end" => {
                self.caret = text.text.chars().count();
            }
            _ => {
                if event.keystroke.modifiers.platform
                    || event.keystroke.modifiers.control
                    || event.keystroke.modifiers.alt
                {
                    return;
                }
                if let Some(character) = event
                    .keystroke
                    .key_char
                    .as_deref()
                    .and_then(|key| key.chars().next())
                    && !character.is_control()
                {
                    let caret = self.caret;
                    text.text
                        .insert(char_to_byte_index(&text.text, caret), character);
                    self.caret = caret + 1;
                }
            }
        }
    }

    // ---- actions ---------------------------------------------------------

    fn set_tool(&mut self, tool: Tool) {
        if self.text_editing.is_some() {
            return;
        }
        if tool == Tool::Crop {
            self.crop = None;
        }
        self.tool = tool;
        if tool != Tool::Select {
            // Keep the selection visible but drop it when switching to a
            // drawing tool, so the next stroke is not a surprise resize.
            self.selected = None;
        }
    }

    fn delete_selected(&mut self) {
        if self.text_editing.is_some() {
            return;
        }
        if let Some(id) = self.selected.take() {
            self.history.begin(&self.doc, None);
            self.doc.delete(&id);
            self.commit();
        }
    }

    fn reorder_selected(&mut self, forward: bool) {
        if self.text_editing.is_some() {
            return;
        }
        if let Some(index) = self.selected_index() {
            self.history.begin(&self.doc, None);
            let moved = if forward {
                self.doc.bring_forward(index)
            } else {
                self.doc.send_backward(index)
            };
            if moved {
                self.commit();
            } else {
                let _ = self.history.cancel();
            }
        }
    }

    fn apply_color_to_selection(&mut self, color_index: usize) {
        self.color_index = color_index;
        let color = self.current_color();
        if self.text_editing.is_some() {
            return;
        }
        if let Some(index) = self.selected_index() {
            self.history.begin(&self.doc, None);
            let shape_color = match &mut self.doc.shapes[index] {
                Shape::Rectangle(shape) => &mut shape.color,
                Shape::Ellipse(shape) => &mut shape.color,
                Shape::Arrow(shape) => &mut shape.color,
                Shape::Pen(shape) => &mut shape.color,
                Shape::Text(shape) => &mut shape.color,
                Shape::Counter(shape) => &mut shape.color,
            };
            if *shape_color != color {
                *shape_color = color;
                self.commit();
            } else {
                let _ = self.history.cancel();
            }
        }
    }

    fn apply_width_to_selection(&mut self, width_index: usize) {
        self.stroke_width_index = width_index;
        if self.text_editing.is_some() {
            return;
        }
        let width = self.current_stroke_width();
        if let Some(index) = self.selected_index() {
            self.history.begin(&self.doc, None);
            match &mut self.doc.shapes[index] {
                Shape::Rectangle(shape) => shape.stroke_width = width,
                Shape::Ellipse(shape) => shape.stroke_width = width,
                Shape::Arrow(shape) => shape.stroke_width = width,
                Shape::Pen(shape) => shape.stroke_width = width,
                Shape::Text(_) | Shape::Counter(_) => {}
            }
            self.commit();
        }
    }

    fn undo(&mut self) {
        if self.text_editing.is_some() {
            return;
        }
        self.drag = Drag::None;
        if let Some(previous) = self.history.undo(&self.doc, Some(&self.rgba)) {
            self.doc = previous.doc;
            if let Some(image) = previous.image {
                self.rgba = image;
                self.rebuild_image_from_rgba();
                if let Err(error) = self.replace_library_image() {
                    eprintln!("Snipsy: {error}");
                }
            }
            if let Some(selected) = &self.selected
                && self.doc.index_of(selected).is_none()
            {
                self.selected = None;
            }
            self.persist();
        }
    }

    fn redo(&mut self) {
        if self.text_editing.is_some() {
            return;
        }
        self.drag = Drag::None;
        if let Some(next) = self.history.redo(&self.doc, Some(&self.rgba)) {
            self.doc = next.doc;
            if let Some(image) = next.image {
                self.rgba = image;
                self.rebuild_image_from_rgba();
                if let Err(error) = self.replace_library_image() {
                    eprintln!("Snipsy: {error}");
                }
            }
            if let Some(selected) = &self.selected
                && self.doc.index_of(selected).is_none()
            {
                self.selected = None;
            }
            self.persist();
        }
    }

    /// Rebuild the GPU texture after the RGBA buffer changed (crop, undo of
    /// a crop).
    fn rebuild_image_from_rgba(&mut self) {
        let mut bgra = self.rgba.clone();
        platform::rgba_to_bgra(bgra.as_mut());
        self.image = Arc::new(RenderImage::new(vec![image::Frame::new(bgra)]));
    }

    fn apply_crop(&mut self, viewport: Size<Pixels>) {
        let Some(crop) = self.crop else {
            return;
        };
        // Integer, in-bounds crop rect.
        let left = crop
            .x
            .min(crop.right())
            .clamp(0.0, self.doc.image_width as f32);
        let top = crop
            .y
            .min(crop.bottom())
            .clamp(0.0, self.doc.image_height as f32);
        let right = crop
            .x
            .max(crop.right())
            .clamp(0.0, self.doc.image_width as f32);
        let bottom = crop
            .y
            .max(crop.bottom())
            .clamp(0.0, self.doc.image_height as f32);
        if right <= left || bottom <= top {
            self.tool = Tool::Select;
            self.crop = None;
            return;
        }
        let x = left.round().clamp(0.0, (self.doc.image_width - 1) as f32) as u32;
        let y = top.round().clamp(0.0, (self.doc.image_height - 1) as f32) as u32;
        let right = right
            .round()
            .clamp((x + 1) as f32, self.doc.image_width as f32) as u32;
        let bottom = bottom
            .round()
            .clamp((y + 1) as f32, self.doc.image_height as f32) as u32;
        let w = right - x;
        let h = bottom - y;
        if w < 1
            || h < 1
            || (w == self.doc.image_width && h == self.doc.image_height && x == 0 && y == 0)
        {
            // Nothing to do; leave crop mode.
            self.tool = Tool::Select;
            return;
        }
        self.history.begin(&self.doc, Some(&self.rgba));
        self.doc
            .crop(&Rect::new(x as f32, y as f32, w as f32, h as f32));
        self.rgba = image::imageops::crop_imm(&self.rgba, x, y, w, h).to_image();
        self.rebuild_image_from_rgba();
        self.commit_with_image();

        // The library's stored capture is now stale — replace it so closing
        // and reopening the editor shows the same image.
        if let Err(error) = self.replace_library_image() {
            eprintln!("Snipsy: {error}");
        }

        self.tool = Tool::Select;
        self.selected = None;
        self.fit(viewport);
    }

    fn replace_library_image(&self) -> Result<(), String> {
        // The doc path is <library>/<id>/annotations.json; the id is the
        // parent directory name.
        let Some(path) = &self.doc_path else {
            return Ok(());
        };
        let Some(id) = path.parent().and_then(|parent| parent.file_name()) else {
            return Ok(());
        };
        let png = crate::capture::encode_png(
            self.doc.image_width,
            self.doc.image_height,
            self.rgba.as_raw(),
        )?;
        library::replace_image(
            &id.to_string_lossy(),
            &png,
            self.doc.image_width,
            self.doc.image_height,
        )
    }

    fn cancel_interaction(&mut self) {
        if self.text_editing.is_some() {
            self.commit_text_edit();
            return;
        }
        if self.tool == Tool::Crop {
            self.tool = Tool::Select;
            self.crop = None;
            self.drag = Drag::None;
            return;
        }
        if let Some(previous) = self.history.cancel() {
            self.doc = previous.doc;
            if let Some(image) = previous.image {
                self.rgba = image;
                self.rebuild_image_from_rgba();
            }
        }
        self.drag = Drag::None;
        if let Some(selected) = &self.selected
            && self.doc.index_of(selected).is_none()
        {
            self.selected = None;
        }
        self.persist();
    }

    fn copy_flattened(&mut self) {
        match self.end_flattened_png() {
            Ok(png) => {
                if let Err(error) = clipboard::copy_png_image(&png) {
                    eprintln!("Snipsy: {error}");
                }
            }
            Err(error) => eprintln!("Snipsy: {error}"),
        }
    }

    fn save_flattened(&mut self) {
        match self.end_flattened_png() {
            Ok(png) => {
                if let Err(error) = save_dialog::save_png_via_dialog(&png, &self.title) {
                    eprintln!("Snipsy: {error}");
                }
            }
            Err(error) => eprintln!("Snipsy: {error}"),
        }
    }
}

fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(byte_index, _)| byte_index)
        .unwrap_or(text.len())
}

fn next_shape_id(prefix: &str) -> ShapeId {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    format!(
        "{prefix}-{}-{}",
        library::now_millis(),
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

impl Render for EditorWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        if self.zoom <= 0.0 {
            self.fit(viewport);
        }

        let canvas_state = self.canvas_state();
        let focus = self.focus.clone();
        let has_selection = self.selected_index().is_some() && self.text_editing.is_none();

        let mut root = div()
            .id("editor-root")
            .relative()
            .size_full()
            .bg(rgb(0x1e1e22))
            .key_context(if self.text_editing.is_some() {
                "EditorText"
            } else {
                "EditorWindow"
            })
            .track_focus(&self.focus)
            .cursor(if self.text_editing.is_some() {
                gpui::CursorStyle::IBeam
            } else {
                self.tool.cursor()
            })
            .on_action(cx.listener(|this, _: &ToolSelect, _, cx| {
                this.set_tool(Tool::Select);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolRectangle, _, cx| {
                this.set_tool(Tool::Rectangle);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolEllipse, _, cx| {
                this.set_tool(Tool::Ellipse);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolArrow, _, cx| {
                this.set_tool(Tool::Arrow);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolPen, _, cx| {
                this.set_tool(Tool::Pen);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolText, _, cx| {
                this.set_tool(Tool::Text);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolCounter, _, cx| {
                this.set_tool(Tool::Counter);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolCrop, _, cx| {
                this.set_tool(Tool::Crop);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DeleteShape, _, cx| {
                this.delete_selected();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &UndoEdit, _, cx| {
                this.undo();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &RedoEdit, _, cx| {
                this.redo();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CopyFlattened, _, _| this.copy_flattened()))
            .on_action(cx.listener(|this, _: &SaveFlattened, _, _| this.save_flattened()))
            .on_action(cx.listener(|this, _: &BringForwardShape, _, cx| {
                this.reorder_selected(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SendBackwardShape, _, cx| {
                this.reorder_selected(false);
                cx.notify();
            }))
            .on_action(cx.listener(move |this, _: &ZoomIn, _, cx| {
                let center = size_to_point(viewport);
                this.zoom_at(1.2, center);
                cx.notify();
            }))
            .on_action(cx.listener(move |this, _: &ZoomOut, _, cx| {
                let center = size_to_point(viewport);
                this.zoom_at(1.0 / 1.2, center);
                cx.notify();
            }))
            .on_action(cx.listener(move |this, _: &ZoomFit, _, cx| {
                this.fit(viewport);
                cx.notify();
            }))
            .on_action(cx.listener(move |this, _: &ApplyCrop, window, cx| {
                this.apply_crop(viewport);
                cx.notify();
                let _ = window;
            }))
            .on_action(cx.listener(|this, _: &CancelInteraction, _, cx| {
                this.cancel_interaction();
                cx.notify();
            }))
            .on_action(cx.listener(|_, _: &CloseEditor, window, _| {
                window.remove_window();
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                this.on_key_down_raw(event);
                cx.notify();
            }))
            .on_key_up(cx.listener(|this, event: &gpui::KeyUpEvent, _, cx| {
                this.on_key_up_raw(event);
                cx.notify();
            }))
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let factor = match event.delta {
                    gpui::ScrollDelta::Lines(lines) => (1.0 + lines.y * -0.1).clamp(0.5, 2.0),
                    gpui::ScrollDelta::Pixels(pixels) => {
                        (1.0 + f32::from(pixels.y) * -0.005).clamp(0.5, 2.0)
                    }
                };
                this.zoom_at(factor, event.position);
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, _| {
                    if in_chrome(event.position, viewport, has_selection) {
                        return;
                    }
                    this.on_canvas_mouse_down(event);
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, event: &MouseDownEvent, _, _| {
                    if !in_chrome(event.position, viewport, has_selection) {
                        this.on_canvas_mouse_down(event);
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.drag_needs_move() {
                    this.on_canvas_mouse_move(event);
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.on_canvas_mouse_up(event);
                    cx.notify();
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, _: &MouseUpEvent, _, _| {
                    this.drag = Drag::None;
                }),
            );

        // The canvas paints everything geometric; text renders as divs above.
        let canvas_bounds_for_hit = Bounds {
            origin: Point::default(),
            size: Size {
                width: viewport.width,
                height: viewport.height - px(TOOLBAR_HEIGHT),
            },
        };
        root = root
            .child(
                canvas(
                    |_, _, _| canvas_state,
                    move |bounds, state, window, _| {
                        crate::editor::window::paint_canvas(bounds, &state, window);
                    },
                )
                .size_full(),
            )
            .children(self.text_overlays())
            .children(self.crop_status())
            .child(self.toolbar(cx))
            .children(self.selection_bar(cx))
            .child(self.zoom_indicator(viewport))
            .child({
                let _ = (canvas_bounds_for_hit, focus);
                div()
            });
        root
    }
}

fn size_to_point(size: Size<Pixels>) -> Point<Pixels> {
    Point {
        x: size.width / 2.0,
        y: size.height / 2.0,
    }
}

fn in_chrome(position: Point<Pixels>, viewport: Size<Pixels>, has_selection: bool) -> bool {
    let bar_height = TOOLBAR_HEIGHT
        + if has_selection {
            SELECTION_BAR_HEIGHT
        } else {
            0.0
        };
    position.y >= viewport.height - px(bar_height)
}

impl EditorWindow {
    fn drag_needs_move(&self) -> bool {
        !matches!(self.drag, Drag::None)
    }

    /// Everything the canvas paint closure needs, resolved at render time.
    fn canvas_state(&self) -> CanvasState {
        CanvasState {
            image: self.image.clone(),
            image_width: self.doc.image_width,
            image_height: self.doc.image_height,
            zoom: self.zoom,
            pan: self.pan,
            shapes: self.doc.shapes.clone(),
            preview: self.preview_shape(),
            selected: self.selected.clone(),
            space_or_pan: self.space_held,
            crop: if self.tool == Tool::Crop {
                self.crop
            } else {
                None
            },
        }
    }

    /// The shape currently being drawn, for live preview.
    fn preview_shape(&self) -> Option<Shape> {
        match &self.drag {
            Drag::Draw { start, current } => {
                let color = self.current_color();
                let stroke_width = self.current_stroke_width();
                Some(match self.tool {
                    Tool::Rectangle => Shape::Rectangle(RectShape {
                        id: "preview".to_string(),
                        x: start.0.min(current.0),
                        y: start.1.min(current.1),
                        w: (start.0 - current.0).abs(),
                        h: (start.1 - current.1).abs(),
                        color,
                        stroke_width,
                    }),
                    Tool::Ellipse => Shape::Ellipse(EllipseShape {
                        id: "preview".to_string(),
                        x: start.0.min(current.0),
                        y: start.1.min(current.1),
                        w: (start.0 - current.0).abs(),
                        h: (start.1 - current.1).abs(),
                        color,
                        stroke_width,
                    }),
                    Tool::Arrow => Shape::Arrow(ArrowShape {
                        id: "preview".to_string(),
                        x1: start.0,
                        y1: start.1,
                        x2: current.0,
                        y2: current.1,
                        color,
                        stroke_width,
                    }),
                    _ => return None,
                })
            }
            Drag::PenDraft { points } if points.len() >= 2 => Some(Shape::Pen(PenShape {
                id: "preview".to_string(),
                points: points.clone(),
                color: self.current_color(),
                stroke_width: self.current_stroke_width(),
            })),
            _ => None,
        }
    }

    /// Text shapes (and counter digits) as absolutely-positioned divs.
    fn text_overlays(&self) -> Vec<gpui::AnyElement> {
        let mut overlays = Vec::new();
        for shape in &self.doc.shapes {
            match shape {
                Shape::Text(text) => {
                    let left = self.to_screen_x(text.x);
                    let top = self.to_screen_y(text.y);
                    let font_px = (text.size * self.zoom).max(6.0);
                    let is_editing = self.text_editing.as_ref() == Some(&text.id);
                    let color = parse_gpui_color(&text.color);
                    let mut styled = div()
                        .absolute()
                        .left(left)
                        .top(top)
                        .font_family(default_font_family())
                        .text_size(px(font_px))
                        .text_color(color);
                    if is_editing {
                        // Editing box: render the text with a caret block.
                        let before: String = text.text.chars().take(self.caret).collect();
                        let after: String = text.text.chars().skip(self.caret).collect();
                        styled = styled
                            .border_1()
                            .border_color(rgb(0x3B82F6))
                            .bg(rgb(0x000000).alpha(0.25))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .text_color(color)
                                    .child(before.clone())
                                    .child(
                                        div()
                                            .w(px(2.0))
                                            .h(px(font_px))
                                            .bg(rgb(0xffffff))
                                            .text_color(color),
                                    )
                                    .child(after.clone()),
                            );
                    } else {
                        styled = styled.child(
                            div().flex().flex_col().children(
                                text.text
                                    .split('\n')
                                    .map(|line| div().child(line.to_string())),
                            ),
                        );
                    }
                    overlays.push(styled.into_any_element());
                }
                Shape::Counter(counter) => {
                    let diameter = counter.diameter * self.zoom;
                    let left = self.to_screen_x(counter.x);
                    let top = self.to_screen_y(counter.y);
                    overlays.push(
                        div()
                            .absolute()
                            .left(left)
                            .top(top)
                            .w(px(diameter))
                            .h(px(diameter))
                            .flex()
                            .items_center()
                            .justify_center()
                            .font_family(default_font_family())
                            .text_size(px((diameter * 0.55).max(8.0)))
                            .text_color(rgb(0xffffff))
                            .child(counter.n.to_string())
                            .into_any_element(),
                    );
                }
                _ => {}
            }
        }
        overlays
    }

    /// A small hint line while the crop tool is active.
    fn crop_status(&self) -> Option<gpui::AnyElement> {
        if self.tool != Tool::Crop {
            return None;
        }
        Some(
            div()
                .absolute()
                .top(px(8.0))
                .left(px(12.0))
                .px_2()
                .py_1()
                .rounded(px(4.0))
                .bg(rgb(0x000000).alpha(0.7))
                .text_size(px(12.0))
                .text_color(rgb(0xffffff))
                .font_family(default_font_family())
                .child("Drag a region · Ctrl+Enter to crop · Esc to cancel")
                .into_any_element(),
        )
    }

    fn zoom_indicator(&self, viewport: Size<Pixels>) -> gpui::AnyElement {
        let _ = viewport;
        let percent = (self.zoom * 100.0).round() as i32;
        div()
            .absolute()
            .right(px(12.0))
            .bottom(px(TOOLBAR_HEIGHT + 8.0))
            .px_2()
            .py_1()
            .rounded(px(4.0))
            .bg(rgb(0x000000).alpha(0.6))
            .text_size(px(12.0))
            .text_color(rgb(0xffffff))
            .font_family(default_font_family())
            .child(format!("{percent}% · 0 to fit"))
            .into_any_element()
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let tools = [
            (Tool::Select, "1", "Select"),
            (Tool::Rectangle, "2", "Rectangle"),
            (Tool::Ellipse, "3", "Ellipse"),
            (Tool::Arrow, "4", "Arrow"),
            (Tool::Pen, "5", "Pen"),
            (Tool::Text, "6", "Text"),
            (Tool::Counter, "7", "Counter"),
            (Tool::Crop, "8", "Crop"),
        ];
        let mut bar = div()
            .absolute()
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .h(px(TOOLBAR_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .px_2()
            .gap_1()
            .bg(rgb(0x2b2b31))
            .border_t_1()
            .border_color(rgb(0x000000).alpha(0.4));

        for (tool, shortcut, label) in tools {
            let active = self.tool == tool;
            let mut button = div()
                .id(gpui::ElementId::Name(label.to_string().into()))
                .w(px(38.0))
                .h(px(38.0))
                .rounded(px(6.0))
                .flex()
                .items_center()
                .justify_center()
                .relative()
                .cursor(gpui::CursorStyle::PointingHand)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        this.set_tool(tool);
                        cx.notify();
                    }),
                )
                .child(icon_for(tool, active))
                .child(
                    div()
                        .absolute()
                        .right(px(2.0))
                        .bottom(px(1.0))
                        .text_size(px(9.0))
                        .text_color(rgb(0xffffff).alpha(0.45))
                        .font_family(default_font_family())
                        .child(shortcut),
                );
            if active {
                button = button.bg(rgb(0x3B82F6));
            }
            bar = bar.child(button);
        }

        // Export actions at the right end of the bar.
        bar = bar
            .child(div().flex_1())
            .child(
                small_button("Undo")
                    .opacity(if self.history.can_undo() { 1.0 } else { 0.45 })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            this.undo();
                            cx.notify();
                        }),
                    ),
            )
            .child(
                small_button("Redo")
                    .opacity(if self.history.can_redo() { 1.0 } else { 0.45 })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            this.redo();
                            cx.notify();
                        }),
                    ),
            )
            .child(small_button("Copy").on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, _| {
                    this.copy_flattened();
                }),
            ))
            .child(small_button("Save PNG").on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, _| {
                    this.save_flattened();
                }),
            ));
        bar.into_any_element()
    }

    /// Color / width / z-order / delete bar shown while a shape is selected.
    fn selection_bar(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if self.selected_index().is_none() || self.text_editing.is_some() {
            return None;
        }
        let selected_color_index = self.selected_color_index();
        let selected_width_index = self.selected_width_index();
        let mut bar = div()
            .absolute()
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(TOOLBAR_HEIGHT))
            .h(px(SELECTION_BAR_HEIGHT))
            .flex()
            .flex_row()
            .items_center()
            .px_3()
            .gap_2()
            .bg(rgb(0x2b2b31).alpha(0.95))
            .border_t_1()
            .border_color(rgb(0x000000).alpha(0.4));

        for (index, color) in PALETTE.iter().enumerate() {
            let swatch = rgb(parse_hex(color));
            let mut chip = div()
                .id(gpui::ElementId::Name(format!("color-{index}").into()))
                .w(px(20.0))
                .h(px(20.0))
                .rounded(px(4.0))
                .bg(swatch)
                .cursor(gpui::CursorStyle::PointingHand)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        this.apply_color_to_selection(index);
                        cx.notify();
                    }),
                );
            if selected_color_index == Some(index) {
                chip = chip.border_2().border_color(rgb(0xffffff));
            }
            bar = bar.child(chip);
        }

        bar = bar.child(div().w(px(8.0)).h(px(0.0)));

        if selected_width_index.is_some() {
            for (index, label) in ["S", "M", "L", "XL"].iter().enumerate() {
                let active = selected_width_index == Some(index);
                let mut chip = div()
                    .id(gpui::ElementId::Name(format!("width-{index}").into()))
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .cursor(gpui::CursorStyle::PointingHand)
                    .text_size(px(12.0))
                    .font_family(default_font_family())
                    .text_color(if active {
                        rgb(0xffffff)
                    } else {
                        rgb(0xffffff).alpha(0.55)
                    })
                    .child(*label)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            this.apply_width_to_selection(index);
                            cx.notify();
                        }),
                    );
                if active {
                    chip = chip.bg(rgb(0x3B82F6));
                }
                bar = bar.child(chip);
            }
        }

        bar = bar
            .child(div().flex_1())
            .child(small_button("◀ Back").on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.reorder_selected(false);
                    cx.notify();
                }),
            ))
            .child(small_button("Forward ▶").on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.reorder_selected(true);
                    cx.notify();
                }),
            ))
            .child(small_button("× Delete").on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.delete_selected();
                    cx.notify();
                }),
            ));
        Some(bar.into_any_element())
    }

    /// The palette index of the selected shape's color, if it matches one.
    fn selected_color_index(&self) -> Option<usize> {
        let index = self.selected_index()?;
        let color = match &self.doc.shapes[index] {
            Shape::Rectangle(shape) => &shape.color,
            Shape::Ellipse(shape) => &shape.color,
            Shape::Arrow(shape) => &shape.color,
            Shape::Pen(shape) => &shape.color,
            Shape::Text(shape) => &shape.color,
            Shape::Counter(shape) => &shape.color,
        };
        PALETTE.iter().position(|candidate| candidate == color)
    }

    /// The width bucket of the selected shape, if it is one of S/M/L/XL.
    fn selected_width_index(&self) -> Option<usize> {
        let index = self.selected_index()?;
        let width = match &self.doc.shapes[index] {
            Shape::Rectangle(shape) => shape.stroke_width,
            Shape::Ellipse(shape) => shape.stroke_width,
            Shape::Arrow(shape) => shape.stroke_width,
            Shape::Pen(shape) => shape.stroke_width,
            Shape::Text(_) | Shape::Counter(_) => return None,
        };
        STROKE_WIDTHS
            .iter()
            .position(|candidate| (*candidate - width).abs() < 0.01)
    }
}

fn small_button(label: &'static str) -> gpui::Div {
    div()
        .px_2()
        .py_1()
        .rounded(px(4.0))
        .bg(rgb(0x3a3a42))
        .text_size(px(12.0))
        .text_color(rgb(0xffffff))
        .font_family(default_font_family())
        .cursor(gpui::CursorStyle::PointingHand)
        .child(label)
}

fn default_font_family() -> &'static str {
    #[cfg(windows)]
    {
        "Segoe UI"
    }
    #[cfg(not(windows))]
    {
        "DejaVu Sans"
    }
}

fn parse_hex(color: &str) -> u32 {
    u32::from_str_radix(color.trim_start_matches('#'), 16).unwrap_or(0x000000)
}

fn parse_gpui_color(color: &str) -> gpui::Hsla {
    let value = parse_hex(color);
    rgb(value).into()
}

// ---- canvas painting -----------------------------------------------------

struct CanvasState {
    image: Arc<RenderImage>,
    image_width: u32,
    image_height: u32,
    zoom: f32,
    pan: Point<Pixels>,
    shapes: Vec<Shape>,
    preview: Option<Shape>,
    selected: Option<ShapeId>,
    #[allow(dead_code)]
    space_or_pan: bool,
    crop: Option<Rect>,
}

fn paint_canvas(bounds: Bounds<Pixels>, state: &CanvasState, window: &mut Window) {
    // Canvas background behind the image.
    window.paint_quad(gpui::quad(
        bounds,
        px(0.),
        rgb(0x1e1e22),
        px(0.),
        gpui::transparent_black(),
        Default::default(),
    ));

    let image_bounds = Bounds {
        origin: Point {
            x: bounds.origin.x + state.pan.x,
            y: bounds.origin.y + state.pan.y,
        },
        size: Size {
            width: px(state.image_width as f32 * state.zoom),
            height: px(state.image_height as f32 * state.zoom),
        },
    };
    // Shadow behind the image so its edges read on any background.
    window.paint_quad(gpui::quad(
        Bounds {
            origin: Point {
                x: image_bounds.origin.x - px(1.0),
                y: image_bounds.origin.y - px(1.0),
            },
            size: Size {
                width: image_bounds.size.width + px(2.0),
                height: image_bounds.size.height + px(2.0),
            },
        },
        px(0.),
        gpui::transparent_black(),
        px(0.),
        rgb(0xffffff).alpha(0.15),
        Default::default(),
    ));
    window
        .paint_image(
            bounds,
            image_bounds,
            Corners::<Pixels>::default(),
            state.image.clone(),
            0,
            false,
        )
        .ok();

    // Shapes, document order (later on top).
    for shape in state.shapes.iter().chain(state.preview.as_ref()) {
        let selected = state.selected.as_deref() == Some(shape.id().as_str());
        paint_shape(bounds, state, window, shape, selected);
    }

    // Selection outline + handles.
    if let Some(id) = &state.selected
        && let Some(shape) = state.shapes.iter().find(|shape| shape.id() == id)
    {
        let screen = shape_bounds_to_screen(shape, state, bounds);
        paint_selection_ui(window, &screen, state.zoom);
    }

    // Crop overlay.
    if let Some(crop) = state.crop {
        paint_crop_overlay(bounds, state, window, crop);
    }
}

// Dimensions remain available for viewport recalculation and preserve the
// original-resolution invariant alongside the render texture.

fn shape_bounds_to_screen(shape: &Shape, state: &CanvasState, bounds: Bounds<Pixels>) -> Rect {
    let doc_bounds = shape.bounds();
    Rect::new(
        f32::from(bounds.origin.x + state.pan.x) + doc_bounds.x * state.zoom,
        f32::from(bounds.origin.y + state.pan.y) + doc_bounds.y * state.zoom,
        doc_bounds.w * state.zoom,
        doc_bounds.h * state.zoom,
    )
}

fn image_to_screen(state: &CanvasState, bounds: Bounds<Pixels>, x: f32, y: f32) -> Point<Pixels> {
    Point {
        x: bounds.origin.x + state.pan.x + px(x * state.zoom),
        y: bounds.origin.y + state.pan.y + px(y * state.zoom),
    }
}

fn screen_stroke_width(width: f32, state: &CanvasState) -> Pixels {
    px((width * state.zoom).max(1.0))
}

fn paint_shape(
    _bounds: Bounds<Pixels>,
    state: &CanvasState,
    window: &mut Window,
    shape: &Shape,
    _selected: bool,
) {
    let color = shape_color(shape);
    match shape {
        Shape::Rectangle(rect) => {
            if let Some(path) = rect_path(
                bounds_of_canvas(rect.x, rect.y, rect.w, rect.h, _bounds, state),
                screen_stroke_width(rect.stroke_width, state),
            ) {
                window.paint_path(path, color);
            }
        }
        Shape::Ellipse(ellipse) => {
            if let Some(path) = ellipse_screen_path(
                _bounds,
                state,
                Rect::new(ellipse.x, ellipse.y, ellipse.w, ellipse.h),
                screen_stroke_width(ellipse.stroke_width, state),
                false,
            ) {
                window.paint_path(path, color);
            }
        }
        Shape::Arrow(arrow) => {
            let head_size = ((arrow.stroke_width * 4.0).max(12.0)) * state.zoom;
            let (x1, y1) = (arrow.x1, arrow.y1);
            let (x2, y2) = (arrow.x2, arrow.y2);
            // Line.
            let mut builder = PathBuilder::stroke(screen_stroke_width(arrow.stroke_width, state));
            let start = image_to_screen(state, _bounds, x1, y1);
            let end = image_to_screen(state, _bounds, x2, y2);
            builder.move_to(start);
            builder.line_to(end);
            if let Ok(path) = builder.build() {
                window.paint_path(path, color);
            }
            // Head (filled triangle).
            let head = arrow.head_triangle(head_size / state.zoom);
            let mut builder = PathBuilder::fill();
            let points = head.map(|(hx, hy)| image_to_screen(state, _bounds, hx, hy));
            builder.move_to(points[0]);
            builder.line_to(points[1]);
            builder.line_to(points[2]);
            builder.close();
            if let Ok(path) = builder.build() {
                window.paint_path(path, color);
            }
        }
        Shape::Pen(pen) => {
            if pen.points.len() >= 2 {
                let mut builder = PathBuilder::stroke(screen_stroke_width(pen.stroke_width, state));
                let first = image_to_screen(state, _bounds, pen.points[0].0, pen.points[0].1);
                builder.move_to(first);
                for (x, y) in &pen.points[1..] {
                    builder.line_to(image_to_screen(state, _bounds, *x, *y));
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, color);
                }
            }
        }
        // Text renders as divs above the canvas; counters paint their badge
        // here and the digit as a div.
        Shape::Text(_) => {}
        Shape::Counter(counter) => {
            if let Some(path) = ellipse_screen_path(
                _bounds,
                state,
                Rect::new(counter.x, counter.y, counter.diameter, counter.diameter),
                px(0.0),
                true,
            ) {
                window.paint_path(path, color);
            }
            let ring = ellipse_screen_path(
                _bounds,
                state,
                Rect::new(
                    counter.x + counter.diameter * 0.0625,
                    counter.y + counter.diameter * 0.0625,
                    counter.diameter * 0.875,
                    counter.diameter * 0.875,
                ),
                px(1.0),
                false,
            );
            // A filled badge plus a thin white ring reads on any background.
            if let Some(ring) = ring {
                window.paint_path(ring, rgb(0xffffff));
            }
        }
    }
}

fn bounds_of_canvas(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    bounds: Bounds<Pixels>,
    state: &CanvasState,
) -> Bounds<Pixels> {
    Bounds {
        origin: image_to_screen(state, bounds, x, y),
        size: Size {
            width: px(w * state.zoom),
            height: px(h * state.zoom),
        },
    }
}

/// A stroked rectangle path in screen coordinates.
fn rect_path(rect: Bounds<Pixels>, stroke: Pixels) -> Option<gpui::Path<Pixels>> {
    let mut builder = PathBuilder::stroke(stroke);
    builder.move_to(rect.origin);
    builder.line_to(Point {
        x: rect.right(),
        y: rect.origin.y,
    });
    builder.line_to(Point {
        x: rect.right(),
        y: rect.bottom(),
    });
    builder.line_to(Point {
        x: rect.origin.x,
        y: rect.bottom(),
    });
    builder.close();
    builder.build().ok()
}

fn ellipse_screen_path(
    bounds: Bounds<Pixels>,
    state: &CanvasState,
    rect: Rect,
    stroke: Pixels,
    fill: bool,
) -> Option<gpui::Path<Pixels>> {
    const SEGMENTS: usize = 64;
    let cx = rect.x + rect.w / 2.0;
    let cy = rect.y + rect.h / 2.0;
    let rx = (rect.w / 2.0).max(0.5);
    let ry = (rect.h / 2.0).max(0.5);
    let mut builder = if fill {
        PathBuilder::fill()
    } else {
        PathBuilder::stroke(stroke)
    };
    for index in 0..SEGMENTS {
        let angle = index as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let point = image_to_screen(state, bounds, cx + angle.cos() * rx, cy + angle.sin() * ry);
        if index == 0 {
            builder.move_to(point);
        } else {
            builder.line_to(point);
        }
    }
    builder.close();
    builder.build().ok()
}

fn paint_selection_ui(window: &mut Window, screen_bounds: &Rect, _zoom: f32) {
    let handle_screen = px(HANDLE_SCREEN_SIZE);
    // Outline.
    let mut builder = PathBuilder::stroke(px(1.5));
    builder.move_to(Point {
        x: px(screen_bounds.x),
        y: px(screen_bounds.y),
    });
    builder.line_to(Point {
        x: px(screen_bounds.right()),
        y: px(screen_bounds.y),
    });
    builder.line_to(Point {
        x: px(screen_bounds.right()),
        y: px(screen_bounds.bottom()),
    });
    builder.line_to(Point {
        x: px(screen_bounds.x),
        y: px(screen_bounds.bottom()),
    });
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, rgb(0x3B82F6));
    }
    // Handles: white squares with a blue border.
    for handle in Handle::ALL {
        let (hx, hy) = handle.position(screen_bounds);
        let quad_bounds = Bounds {
            origin: Point {
                x: px(hx) - handle_screen / 2.0,
                y: px(hy) - handle_screen / 2.0,
            },
            size: Size {
                width: handle_screen,
                height: handle_screen,
            },
        };
        window.paint_quad(gpui::quad(
            quad_bounds,
            px(2.0),
            rgb(0xffffff),
            px(1.5),
            rgb(0x3B82F6),
            Default::default(),
        ));
    }
}

fn paint_crop_overlay(
    bounds: Bounds<Pixels>,
    state: &CanvasState,
    window: &mut Window,
    crop: Rect,
) {
    let left = f32::from(bounds.origin.x + state.pan.x) + crop.x * state.zoom;
    let top = f32::from(bounds.origin.y + state.pan.y) + crop.y * state.zoom;
    let right = left + crop.w * state.zoom;
    let bottom = top + crop.h * state.zoom;
    let dim = rgb(0x000000).alpha(0.55);
    // Four dim quads around the crop rect.
    for region in [
        Rect::new(
            f32::from(bounds.origin.x),
            f32::from(bounds.origin.y),
            f32::from(bounds.size.width),
            top - f32::from(bounds.origin.y),
        ),
        Rect::new(
            f32::from(bounds.origin.x),
            bottom,
            f32::from(bounds.size.width),
            f32::from(bounds.origin.y + bounds.size.height) - bottom,
        ),
        Rect::new(
            f32::from(bounds.origin.x),
            top,
            left - f32::from(bounds.origin.x),
            bottom - top,
        ),
        Rect::new(
            right,
            top,
            f32::from(bounds.origin.x + bounds.size.width) - right,
            bottom - top,
        ),
    ] {
        if region.w > 0.0 && region.h > 0.0 {
            window.paint_quad(gpui::quad(
                Bounds {
                    origin: Point {
                        x: px(region.x),
                        y: px(region.y),
                    },
                    size: Size {
                        width: px(region.w),
                        height: px(region.h),
                    },
                },
                px(0.),
                dim,
                px(0.),
                gpui::transparent_black(),
                Default::default(),
            ));
        }
    }
    // Border.
    let mut builder = PathBuilder::stroke(px(1.5));
    builder.move_to(Point {
        x: px(left),
        y: px(top),
    });
    builder.line_to(Point {
        x: px(right),
        y: px(top),
    });
    builder.line_to(Point {
        x: px(right),
        y: px(bottom),
    });
    builder.line_to(Point {
        x: px(left),
        y: px(bottom),
    });
    builder.close();
    if let Ok(path) = builder.build() {
        window.paint_path(path, rgb(0xffffff));
    }
    // Rule-of-thirds grid lines, lightly.
    for fraction in [1.0 / 3.0, 2.0 / 3.0] {
        let mut builder = PathBuilder::stroke(px(1.0));
        builder.move_to(Point {
            x: px(left + crop.w * state.zoom * fraction),
            y: px(top),
        });
        builder.line_to(Point {
            x: px(left + crop.w * state.zoom * fraction),
            y: px(bottom),
        });
        builder.move_to(Point {
            x: px(left),
            y: px(top + crop.h * state.zoom * fraction),
        });
        builder.line_to(Point {
            x: px(right),
            y: px(top + crop.h * state.zoom * fraction),
        });
        if let Ok(path) = builder.build() {
            window.paint_path(path, rgb(0xffffff).alpha(0.35));
        }
    }
    let crop_bounds = Rect::new(left, top, right - left, bottom - top);
    paint_selection_ui(window, &crop_bounds, state.zoom);
}

fn shape_color(shape: &Shape) -> gpui::Hsla {
    let hex = match shape {
        Shape::Rectangle(shape) => &shape.color,
        Shape::Ellipse(shape) => &shape.color,
        Shape::Arrow(shape) => &shape.color,
        Shape::Pen(shape) => &shape.color,
        Shape::Text(shape) => &shape.color,
        Shape::Counter(shape) => &shape.color,
    };
    rgb(parse_hex(hex)).into()
}

/// Toolbar/selection icons drawn as vector shapes.
fn icon_for(tool: Tool, active: bool) -> gpui::AnyElement {
    let color = if active { 0xffffff } else { 0xd0d0d6 };
    canvas(
        move |_, _, _| {},
        move |bounds, _, window, _| {
            let c = rgb(color);
            let x = f32::from(bounds.origin.x);
            let y = f32::from(bounds.origin.y);
            let w = f32::from(bounds.size.width);
            let h = f32::from(bounds.size.height);
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            let mut path = |points: &[(f32, f32)], close: bool, stroke_w: f32| {
                let mut builder = PathBuilder::stroke(px(stroke_w));
                if let Some(first) = points.first() {
                    builder.move_to(Point {
                        x: px(x + first.0 * w),
                        y: px(y + first.1 * h),
                    });
                    for point in &points[1..] {
                        builder.line_to(Point {
                            x: px(x + point.0 * w),
                            y: px(y + point.1 * h),
                        });
                    }
                    if close {
                        builder.close();
                    }
                }
                builder.build().ok().map(|path| window.paint_path(path, c))
            };
            match tool {
                Tool::Select => {
                    // Pointer arrow.
                    let fill_points: Vec<(f32, f32)> = vec![
                        (0.28, 0.15),
                        (0.28, 0.75),
                        (0.44, 0.6),
                        (0.54, 0.82),
                        (0.62, 0.78),
                        (0.52, 0.57),
                        (0.72, 0.57),
                    ];
                    let mut builder = PathBuilder::fill();
                    builder.move_to(Point {
                        x: px(x + fill_points[0].0 * w),
                        y: px(y + fill_points[0].1 * h),
                    });
                    for point in &fill_points[1..] {
                        builder.line_to(Point {
                            x: px(x + point.0 * w),
                            y: px(y + point.1 * h),
                        });
                    }
                    builder.close();
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, c);
                    }
                }
                Tool::Rectangle => {
                    path(
                        &[(0.25, 0.25), (0.75, 0.25), (0.75, 0.75), (0.25, 0.75)],
                        true,
                        2.0,
                    );
                }
                Tool::Ellipse => {
                    const SEGMENTS: usize = 40;
                    let mut builder = PathBuilder::stroke(px(2.0));
                    for index in 0..SEGMENTS {
                        let angle = index as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                        let px_ = cx + angle.cos() * w * 0.25;
                        let py = cy + angle.sin() * h * 0.25;
                        if index == 0 {
                            builder.move_to(Point {
                                x: px(px_),
                                y: px(py),
                            });
                        } else {
                            builder.line_to(Point {
                                x: px(px_),
                                y: px(py),
                            });
                        }
                    }
                    builder.close();
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, c);
                    }
                }
                Tool::Arrow => {
                    path(&[(0.25, 0.75), (0.7, 0.3)], false, 2.0);
                    let mut builder = PathBuilder::fill();
                    builder.move_to(Point {
                        x: px(x + 0.78 * w),
                        y: px(y + 0.22 * h),
                    });
                    builder.line_to(Point {
                        x: px(x + 0.6 * w),
                        y: px(y + 0.26 * h),
                    });
                    builder.line_to(Point {
                        x: px(x + 0.74 * w),
                        y: px(y + 0.4 * h),
                    });
                    builder.close();
                    if let Ok(head) = builder.build() {
                        window.paint_path(head, c);
                    }
                }
                Tool::Pen => {
                    path(
                        &[(0.25, 0.7), (0.4, 0.35), (0.55, 0.6), (0.75, 0.3)],
                        false,
                        2.0,
                    );
                }
                Tool::Text => {
                    path(&[(0.3, 0.3), (0.7, 0.3)], false, 2.0);
                    path(&[(0.5, 0.3), (0.5, 0.78)], false, 2.0);
                }
                Tool::Counter => {
                    const SEGMENTS: usize = 32;
                    let mut builder = PathBuilder::fill();
                    for index in 0..SEGMENTS {
                        let angle = index as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                        let px_ = cx + angle.cos() * w * 0.28;
                        let py = cy + angle.sin() * h * 0.28;
                        if index == 0 {
                            builder.move_to(Point {
                                x: px(px_),
                                y: px(py),
                            });
                        } else {
                            builder.line_to(Point {
                                x: px(px_),
                                y: px(py),
                            });
                        }
                    }
                    builder.close();
                    if let Ok(path) = builder.build() {
                        window.paint_path(path, c);
                    }
                }
                Tool::Crop => {
                    path(&[(0.35, 0.15), (0.35, 0.65), (0.85, 0.65)], false, 2.0);
                    path(&[(0.15, 0.35), (0.65, 0.35), (0.65, 0.85)], false, 2.0);
                }
            }
        },
    )
    .size_full()
    .into_any_element()
}
