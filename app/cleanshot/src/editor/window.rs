//! The annotation editor window: the captured image on a zoom/pan canvas,
//! shape tools, selection with handles, undo/redo, crop, and export.
//!
//! Coordinate spaces, kept strictly apart (see `platform.rs` notes):
//! - **image px** — the capture's original resolution; all document geometry.
//! - **screen px (GPUI logical)** — window-relative; `screen = pan + image × zoom`.
//!   Export flattens 1:1 in image px via `flatten`, never screen px.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    App, Bounds, Context, Corners, FocusHandle, KeyBinding, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Point, Render, RenderImage,
    ScrollWheelEvent, Size, Window, WindowBounds, WindowHandle, WindowOptions, actions, canvas,
    div, prelude::*, px, rgb, size,
};

use crate::clipboard;
use crate::editor::flatten::flatten_to_png;
use crate::editor::history::History;
use crate::editor::hit_test::{
    ArrowEndpoint, Handle, arrow_endpoint_at, handle_at, hit_test, shapes_intersecting,
};
use crate::editor::icons::IconGlyph;
use crate::editor::model::{
    ARROW_STROKE_WIDTHS, ArrowShape, COUNTER_DIAMETER, CounterShape, DEFAULT_TEXT_SIZE, Document,
    EffectKind, EffectShape, EllipseShape, IconShape, PALETTE, PenShape, Rect, RectShape,
    STROKE_WIDTHS, Shape, ShapeId, TextShape,
};
use crate::library;
use crate::settings::{AppSettings, current_settings};
use crate::theme::{R_BTN, R_INPUT, R_PANEL, R_PILL, R_THUMB, Theme};
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
        ToolBlur,
        ToolPixelate,
        ToolRedact,
        ToolIcon,
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

const TOOLBAR_HEIGHT: f32 = 52.0;
const TOOLBAR_BOTTOM: f32 = 18.0;
const SELECTION_BAR_HEIGHT: f32 = 44.0;
const SIDE_PANEL_WIDTH: f32 = 284.0;
const ICON_CACHE_MAX_BYTES: usize = 16 * 1024 * 1024;
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
    Blur,
    Pixelate,
    Redact,
    Icon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorPanel {
    Icons,
    Library,
    Preferences,
}

impl Tool {
    fn icon_slug(self) -> &'static str {
        match self {
            Tool::Select => "select",
            Tool::Rectangle => "rectangle",
            Tool::Ellipse => "ellipse",
            Tool::Arrow => "arrow",
            Tool::Pen => "pen",
            Tool::Text => "text",
            Tool::Counter => "counter",
            Tool::Crop => "crop",
            Tool::Blur => "blur",
            Tool::Pixelate => "pixelate",
            Tool::Redact => "redact",
            Tool::Icon => "sticker",
        }
    }

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
    MoveMany {
        originals: Vec<Shape>,
        start: (f32, f32),
    },
    ArrowEndpoint {
        id: ShapeId,
        original: ArrowShape,
        endpoint: ArrowEndpoint,
    },
    Marquee {
        start: (f32, f32),
        current: (f32, f32),
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
    pub annotations: library::AnnotationSource,
}

#[derive(Default)]
pub struct EditorSession {
    window: Option<WindowHandle<EditorWindow>>,
}

impl gpui::Global for EditorSession {}

struct PreparedCapture {
    title: String,
    image: Arc<RenderImage>,
    rgba: image::RgbaImage,
    doc: Document,
    doc_path: Option<PathBuf>,
    annotation_warning: bool,
}

pub struct EditorWindow {
    focus: FocusHandle,
    title: String,
    image: Arc<RenderImage>,
    /// The base image, RGBA, at the document's resolution.
    rgba: image::RgbaImage,
    doc: Document,
    doc_path: Option<PathBuf>,
    annotation_warning: bool,
    history: History,
    tool: Tool,
    color_index: usize,
    stroke_width_index: usize,
    icon_glyph: IconGlyph,
    panel: Option<EditorPanel>,
    captures: Vec<library::CaptureMetadata>,
    library_error: Option<String>,
    library_notice: Option<String>,
    pending_delete: Option<String>,
    settings: AppSettings,
    settings_notice: Option<String>,
    capturing_shortcut: bool,
    icon_image_cache: HashMap<(IconGlyph, String, u32), Arc<RenderImage>>,
    thumbs: HashMap<String, Arc<RenderImage>>,
    history_search: String,
    history_search_active: bool,
    search_focus: gpui::FocusHandle,
    icon_image_cache_bytes: usize,
    selected: Vec<ShapeId>,
    zoom: f32,
    pan: Point<Pixels>,
    drag: Drag,
    space_held: bool,
    /// The text shape being edited and its caret (char index).
    text_editing: Option<ShapeId>,
    caret: usize,
    /// Crop rect in image px while the crop tool is active.
    crop: Option<Rect>,
    source_revision: u64,
    effect_cache: Option<EffectCache>,
}

struct EffectCache {
    source_revision: u64,
    shapes: Vec<Shape>,
    editing_shape: Option<ShapeId>,
    preview: Option<Arc<RenderImage>>,
}

fn prepare_capture(source: EditorSource) -> Result<PreparedCapture, String> {
    let rgba = image::ImageReader::new(std::io::Cursor::new(&source.png))
        .with_guessed_format()
        .map_err(|error| format!("Could not open the capture image: {error}"))?
        .decode()
        .map_err(|error| format!("Could not decode the capture image: {error}"))?
        .into_rgba8();
    let (width, height) = rgba.dimensions();

    let (mut doc, doc_path, annotation_warning) = match source.annotations {
        library::AnnotationSource::Transient => (None, None, false),
        library::AnnotationSource::Missing { path } => (None, Some(path), false),
        library::AnnotationSource::Loaded { document, path } => (Some(document), Some(path), false),
        library::AnnotationSource::Rejected { error } => {
            eprintln!("Snipsy: {error}");
            (None, None, true)
        }
    };

    let mut doc = match doc.take() {
        Some(doc) => doc,
        None => Document::new(width, height),
    };
    doc.image_id = doc_path
        .as_ref()
        .and_then(|path| path.parent())
        .and_then(|path| path.file_name())
        .map(|id| id.to_string_lossy().into_owned());
    if doc.image_width != width || doc.image_height != height {
        return Err(format!(
            "The annotation document is {}×{} but the image is {width}×{height}.",
            doc.image_width, doc.image_height
        ));
    }

    Ok(PreparedCapture {
        title: source.title,
        image: crate::pin::rgba_to_render_image(rgba.clone()),
        rgba,
        doc,
        doc_path,
        annotation_warning,
    })
}

/// Open an editor window for a capture, reusing the current window when present.
pub fn open(cx: &mut App, source: EditorSource) -> Result<(), String> {
    let prepared = prepare_capture(source)?;
    if !cx.has_global::<EditorSession>() {
        cx.set_global(EditorSession::default());
    }

    if let Some(handle) = cx.global::<EditorSession>().window {
        let mut prepared = Some(prepared);
        match handle.update(cx, |editor, window, cx| {
            editor.replace_capture(
                prepared.take().expect("capture is only loaded once"),
                window,
                cx,
            );
        }) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let Some(prepared) = prepared else {
                    return Err(format!("Could not update the existing editor: {error}"));
                };
                cx.global_mut::<EditorSession>().window = None;
                return open_new(cx, prepared);
            }
        }
    }

    open_new(cx, prepared)
}

fn open_new(cx: &mut App, prepared: PreparedCapture) -> Result<(), String> {
    // Editor: a normal, decorated window the user can move and resize.
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: platform::gpui_virtual_screen_bounds(cx).origin + gpui::point(px(60.), px(60.)),
            size: size(px(960.), px(700.)),
        })),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some(format!("Snipsy — {}", prepared.title).into()),
            ..Default::default()
        }),
        focus: true,
        show: true,
        #[cfg(not(windows))]
        window_decorations: Some(gpui::WindowDecorations::Client),
        ..Default::default()
    };

    let handle = cx
        .open_window(options, |window, cx| {
            let view = cx.new(|cx| EditorWindow::from_prepared(prepared, cx));
            let focus = view.read(cx).focus.clone();
            window.focus(&focus, cx);
            view
        })
        .map_err(|error| format!("Could not open the editor window: {error}"))?;
    cx.global_mut::<EditorSession>().window = Some(handle);
    Ok(())
}

impl EditorWindow {
    fn from_prepared(prepared: PreparedCapture, cx: &mut Context<Self>) -> Self {
        let settings = current_settings();
        crate::theme::set_scheme(cx, settings.theme);
        let (captures, library_error) = match library::list_captures() {
            Ok(captures) => (captures, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        Self {
            focus: cx.focus_handle(),
            title: prepared.title,
            image: prepared.image,
            rgba: prepared.rgba,
            doc: prepared.doc,
            doc_path: prepared.doc_path,
            annotation_warning: prepared.annotation_warning,
            history: History::new(),
            tool: Tool::Select,
            color_index: 4,
            stroke_width_index: 1,
            icon_glyph: IconGlyph::Star,
            panel: None,
            captures,
            library_error,
            library_notice: None,
            pending_delete: None,
            settings,
            settings_notice: None,
            capturing_shortcut: false,
            icon_image_cache: HashMap::new(),
            icon_image_cache_bytes: 0,
            thumbs: HashMap::new(),
            history_search: String::new(),
            history_search_active: false,
            search_focus: cx.focus_handle(),
            selected: Vec::new(),
            zoom: 0.0,
            pan: Point::default(),
            drag: Drag::None,
            space_held: false,
            text_editing: None,
            caret: 0,
            crop: None,
            source_revision: 0,
            effect_cache: None,
        }
    }

    fn replace_capture(
        &mut self,
        prepared: PreparedCapture,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_text_edit();
        self.title = prepared.title;
        window.set_window_title(&format!("Snipsy — {}", self.title));
        self.image = prepared.image;
        self.rgba = prepared.rgba;
        self.doc = prepared.doc;
        self.doc_path = prepared.doc_path;
        self.annotation_warning = prepared.annotation_warning;
        self.library_notice = None;
        self.pending_delete = None;
        self.history = History::new();
        self.tool = Tool::Select;
        self.color_index = 4;
        self.stroke_width_index = 1;
        self.selected.clear();
        self.zoom = 0.0;
        self.pan = Point::default();
        self.drag = Drag::None;
        self.space_held = false;
        self.text_editing = None;
        self.caret = 0;
        self.crop = None;
        self.source_revision = self.source_revision.wrapping_add(1);
        self.effect_cache = None;
        self.refresh_library(cx);
        window.activate_window();
        window.focus(&self.focus, cx);
        cx.notify();
    }
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
        KeyBinding::new("b", ToolBlur, Some("EditorWindow")),
        KeyBinding::new("m", ToolPixelate, Some("EditorWindow")),
        KeyBinding::new("shift-r", ToolRedact, Some("EditorWindow")),
        KeyBinding::new("i", ToolIcon, Some("EditorWindow")),
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
        KeyBinding::new("shift-0", ZoomFit, Some("EditorWindow")),
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
        if self.tool == Tool::Arrow {
            ARROW_STROKE_WIDTHS[self.stroke_width_index]
        } else {
            STROKE_WIDTHS[self.stroke_width_index]
        }
    }

    fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.first()?;
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
            self.selected.retain(|selected| selected != &id);
        }
        self.commit();
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
            Tool::Rectangle
            | Tool::Ellipse
            | Tool::Arrow
            | Tool::Blur
            | Tool::Pixelate
            | Tool::Redact => {
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
            Tool::Icon => self.place_icon(ix, iy),
            Tool::Crop => self.crop_mouse_down(ix, iy),
        }
    }

    fn select_mouse_down(&mut self, ix: f32, iy: f32) {
        if self.selected.len() == 1
            && let Some(index) = self.selected_index()
        {
            match &self.doc.shapes[index] {
                Shape::Arrow(arrow) => {
                    if let Some(endpoint) =
                        arrow_endpoint_at(arrow, ix, iy, HANDLE_SCREEN_SIZE / self.zoom)
                    {
                        self.history.begin(&self.doc, None);
                        self.drag = Drag::ArrowEndpoint {
                            id: arrow.id.clone(),
                            original: arrow.clone(),
                            endpoint,
                        };
                        return;
                    }
                }
                shape => {
                    if let Some(handle) =
                        handle_at(&shape.bounds(), ix, iy, HANDLE_SCREEN_SIZE / self.zoom)
                    {
                        self.history.begin(&self.doc, None);
                        self.drag = Drag::Resize {
                            id: shape.id().clone(),
                            original: shape.clone(),
                            handle,
                        };
                        return;
                    }
                }
            }
        }
        match hit_test(&self.doc, ix, iy, 4.0 / self.zoom) {
            Some(index) => {
                let id = self.doc.shapes[index].id().clone();
                if self.selected.len() > 1 && self.selected.contains(&id) {
                    let originals = self
                        .selected
                        .iter()
                        .filter_map(|selected| {
                            self.doc
                                .index_of(selected)
                                .map(|index| self.doc.shapes[index].clone())
                        })
                        .collect();
                    self.history.begin(&self.doc, None);
                    self.drag = Drag::MoveMany {
                        originals,
                        start: (ix, iy),
                    };
                } else {
                    self.selected = vec![id.clone()];
                    self.history.begin(&self.doc, None);
                    self.drag = Drag::Move {
                        id,
                        original: self.doc.shapes[index].clone(),
                        start: (ix, iy),
                    };
                }
            }
            None => {
                self.drag = Drag::Marquee {
                    start: (ix, iy),
                    current: (ix, iy),
                };
            }
        }
    }

    fn place_text(&mut self, ix: f32, iy: f32) {
        if let Some(index) = hit_test(&self.doc, ix, iy, 4.0 / self.zoom)
            && matches!(&self.doc.shapes[index], Shape::Text(_))
        {
            let id = self.doc.shapes[index].id().clone();
            self.selected = vec![id.clone()];
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
        self.selected = vec![id.clone()];
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
        self.selected = self
            .doc
            .shapes
            .last()
            .map(|shape| vec![shape.id().clone()])
            .unwrap_or_default();
    }

    fn place_icon(&mut self, ix: f32, iy: f32) {
        let size = 48.0;
        self.history.begin(&self.doc, None);
        self.doc.shapes.push(Shape::Icon(IconShape {
            id: next_shape_id("icon"),
            glyph: self.icon_glyph,
            x: ix - size / 2.0,
            y: iy - size / 2.0,
            size,
            color: self.current_color(),
        }));
        self.selected = self
            .doc
            .shapes
            .last()
            .map(|shape| vec![shape.id().clone()])
            .unwrap_or_default();
        self.commit();
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
            Drag::MoveMany { originals, start } => {
                let (ix, iy) = self.to_image(position);
                for original in originals {
                    if let Some(shape) = self
                        .doc
                        .shapes
                        .iter_mut()
                        .find(|shape| shape.id() == original.id())
                    {
                        *shape = original.clone();
                        shape.translate(ix - start.0, iy - start.1);
                    }
                }
            }
            Drag::ArrowEndpoint {
                id,
                original,
                endpoint,
            } => {
                let (ix, iy) = self.to_image(position);
                if let Some(Shape::Arrow(arrow)) =
                    self.doc.shapes.iter_mut().find(|shape| shape.id() == &id)
                {
                    *arrow = original;
                    match endpoint {
                        ArrowEndpoint::Start => (arrow.x1, arrow.y1) = (ix, iy),
                        ArrowEndpoint::End => (arrow.x2, arrow.y2) = (ix, iy),
                    }
                }
            }
            Drag::Marquee { start, .. } => {
                self.drag = Drag::Marquee {
                    start,
                    current: self.to_image(position),
                };
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
                    self.selected = self
                        .doc
                        .shapes
                        .last()
                        .map(|shape| vec![shape.id().clone()])
                        .unwrap_or_default();
                    self.commit();
                } else {
                    let _ = self.history.cancel();
                }
            }
            Drag::Move { .. }
            | Drag::MoveMany { .. }
            | Drag::ArrowEndpoint { .. }
            | Drag::Resize { .. } => self.commit(),
            Drag::Marquee { start, .. } => {
                let current = self.to_image(event.position);
                let selection = Rect::new(
                    start.0.min(current.0),
                    start.1.min(current.1),
                    (start.0 - current.0).abs(),
                    (start.1 - current.1).abs(),
                );
                self.selected = shapes_intersecting(&self.doc, &selection);
            }
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
            Tool::Blur | Tool::Pixelate | Tool::Redact => {
                (start.0 - current.0).abs() >= 2.0 && (start.1 - current.1).abs() >= 2.0
            }
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
            Tool::Blur | Tool::Pixelate | Tool::Redact => Shape::Effect(EffectShape {
                id,
                bounds: Rect::new(
                    start.0.min(current.0),
                    start.1.min(current.1),
                    (start.0 - current.0).abs(),
                    (start.1 - current.1).abs(),
                ),
                kind: match self.tool {
                    Tool::Blur => EffectKind::Blur { radius_px: 14.0 },
                    Tool::Pixelate => EffectKind::Pixelate { block_px: 10 },
                    Tool::Redact => EffectKind::Redact,
                    _ => unreachable!(),
                },
            }),
            _ => unreachable!("finish_draw only runs for drawing tools"),
        };
        self.doc.shapes.push(shape);
        self.selected = self
            .doc
            .shapes
            .last()
            .map(|shape| vec![shape.id().clone()])
            .unwrap_or_default();
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
            Tool::Blur => "blur",
            Tool::Pixelate => "pixelate",
            Tool::Redact => "redact",
            Tool::Icon => "icon",
        }
    }

    // ---- interaction: keyboard ------------------------------------------

    fn on_key_down_raw(&mut self, event: &gpui::KeyDownEvent) {
        if self.capturing_shortcut {
            if event.keystroke.key == "escape" {
                self.capturing_shortcut = false;
                self.settings_notice = Some("Shortcut recording cancelled.".to_string());
            } else if let Some(shortcut) = captured_shortcut(event) {
                self.settings.capture_hotkey = shortcut;
                self.capturing_shortcut = false;
                self.persist_settings();
            } else {
                self.settings_notice = Some(
                    "Hold Ctrl, Alt or Shift and press a letter, number or function key."
                        .to_string(),
                );
            }
            return;
        }
        if self.history_search_active {
            self.handle_search_keystroke(event);
            return;
        }
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

    /// Keys while the history search is focused: printable characters and
    /// backspace filter the rail, Esc exits.
    fn handle_search_keystroke(&mut self, event: &gpui::KeyDownEvent) {
        match event.keystroke.key.as_str() {
            "escape" => {
                self.history_search.clear();
                self.history_search_active = false;
            }
            "backspace" => {
                self.history_search.pop();
            }
            "enter" => {
                self.history_search_active = false;
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
                    self.history_search.push(character);
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
            self.selected.clear();
        }
    }

    fn show_panel(&mut self, panel: EditorPanel) {
        self.capturing_shortcut = false;
        let next = (self.panel != Some(panel)).then_some(panel);
        if next != Some(EditorPanel::Library) {
            self.pending_delete = None;
        }
        self.panel = next;
    }

    fn persist_settings(&mut self) {
        self.settings_notice = Some(match crate::settings::write_settings(&self.settings) {
            Ok(()) => "Saved. Cursor applies next capture; restart to apply the hotkey. Startup registration is not wired in this GPUI shell yet.".to_string(),
            Err(error) => format!("Could not save preferences: {error}"),
        });
    }

    fn refresh_library(&mut self, cx: &mut Context<Self>) {
        match library::list_captures() {
            Ok(captures) => {
                self.captures = captures;
                self.library_error = None;
                self.load_thumbnails(cx);
            }
            Err(error) => self.library_error = Some(error),
        }
    }

    /// Decode 128px-wide thumbnails for the newest captures off the main
    /// thread; rows appear as each thumbnail lands.
    fn load_thumbnails(&mut self, cx: &mut Context<Self>) {
        let pending: Vec<String> = self
            .captures
            .iter()
            .filter(|capture| !self.thumbs.contains_key(&capture.id))
            .take(50)
            .map(|capture| capture.id.clone())
            .collect();
        if pending.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            for id in pending {
                let task_id = id.clone();
                let thumb = cx
                    .background_executor()
                    .spawn(async move { decode_thumbnail(&task_id) })
                    .await;
                this.update(cx, |editor, cx| {
                    if let Some(thumb) = thumb {
                        editor.thumbs.insert(id.clone(), thumb);
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn open_library_capture(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(capture) = self.captures.iter().find(|capture| capture.id == id) else {
            self.library_notice = Some("That capture is no longer in the library.".to_string());
            self.refresh_library(cx);
            return;
        };
        let source = EditorSource {
            png: match library::read_image_png(id) {
                Ok(png) => png,
                Err(error) => {
                    self.library_notice = Some(error);
                    self.refresh_library(cx);
                    return;
                }
            },
            title: capture.title.clone(),
            annotations: library::read_annotations(id),
        };
        match prepare_capture(source) {
            Ok(prepared) => self.replace_capture(prepared, window, cx),
            Err(error) => self.library_notice = Some(error),
        }
    }

    fn delete_library_capture(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.doc.image_id.as_deref() == Some(id) {
            self.library_notice = Some("Close this capture before deleting it.".to_string());
            self.pending_delete = None;
            return;
        }
        self.library_notice = match library::delete_capture(id) {
            Ok(()) => Some("Capture deleted.".to_string()),
            Err(error) => Some(error),
        };
        self.pending_delete = None;
        self.refresh_library(cx);
    }

    fn delete_selected(&mut self) {
        if self.text_editing.is_some() {
            return;
        }
        if !self.selected.is_empty() {
            let selected = std::mem::take(&mut self.selected);
            self.history.begin(&self.doc, None);
            for id in selected {
                self.doc.delete(&id);
            }
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
            if matches!(&self.doc.shapes[index], Shape::Effect(_)) {
                return;
            }
            self.history.begin(&self.doc, None);
            let shape_color = match &mut self.doc.shapes[index] {
                Shape::Rectangle(shape) => &mut shape.color,
                Shape::Ellipse(shape) => &mut shape.color,
                Shape::Arrow(shape) => &mut shape.color,
                Shape::Pen(shape) => &mut shape.color,
                Shape::Text(shape) => &mut shape.color,
                Shape::Counter(shape) => &mut shape.color,
                Shape::Icon(shape) => &mut shape.color,
                Shape::Effect(_) => unreachable!(),
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
        if let Some(index) = self.selected_index() {
            let width = if matches!(&self.doc.shapes[index], Shape::Arrow(_)) {
                ARROW_STROKE_WIDTHS[width_index]
            } else {
                STROKE_WIDTHS[width_index]
            };
            self.history.begin(&self.doc, None);
            match &mut self.doc.shapes[index] {
                Shape::Rectangle(shape) => shape.stroke_width = width,
                Shape::Ellipse(shape) => shape.stroke_width = width,
                Shape::Arrow(shape) => shape.stroke_width = width,
                Shape::Pen(shape) => shape.stroke_width = width,
                Shape::Text(_) | Shape::Counter(_) | Shape::Icon(_) | Shape::Effect(_) => {}
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
                self.source_revision = self.source_revision.wrapping_add(1);
                self.effect_cache = None;
                if let Err(error) = self.replace_library_image() {
                    eprintln!("Snipsy: {error}");
                }
            }
            self.selected
                .retain(|selected| self.doc.index_of(selected).is_some());
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
                self.source_revision = self.source_revision.wrapping_add(1);
                self.effect_cache = None;
                if let Err(error) = self.replace_library_image() {
                    eprintln!("Snipsy: {error}");
                }
            }
            self.selected
                .retain(|selected| self.doc.index_of(selected).is_some());
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
        self.source_revision = self.source_revision.wrapping_add(1);
        self.effect_cache = None;
        self.commit_with_image();

        // The library's stored capture is now stale — replace it so closing
        // and reopening the editor shows the same image.
        if let Err(error) = self.replace_library_image() {
            eprintln!("Snipsy: {error}");
        }

        self.tool = Tool::Select;
        self.selected.clear();
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
        self.selected
            .retain(|selected| self.doc.index_of(selected).is_some());
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

fn captured_shortcut(event: &gpui::KeyDownEvent) -> Option<String> {
    let modifiers = event.keystroke.modifiers;
    shortcut_from_keys(
        &event.keystroke.key,
        modifiers.control,
        modifiers.platform,
        modifiers.alt,
        modifiers.shift,
    )
}

fn shortcut_from_keys(
    key: &str,
    control: bool,
    platform: bool,
    alt: bool,
    shift: bool,
) -> Option<String> {
    let key = key.to_ascii_lowercase();
    let valid_key = (key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric())
        || key
            .strip_prefix('f')
            .and_then(|number| number.parse::<u8>().ok())
            .is_some_and(|number| (1..=24).contains(&number));
    if !valid_key || !(control || platform || alt || shift) {
        return None;
    }
    let mut parts = Vec::new();
    if control || platform {
        parts.push("ctrl");
    }
    if alt {
        parts.push("alt");
    }
    if shift {
        parts.push("shift");
    }
    parts.push(&key);
    Some(parts.join("+"))
}

impl Render for EditorWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        if self.zoom <= 0.0 {
            self.fit(viewport);
        }

        let theme = Theme::current(cx);
        let canvas_state = self.canvas_state(theme.bg_canvas);
        let focus = self.focus.clone();
        let has_selection = !self.selected.is_empty() && self.text_editing.is_none();
        let panel_open = self.panel.is_some();

        let mut root = div()
            .id("editor-root")
            .relative()
            .size_full()
            .bg(theme.bg_window)
            .key_context(if self.capturing_shortcut {
                "ShortcutCapture"
            } else if self.text_editing.is_some() {
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
            .on_action(cx.listener(|this, _: &ToolBlur, _, cx| {
                this.set_tool(Tool::Blur);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolPixelate, _, cx| {
                this.set_tool(Tool::Pixelate);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolRedact, _, cx| {
                this.set_tool(Tool::Redact);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ToolIcon, _, cx| {
                this.set_tool(Tool::Icon);
                this.show_panel(EditorPanel::Icons);
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
                    if in_chrome(event.position, viewport, has_selection, panel_open) {
                        return;
                    }
                    this.on_canvas_mouse_down(event);
                }),
            )
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, event: &MouseDownEvent, _, _| {
                    if !in_chrome(event.position, viewport, has_selection, panel_open) {
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
            .children(self.crop_status(cx))
            .children(self.redact_warning(cx))
            .children(self.annotation_warning(cx))
            .child(self.toolbar(cx))
            .children(self.selection_bar(cx))
            .children(self.side_panel(cx))
            .child(self.zoom_indicator(viewport, cx))
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

fn in_chrome(
    position: Point<Pixels>,
    viewport: Size<Pixels>,
    has_selection: bool,
    panel_open: bool,
) -> bool {
    let bar_height = TOOLBAR_BOTTOM
        + TOOLBAR_HEIGHT
        + if has_selection {
            12.0 + SELECTION_BAR_HEIGHT
        } else {
            0.0
        };
    position.y >= viewport.height - px(bar_height)
        || (panel_open && position.x >= viewport.width - px(SIDE_PANEL_WIDTH))
}

impl EditorWindow {
    fn drag_needs_move(&self) -> bool {
        !matches!(self.drag, Drag::None)
    }

    /// Everything the canvas paint closure needs, resolved at render time.
    fn canvas_state(&mut self, backdrop: gpui::Hsla) -> CanvasState {
        let flattened_preview = self.flattened_preview();
        let shapes = self.doc.shapes.clone();
        let icons = shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Icon(icon) => Some(icon.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut icon_images = HashMap::new();
        for icon in icons {
            if let Some(image) = self.cached_icon_image(&icon) {
                icon_images.insert(icon.id, image);
            }
        }
        CanvasState {
            backdrop,
            image: self.image.clone(),
            image_width: self.doc.image_width,
            image_height: self.doc.image_height,
            zoom: self.zoom,
            pan: self.pan,
            shapes,
            icon_images,
            flattened_preview,
            preview: self.preview_shape(),
            selected: self.selected.clone(),
            marquee: match &self.drag {
                Drag::Marquee { start, current } => Some(Rect::new(
                    start.0.min(current.0),
                    start.1.min(current.1),
                    (start.0 - current.0).abs(),
                    (start.1 - current.1).abs(),
                )),
                _ => None,
            },
            space_or_pan: self.space_held,
            crop: if self.tool == Tool::Crop {
                self.crop
            } else {
                None
            },
        }
    }

    fn cached_icon_image(&mut self, icon: &IconShape) -> Option<Arc<RenderImage>> {
        self.cached_icon_asset(icon.glyph, &icon.color, icon.size)
    }

    fn cached_icon_asset(
        &mut self,
        glyph: IconGlyph,
        color: &str,
        size: f32,
    ) -> Option<Arc<RenderImage>> {
        let key = (glyph, color.to_string(), size.round() as u32);
        if let Some(image) = self.icon_image_cache.get(&key) {
            return Some(image.clone());
        }
        let raster = super::icons::rasterize(glyph, color, size).ok()?;
        let image_bytes = raster.as_raw().len();
        if self.icon_image_cache_bytes + image_bytes > ICON_CACHE_MAX_BYTES {
            self.icon_image_cache.clear();
            self.icon_image_cache_bytes = 0;
        }
        let image = crate::pin::rgba_to_render_image(raster);
        self.icon_image_cache_bytes += image_bytes;
        self.icon_image_cache.insert(key, image.clone());
        Some(image)
    }

    fn flattened_preview(&mut self) -> Option<Arc<RenderImage>> {
        if !self
            .doc
            .shapes
            .iter()
            .any(|shape| matches!(shape, Shape::Effect(_)))
        {
            self.effect_cache = None;
            return None;
        }

        let editing_shape = self.text_editing.clone();
        let cache_is_current = self.effect_cache.as_ref().is_some_and(|cache| {
            cache.source_revision == self.source_revision
                && cache.shapes == self.doc.shapes
                && cache.editing_shape == editing_shape
        });
        if !cache_is_current {
            let preview = match super::flatten::flatten_with_effect_patches_skipping(
                &self.doc,
                self.rgba.as_raw(),
                &super::flatten::SystemTextPainter::new(),
                editing_shape.as_deref(),
            ) {
                Ok((pixmap, _)) => Some(crate::pin::rgba_to_render_image(
                    super::flatten::pixmap_to_rgba(&pixmap),
                )),
                Err(error) => {
                    eprintln!("Snipsy: could not render effect preview: {error}");
                    None
                }
            };
            self.effect_cache = Some(EffectCache {
                source_revision: self.source_revision,
                shapes: self.doc.shapes.clone(),
                editing_shape,
                preview,
            });
        }
        self.effect_cache
            .as_ref()
            .and_then(|cache| cache.preview.clone())
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
                    Tool::Blur | Tool::Pixelate | Tool::Redact => Shape::Effect(EffectShape {
                        id: "preview".to_string(),
                        bounds: Rect::new(
                            start.0.min(current.0),
                            start.1.min(current.1),
                            (start.0 - current.0).abs(),
                            (start.1 - current.1).abs(),
                        ),
                        kind: match self.tool {
                            Tool::Blur => EffectKind::Blur { radius_px: 14.0 },
                            Tool::Pixelate => EffectKind::Pixelate { block_px: 10 },
                            Tool::Redact => EffectKind::Redact,
                            _ => unreachable!(),
                        },
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
        let has_effect = self
            .doc
            .shapes
            .iter()
            .any(|shape| matches!(shape, Shape::Effect(_)));
        for shape in &self.doc.shapes {
            match shape {
                Shape::Text(text) => {
                    let left = self.to_screen_x(text.x);
                    let top = self.to_screen_y(text.y);
                    let font_px = (text.size * self.zoom).max(6.0);
                    let is_editing = self.text_editing.as_ref() == Some(&text.id);
                    if has_effect && !is_editing {
                        continue;
                    }
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
                    if has_effect {
                        continue;
                    }
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
    fn crop_status(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        if self.tool != Tool::Crop {
            return None;
        }
        let theme = Theme::current(cx);
        Some(
            div()
                .absolute()
                .top(px(8.0))
                .left(px(12.0))
                .px_2()
                .py_1()
                .rounded(px(R_INPUT))
                .bg(theme.bg_toolbar)
                .border_1()
                .border_color(theme.hairline)
                .text_size(px(11.0))
                .text_color(theme.text_1)
                .font_family(default_font_family())
                .child("Drag a region · Ctrl+Enter to crop · Esc to cancel")
                .into_any_element(),
        )
    }

    fn redact_warning(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        (self.tool == Tool::Redact
            || self.doc.shapes.iter().any(
                |shape| {
                    matches!(shape, Shape::Effect(effect) if matches!(&effect.kind, EffectKind::Redact))
                },
            ))
        .then(|| {
            let theme = Theme::current(cx);
            div()
                .absolute()
                .top(px(8.0))
                .left(px(12.0))
                .px_2()
                .py_1()
                .rounded(px(R_INPUT))
                .bg(theme.bg_toolbar)
                .border_1()
                .border_color(theme.danger)
                .text_size(px(11.0))
                .text_color(theme.text_1)
                .font_family(default_font_family())
                .child("Redaction affects flattened exports only; the original capture remains unchanged.")
                .into_any_element()
        })
    }

    fn annotation_warning(&self, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        self.annotation_warning.then(|| {
            let theme = Theme::current(cx);
            div()
                .absolute()
                .top(px(8.0))
                .right(px(12.0))
                .w(px(360.0))
                .px_3()
                .py_2()
                .rounded(px(R_PANEL))
                .border_1()
                .border_color(theme.danger)
                .bg(theme.bg_toolbar)
                .text_size(px(11.0))
                .text_color(theme.text_1)
                .font_family(default_font_family())
                .child("Annotations could not be loaded. Changes won't overwrite the file; use Save PNG to keep a copy.")
                .into_any_element()
        })
    }

    fn panel_shell(
        &self,
        panel: EditorPanel,
        title: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let theme = Theme::current(cx);
        div()
            .absolute()
            .top(px(8.0))
            .right(px(8.0))
            .bottom(px(TOOLBAR_BOTTOM + TOOLBAR_HEIGHT + 8.0))
            .w(px(SIDE_PANEL_WIDTH))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(px(R_PANEL))
            .bg(theme.bg_surface)
            .border_1()
            .border_color(theme.hairline)
            .shadow(theme.shadow_panel.clone())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_family(default_font_family())
                            .text_size(px(15.0))
                            .text_color(theme.text_1)
                            .child(title),
                    )
                    .child(self.icon_button_plain(
                        format!("history-panel-close-{panel:?}"),
                        "close",
                        "Close panel",
                        &theme,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            this.show_panel(panel);
                            cx.notify();
                        }),
                    )),
            )
    }

    /// A bare 28x28 icon button used inside panels.
    fn icon_button_plain(
        &self,
        element_id: String,
        slug: &'static str,
        label: &'static str,
        theme: &Theme,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let debug_selector = element_id.clone();
        let button = div()
            .id(gpui::ElementId::Name(element_id.into()))
            .debug_selector(move || debug_selector.clone())
            .w(px(28.0))
            .h(px(28.0))
            .rounded(px(R_BTN))
            .flex()
            .items_center()
            .justify_center()
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(theme.bg_elevated))
            .on_mouse_down(MouseButton::Left, handler)
            .child(crate::ui::chrome_icon(slug, 16.0, theme.text_2));
        tooltip(button, label)
    }

    fn side_panel(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        match self.panel? {
            EditorPanel::Icons => Some(self.icons_panel(cx).into_any_element()),
            EditorPanel::Library => Some(self.library_panel(cx).into_any_element()),
            EditorPanel::Preferences => Some(self.preferences_panel(cx).into_any_element()),
        }
    }

    fn icons_panel(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let mut panel = self.panel_shell(EditorPanel::Icons, "Icons", cx).child(
            div()
                .text_size(px(11.0))
                .text_color(theme.text_2)
                .font_family(default_font_family())
                .child("Pick a symbol, then click the canvas."),
        );
        let color = self.current_color();
        for glyphs in IconGlyph::ALL.chunks(2) {
            let mut row = div().flex().flex_row().gap_1();
            for &glyph in glyphs {
                let preview = self.cached_icon_asset(glyph, &color, 24.0);
                let mut button = div()
                    .id(gpui::ElementId::Name(
                        format!("glyph-{}", glyph.label()).into(),
                    ))
                    .flex_1()
                    .h(px(34.0))
                    .px_2()
                    .rounded(px(R_BTN))
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(11.0))
                    .text_color(theme.text_1)
                    .font_family(default_font_family())
                    .cursor(gpui::CursorStyle::PointingHand)
                    .hover(move |style| style.bg(theme.bg_elevated))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            this.icon_glyph = glyph;
                            this.set_tool(Tool::Icon);
                            cx.notify();
                        }),
                    );
                if let Some(preview) = preview {
                    button = button.child(icon_preview(preview));
                }
                button = button.child(glyph.label());
                if self.icon_glyph == glyph {
                    button = button.bg(theme.accent_soft).text_color(theme.accent);
                } else {
                    button = button.bg(theme.bg_elevated);
                }
                row = row.child(button);
            }
            panel = panel.child(row);
        }
        panel
    }

    /// The history rail (design-spec §5.3): search, day-grouped capture
    /// rows with thumbnails, and inline actions revealed on hover.
    fn library_panel(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let mut panel = self
            .panel_shell(EditorPanel::Library, "History", cx)
            .child(self.search_field(cx));

        if let Some(error) = &self.library_error {
            panel = panel.child(panel_message(error, &theme));
        }
        if let Some(notice) = &self.library_notice {
            panel = panel.child(panel_message(notice, &theme));
        }

        let query = self.history_search.to_lowercase();
        let matches = |capture: &library::CaptureMetadata| {
            query.is_empty() || capture.title.to_lowercase().contains(&query)
        };
        let visible: Vec<&library::CaptureMetadata> =
            self.captures.iter().filter(|c| matches(c)).collect();
        if visible.is_empty() && self.library_error.is_none() {
            panel = panel.child(panel_message(
                if query.is_empty() {
                    "No captures yet."
                } else {
                    "No captures match."
                },
                &theme,
            ));
        }

        let mut entries = div()
            .id("history-entries")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_y_scroll();
        let mut last_group = None::<String>;
        for capture in visible {
            let group = day_label(capture.updated_at);
            if last_group.as_deref() != Some(group.as_str()) {
                entries = entries.child(
                    div()
                        .text_size(px(9.0))
                        .text_color(theme.text_3)
                        .font_family(default_font_family())
                        .pt_1()
                        .child(group.to_uppercase()),
                );
                last_group = Some(group);
            }
            entries = entries.child(self.history_row(capture, &theme, cx));
        }
        panel.child(entries)
    }

    /// A 26px search input: click to focus, type to filter, Esc to clear.
    fn search_field(&self, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        let active = self.history_search_active;
        let mut field = div()
            .id("history-search")
            .h(px(26.0))
            .w_full()
            .px_2()
            .rounded(px(R_INPUT))
            .bg(theme.bg_elevated)
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(crate::ui::chrome_icon("search", 14.0, theme.text_3))
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(if self.history_search.is_empty() {
                        theme.text_3
                    } else {
                        theme.text_1
                    })
                    .font_family(default_font_family())
                    .flex_1()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(if self.history_search.is_empty() {
                        "Search history".to_string()
                    } else {
                        self.history_search.clone()
                    }),
            );
        if !self.history_search.is_empty() {
            field = field.child(self.icon_button_plain(
                "history-search-clear".to_string(),
                "close",
                "Clear search",
                &theme,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.history_search.clear();
                    this.history_search_active = false;
                    cx.notify();
                }),
            ));
        }
        if active {
            field = field.border_1().border_color(theme.accent);
        }
        field.on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _: &MouseDownEvent, window, cx| {
                this.history_search_active = true;
                window.focus(&this.search_focus, cx);
                cx.notify();
            }),
        )
    }

    fn history_row(
        &self,
        capture: &library::CaptureMetadata,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let id = capture.id.clone();
        let row_id = id.clone();
        let current = self.doc.image_id.as_deref() == Some(id.as_str());
        let pending_delete = self.pending_delete.as_deref() == Some(id.as_str());
        let dims = format!("{} × {}", capture.image.width, capture.image.height);
        let time = time_label(capture.updated_at);

        let mut row = div()
            .id(gpui::ElementId::Name(format!("history-{}", id).into()))
            .group("history-row")
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .p_1p5()
            .rounded(px(R_THUMB + 2.0))
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(theme.bg_elevated))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    this.open_library_capture(&row_id, window, cx);
                    cx.notify();
                }),
            );
        if current {
            row = row.bg(theme.accent_soft);
        }

        // 64x40 thumbnail (contained) or a flat placeholder.
        let thumb_area = div()
            .w(px(64.0))
            .h(px(40.0))
            .rounded(px(R_THUMB))
            .bg(theme.bg_elevated)
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .child(match self.thumbs.get(&capture.id) {
                Some(thumb) => {
                    thumbnail_canvas(thumb.clone(), capture.image.width, capture.image.height)
                }
                None => div().into_any_element(),
            });

        let mut meta = div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.text_1)
                    .font_family(default_font_family())
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(capture.title.clone()),
            )
            .child(
                div()
                    .text_size(px(9.0))
                    .text_color(theme.text_3)
                    .font_family(default_font_family())
                    .child(format!("{dims} · {time}")),
            );

        // Inline actions, revealed by hover over the row.
        let mut actions = div()
            .flex()
            .flex_row()
            .gap_1()
            .opacity(0.0)
            .group_hover("history-row", |style| style.opacity(1.0));
        if current {
            actions = actions.child(
                div()
                    .text_size(px(9.0))
                    .text_color(theme.accent)
                    .font_family(default_font_family())
                    .child("OPEN"),
            );
        } else {
            let open_id = id.clone();
            actions = actions.child(self.icon_button_plain(
                format!("history-action-open-{id}"),
                "pen",
                "Open in editor",
                theme,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    this.open_library_capture(&open_id, window, cx);
                    cx.stop_propagation();
                    cx.notify();
                }),
            ));
        }
        if pending_delete {
            let confirm_id = id.clone();
            actions = actions
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(theme.danger)
                        .font_family(default_font_family())
                        .child("Delete?"),
                )
                .child(self.icon_button_plain(
                    format!("history-action-confirm-delete-{id}"),
                    "delete",
                    "Confirm delete",
                    theme,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        this.delete_library_capture(&confirm_id, cx);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                ));
        } else if !current {
            let delete_id = id.clone();
            actions = actions.child(self.icon_button_plain(
                format!("history-action-delete-{id}"),
                "delete",
                "Delete",
                theme,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.pending_delete = Some(delete_id.clone());
                    this.library_notice = None;
                    cx.stop_propagation();
                    cx.notify();
                }),
            ));
        }
        meta = meta.child(div().flex().flex_row().gap_1().child(actions));

        row.child(thumb_area).child(meta)
    }

    fn preferences_panel(&self, cx: &mut Context<Self>) -> gpui::Div {
        let shortcut_label = if self.capturing_shortcut {
            "Press shortcut · Esc cancels".to_string()
        } else {
            format!("Record · {}", self.settings.capture_hotkey)
        };
        let theme = Theme::current(cx);
        let mut panel = self
            .panel_shell(EditorPanel::Preferences, "Preferences", cx)
            .child(panel_label("Capture shortcut", &theme))
            .child(
                div()
                    .px_2()
                    .py_2()
                    .rounded(px(R_INPUT))
                    .bg(if self.capturing_shortcut {
                        theme.accent
                    } else {
                        theme.bg_elevated
                    })
                    .text_size(px(12.0))
                    .text_color(if self.capturing_shortcut {
                        gpui::white()
                    } else {
                        theme.text_1
                    })
                    .cursor(gpui::CursorStyle::PointingHand)
                    .child(shortcut_label)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            this.capturing_shortcut = true;
                            this.settings_notice = Some("Press the new shortcut now.".to_string());
                            cx.notify();
                        }),
                    ),
            )
            .child(toggle_preference(
                "Include cursor in captures",
                self.settings.include_cursor,
                &theme,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.settings.include_cursor = !this.settings.include_cursor;
                    this.persist_settings();
                    cx.notify();
                }),
            ))
            .child(toggle_preference(
                "Launch at Windows startup",
                self.settings.launch_at_startup,
                &theme,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.settings.launch_at_startup = !this.settings.launch_at_startup;
                    this.persist_settings();
                    cx.notify();
                }),
            ))
            .child(panel_label("Theme", &theme))
            .child(theme_switch(
                self.settings.theme,
                &theme,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.toggle_theme(cx);
                }),
            ));
        if let Some(notice) = &self.settings_notice {
            panel = panel.child(panel_message(notice, &theme));
        }
        panel.child(
            div()
                .text_size(px(10.0))
                .text_color(theme.text_3)
                .font_family(default_font_family())
                .child(
                    "Cursor applies to the next capture. Restart to apply the hotkey. Startup is saved but not registered by the GPUI shell yet.",
                ),
        )
    }

    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.settings.theme = self.settings.theme.toggled();
        crate::theme::set_scheme(cx, self.settings.theme);
        self.persist_settings();
        cx.notify();
    }

    fn zoom_indicator(&self, viewport: Size<Pixels>, cx: &mut Context<Self>) -> gpui::AnyElement {
        let percent = (self.zoom * 100.0).round() as i32;
        let right = if self.panel.is_some() {
            SIDE_PANEL_WIDTH + 12.0
        } else {
            12.0
        };
        let theme = Theme::current(cx);
        let button_hover = theme.bg_elevated;
        let button_text = theme.text_1;
        let zoom_out = div()
            .id("zoom-out")
            .debug_selector(|| "zoom-out".to_string())
            .w(px(24.0))
            .h(px(24.0))
            .rounded(px(R_BTN))
            .flex()
            .items_center()
            .justify_center()
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(button_hover))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.zoom_at(1.0 / 1.2, size_to_point(viewport));
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(crate::ui::chrome_icon("zoom_out", 16.0, button_text));
        let zoom_in = div()
            .id("zoom-in")
            .debug_selector(|| "zoom-in".to_string())
            .w(px(24.0))
            .h(px(24.0))
            .rounded(px(R_BTN))
            .flex()
            .items_center()
            .justify_center()
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(button_hover))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.zoom_at(1.2, size_to_point(viewport));
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(crate::ui::chrome_icon("zoom_in", 16.0, button_text));
        let fit = div()
            .id("zoom-fit")
            .debug_selector(|| "zoom-fit".to_string())
            .px_1()
            .text_size(px(10.0))
            .text_color(theme.text_2)
            .font_family(default_font_family())
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.text_color(button_text))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.fit(viewport);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child("Fit · ⇧0");
        div()
            .absolute()
            .right(px(right))
            .top(px(12.0))
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_1()
            .py_1()
            .rounded(px(R_INPUT))
            .bg(theme.bg_toolbar)
            .border_1()
            .border_color(theme.hairline)
            .text_size(px(11.0))
            .text_color(theme.text_2)
            .font_family(default_font_family())
            .child(crate::ui::with_tooltip(zoom_out, "Zoom out (-)"))
            .child(format!("{percent}%"))
            .child(crate::ui::with_tooltip(zoom_in, "Zoom in (+)"))
            .child(fit)
            .into_any_element()
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = Theme::current(cx);
        let tools = [
            (Tool::Select, "1", "Select"),
            (Tool::Rectangle, "2", "Rectangle"),
            (Tool::Ellipse, "3", "Ellipse"),
            (Tool::Arrow, "4", "Arrow"),
            (Tool::Pen, "5", "Pen"),
            (Tool::Text, "6", "Text"),
            (Tool::Counter, "7", "Counter"),
            (Tool::Crop, "8", "Crop"),
            (Tool::Blur, "B", "Blur"),
            (Tool::Pixelate, "M", "Pixelate"),
            (Tool::Redact, "Shift+R", "Redact"),
            (Tool::Icon, "I", "Icon"),
        ];

        let mut pill = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .h(px(TOOLBAR_HEIGHT))
            .px(px(8.0))
            .rounded(px(R_PILL))
            .bg(theme.bg_toolbar)
            .border_1()
            .border_color(theme.hairline)
            .shadow(theme.shadow_floating.clone());

        for (tool, shortcut, label) in tools {
            pill = pill.child(self.tool_button(tool, shortcut, label, &theme, cx));
        }

        pill = pill
            .child(separator(&theme))
            .child(self.action_icon_button(
                "undo",
                "Undo",
                !self.history.can_undo(),
                false,
                &theme,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.undo();
                    cx.notify();
                }),
            ))
            .child(self.action_icon_button(
                "redo",
                "Redo",
                !self.history.can_redo(),
                false,
                &theme,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.redo();
                    cx.notify();
                }),
            ))
            .child(separator(&theme))
            .child(self.panel_icon_button("history", "History", EditorPanel::Library, &theme, cx))
            .child(self.panel_icon_button(
                "preferences",
                "Preferences",
                EditorPanel::Preferences,
                &theme,
                cx,
            ))
            .child(separator(&theme))
            .child(
                text_button("Copy", false, &theme)
                    .id("copy-button")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, _| {
                            this.copy_flattened();
                        }),
                    ),
            )
            .child(
                text_button("Save PNG", true, &theme)
                    .id("save-button")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, _| {
                            this.save_flattened();
                        }),
                    ),
            );

        let mut toolbar = div()
            .debug_selector(|| "editor-toolbar".to_string())
            .absolute()
            .left(px(8.0))
            .right(px(8.0))
            .bottom(px(TOOLBAR_BOTTOM))
            .flex()
            .justify_center();
        toolbar.style().overflow.x = Some(gpui::Overflow::Scroll);
        toolbar.style().overflow.y = Some(gpui::Overflow::Hidden);
        toolbar.child(pill).into_any_element()
    }

    /// A 36x36 tool button: HugeIcons icon, micro shortcut digit in the
    /// bottom-right, tooltip on hover.
    fn tool_button(
        &self,
        tool: Tool,
        shortcut: &'static str,
        label: &'static str,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let active = self.tool == tool;
        let icon_color = if active { theme.accent } else { theme.text_1 };
        let mut button = div()
            .id(gpui::ElementId::Name(tool.icon_slug().into()))
            .w(px(36.0))
            .h(px(36.0))
            .rounded(px(R_BTN))
            .flex()
            .items_center()
            .justify_center()
            .relative()
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(theme.bg_elevated))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.set_tool(tool);
                    cx.notify();
                }),
            )
            .child(crate::ui::chrome_icon(tool.icon_slug(), 20.0, icon_color));
        if active {
            button = button.bg(theme.accent_soft);
        }
        if shortcut.chars().count() == 1 {
            button = button.child(
                div()
                    .absolute()
                    .right(px(3.0))
                    .bottom(px(2.0))
                    .text_size(px(9.0))
                    .text_color(theme.text_3)
                    .font_family(default_font_family())
                    .child(shortcut),
            );
        }
        tooltip(button, tooltip_text(label, Some(shortcut)))
    }

    /// A 36x36 icon button for bar actions (undo, redo, panel toggles).
    #[allow(clippy::too_many_arguments)]
    fn action_icon_button(
        &self,
        slug: &'static str,
        label: &'static str,
        disabled: bool,
        active: bool,
        theme: &Theme,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let icon_color = if active { theme.accent } else { theme.text_1 };
        let mut button = div()
            .id(gpui::ElementId::Name(label.to_lowercase().into()))
            .w(px(36.0))
            .h(px(36.0))
            .rounded(px(R_BTN))
            .flex()
            .items_center()
            .justify_center()
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(theme.bg_elevated))
            .on_mouse_down(MouseButton::Left, handler)
            .child(crate::ui::chrome_icon(slug, 18.0, icon_color));
        if disabled {
            button = button.opacity(0.45);
        }
        if active {
            button = button.bg(theme.accent_soft);
        }
        tooltip(button, tooltip_text(label, None))
    }

    fn panel_icon_button(
        &self,
        slug: &'static str,
        label: &'static str,
        panel: EditorPanel,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        self.action_icon_button(
            slug,
            label,
            false,
            self.panel == Some(panel),
            theme,
            cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                this.show_panel(panel);
                cx.notify();
            }),
        )
    }

    /// Color / width / z-order / delete strip shown while a shape is
    /// selected: a floating pill 12 px above the toolbar (spec §5.2).
    fn selection_bar(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if self.selected.len() != 1 || self.text_editing.is_some() {
            return None;
        }
        let theme = Theme::current(cx);
        let selected_color_index = self.selected_color_index();
        let selected_width_index = self.selected_width_index();

        let mut pill = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .h(px(SELECTION_BAR_HEIGHT))
            .px(px(8.0))
            .rounded(px(R_PILL))
            .bg(theme.bg_toolbar)
            .border_1()
            .border_color(theme.hairline)
            .shadow(theme.shadow_floating.clone());

        for (index, color) in PALETTE.iter().enumerate() {
            let swatch = rgb(parse_hex(color));
            let mut chip = div()
                .id(gpui::ElementId::Name(format!("color-{index}").into()))
                .w(px(18.0))
                .h(px(18.0))
                .rounded_full()
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
                chip = chip.border_2().border_color(theme.accent);
            } else {
                chip = chip.border_1().border_color(theme.hairline_strong);
            }
            pill = pill.child(chip);
        }

        if selected_width_index.is_some() {
            pill = pill.child(separator(&theme));
            for (index, label) in ["S", "M", "L", "XL"].iter().enumerate() {
                let active = selected_width_index == Some(index);
                let mut chip = div()
                    .id(gpui::ElementId::Name(format!("width-{index}").into()))
                    .w(px(24.0))
                    .h(px(24.0))
                    .rounded(px(R_INPUT))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor(gpui::CursorStyle::PointingHand)
                    .text_size(px(11.0))
                    .font_family(default_font_family())
                    .text_color(if active { theme.text_1 } else { theme.text_3 })
                    .child(*label)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            this.apply_width_to_selection(index);
                            cx.notify();
                        }),
                    );
                if active {
                    chip = chip.bg(theme.accent_soft).text_color(theme.accent);
                }
                pill = pill.child(chip);
            }
        }

        let z_order = self.selected_index().is_some();
        if z_order {
            pill = pill
                .child(separator(&theme))
                .child(self.selection_icon_button(
                    "send_backward",
                    "Send backward",
                    &theme,
                    cx.listener(|this, _: &MouseDownEvent, _, cx| {
                        this.reorder_selected(false);
                        cx.notify();
                    }),
                ))
                .child(self.selection_icon_button(
                    "bring_forward",
                    "Bring forward",
                    &theme,
                    cx.listener(|this, _: &MouseDownEvent, _, cx| {
                        this.reorder_selected(true);
                        cx.notify();
                    }),
                ))
                .child(separator(&theme))
                .child(self.selection_icon_button(
                    "delete",
                    "Delete",
                    &theme,
                    cx.listener(|this, _: &MouseDownEvent, _, cx| {
                        this.delete_selected();
                        cx.notify();
                    }),
                ));
        }

        Some(
            div()
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .bottom(px(TOOLBAR_BOTTOM + TOOLBAR_HEIGHT + 12.0))
                .flex()
                .justify_center()
                .child(pill)
                .into_any_element(),
        )
    }

    /// An icon button inside the selection strip; delete is tinted danger.
    fn selection_icon_button(
        &self,
        slug: &'static str,
        label: &'static str,
        theme: &Theme,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> gpui::Stateful<gpui::Div> {
        let icon_color = if slug == "delete" {
            theme.danger
        } else {
            theme.text_1
        };
        let button = div()
            .id(gpui::ElementId::Name(slug.into()))
            .w(px(32.0))
            .h(px(32.0))
            .rounded(px(R_BTN))
            .flex()
            .items_center()
            .justify_center()
            .cursor(gpui::CursorStyle::PointingHand)
            .hover(move |style| style.bg(theme.bg_elevated))
            .on_mouse_down(MouseButton::Left, handler)
            .child(crate::ui::chrome_icon(slug, 18.0, icon_color));
        tooltip(button, label)
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
            Shape::Icon(shape) => &shape.color,
            Shape::Effect(_) => return None,
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
            Shape::Text(_) | Shape::Counter(_) | Shape::Icon(_) | Shape::Effect(_) => return None,
        };
        let widths = if matches!(&self.doc.shapes[index], Shape::Arrow(_)) {
            &ARROW_STROKE_WIDTHS
        } else {
            &STROKE_WIDTHS
        };
        widths
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                (*left - width).abs().total_cmp(&(*right - width).abs())
            })
            .map(|(index, _)| index)
    }
}

/// A 1px vertical hairline between toolbar groups.
fn separator(theme: &Theme) -> gpui::Div {
    div().w(px(1.0)).h(px(24.0)).bg(theme.hairline_strong)
}

/// Attach a hover tooltip to a stateful div.
fn tooltip(element: gpui::Stateful<gpui::Div>, text: &'static str) -> gpui::Stateful<gpui::Div> {
    crate::ui::with_tooltip(element, text)
}

fn tooltip_text(label: &'static str, shortcut: Option<&'static str>) -> &'static str {
    match (label, shortcut) {
        ("Select", Some("1")) => "Select (1)",
        ("Rectangle", Some("2")) => "Rectangle (2)",
        ("Ellipse", Some("3")) => "Ellipse (3)",
        ("Arrow", Some("4")) => "Arrow (4)",
        ("Pen", Some("5")) => "Pen (5)",
        ("Text", Some("6")) => "Text (6)",
        ("Counter", Some("7")) => "Counter (7)",
        ("Crop", Some("8")) => "Crop (8)",
        ("Blur", Some("B")) => "Blur (B)",
        ("Pixelate", Some("M")) => "Pixelate (M)",
        ("Redact", Some("Shift+R")) => "Redact (Shift+R)",
        ("Icon", Some("I")) => "Icon (I)",
        _ => label,
    }
}

/// A toolbar text button (Copy, Save PNG).
fn text_button(label: &'static str, accent_fill: bool, theme: &Theme) -> gpui::Div {
    let mut button = div()
        .h(px(36.0))
        .px(px(12.0))
        .rounded(px(R_BTN))
        .flex()
        .items_center()
        .gap_1()
        .text_size(px(12.0))
        .font_family(default_font_family())
        .cursor(gpui::CursorStyle::PointingHand)
        .child(label);
    if accent_fill {
        button = button
            .bg(theme.accent)
            .text_color(gpui::white())
            .child(crate::ui::chrome_icon("save", 16.0, gpui::white()));
    } else {
        button = button
            .text_color(theme.text_1)
            .hover(move |style| style.bg(theme.bg_elevated));
    }
    button
}

fn panel_label(label: &'static str, theme: &Theme) -> gpui::Div {
    div()
        .font_family(default_font_family())
        .text_size(px(12.0))
        .text_color(theme.text_2)
        .child(label)
}

fn panel_message(message: &str, theme: &Theme) -> gpui::Div {
    div()
        .px_2()
        .py_2()
        .rounded(px(R_INPUT))
        .bg(theme.bg_elevated)
        .text_size(px(11.0))
        .text_color(theme.text_2)
        .font_family(default_font_family())
        .child(message.to_string())
}

fn icon_preview(image: Arc<RenderImage>) -> gpui::AnyElement {
    canvas(
        move |_, _, _| image.clone(),
        |bounds, image, window, _| {
            window
                .paint_image(
                    bounds,
                    bounds,
                    Corners::<Pixels>::default(),
                    image.clone(),
                    0,
                    false,
                )
                .ok();
        },
    )
    .w(px(24.0))
    .h(px(24.0))
    .into_any_element()
}

fn toggle_preference(
    label: &'static str,
    enabled: bool,
    theme: &Theme,
    listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    // Toggle: a 34x20 track with a knob, accent when on (spec §5.6).
    let knob_color = if enabled { gpui::white() } else { theme.text_3 };
    div()
        .id(gpui::ElementId::Name(label.to_lowercase().into()))
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px_2()
        .py_1()
        .rounded(px(R_INPUT))
        .bg(theme.bg_elevated)
        .text_size(px(11.0))
        .text_color(theme.text_1)
        .font_family(default_font_family())
        .cursor(gpui::CursorStyle::PointingHand)
        .hover(move |style| style.bg(theme.hairline_strong))
        .child(label)
        .child(
            div()
                .w(px(34.0))
                .h(px(20.0))
                .rounded_full()
                .bg(if enabled {
                    theme.accent
                } else {
                    theme.hairline_strong
                })
                .border_1()
                .border_color(theme.hairline_strong)
                .flex()
                .items_center()
                .px(px(2.0))
                .child(div().w(px(16.0)).h(px(14.0)).rounded_full().bg(knob_color))
                .when(enabled, |track| track.justify_end())
                .when(!enabled, |track| track.justify_start()),
        )
        .on_mouse_down(MouseButton::Left, listener)
}

/// A Dark / Light two-chip switch for the preferences panel.
fn theme_switch(
    current: crate::theme::Scheme,
    theme: &Theme,
    listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> gpui::Div {
    let mut row = div().flex().flex_row().gap_1();
    for scheme in [crate::theme::Scheme::Dark, crate::theme::Scheme::Light] {
        let active = scheme == current;
        let mut chip = div()
            .id(gpui::ElementId::Name(format!("theme-{:?}", scheme).into()))
            .px_2()
            .py_1()
            .rounded(px(R_INPUT))
            .text_size(px(11.0))
            .font_family(default_font_family())
            .cursor(gpui::CursorStyle::PointingHand)
            .child(scheme.label());
        if active {
            chip = chip.bg(theme.accent_soft).text_color(theme.accent);
        } else {
            chip = chip.bg(theme.bg_elevated).text_color(theme.text_1);
        }
        row = row.child(chip);
    }
    row.on_mouse_down(MouseButton::Left, listener)
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

/// Decode a library capture into a 128px-wide thumbnail for the rail.
fn decode_thumbnail(id: &str) -> Option<Arc<RenderImage>> {
    let png = library::read_image_png(id).ok()?;
    let decoded = image::ImageReader::new(std::io::Cursor::new(&png))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?
        .into_rgba8();
    let (width, height) = decoded.dimensions();
    let scale = 128.0 / width.max(1) as f32;
    let thumb = if scale < 1.0 {
        image::imageops::resize(
            &decoded,
            128,
            ((height as f32) * scale).round().max(1.0) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        decoded
    };
    Some(crate::pin::rgba_to_render_image(thumb))
}

/// A contain-fit thumbnail painter for a rail row.
fn thumbnail_canvas(thumb: Arc<RenderImage>, width: u32, height: u32) -> gpui::AnyElement {
    canvas(
        move |_, _, _| thumb.clone(),
        move |bounds, thumb, window, _| {
            let (w, h) = (width.max(1) as f32, height.max(1) as f32);
            let fit = (bounds.size.width / px(w))
                .min(bounds.size.height / px(h))
                .min(1.0);
            let draw = size(px(w * fit), px(h * fit));
            let image_bounds = Bounds {
                origin: gpui::point(
                    bounds.origin.x + (bounds.size.width - draw.width) / 2.0,
                    bounds.origin.y + (bounds.size.height - draw.height) / 2.0,
                ),
                size: draw,
            };
            window
                .paint_image(
                    bounds,
                    image_bounds,
                    Corners::all(px(R_THUMB)),
                    thumb.clone(),
                    0,
                    false,
                )
                .ok();
        },
    )
    .w_full()
    .h_full()
    .into_any_element()
}

/// Day group header for a capture timestamp (UTC; no local-tz in std).
fn day_label(millis: i64) -> String {
    let (year, month, day) = civil_from_days(millis.div_euclid(86_400_000));
    let today = {
        let now = library::now_millis();
        let (y, m, d) = civil_from_days(now.div_euclid(86_400_000));
        (y, m, d)
    };
    let yesterday = {
        let (y, m, d) = civil_from_days(library::now_millis().div_euclid(86_400_000) - 1);
        (y, m, d)
    };
    match (year, month, day) {
        _ if (year, month, day) == today => "Today".to_string(),
        _ if (year, month, day) == yesterday => "Yesterday".to_string(),
        _ => format!(
            "{} {:02}, {}",
            MONTHS[(month - 1).clamp(1, 12) as usize - 1],
            day,
            year
        ),
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days-since-epoch → (year, month, day), Howard Hinnant's civil_from_days.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// HH:MM for a capture timestamp (UTC; no local-tz in std).
fn time_label(millis: i64) -> String {
    let seconds = millis.rem_euclid(86_400_000) / 1000;
    format!("{:02}:{:02}", seconds / 3600, (seconds % 3600) / 60)
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
    backdrop: gpui::Hsla,
    image: Arc<RenderImage>,
    image_width: u32,
    image_height: u32,
    zoom: f32,
    pan: Point<Pixels>,
    shapes: Vec<Shape>,
    icon_images: HashMap<ShapeId, Arc<RenderImage>>,
    flattened_preview: Option<Arc<RenderImage>>,
    preview: Option<Shape>,
    selected: Vec<ShapeId>,
    marquee: Option<Rect>,
    #[allow(dead_code)]
    space_or_pan: bool,
    crop: Option<Rect>,
}

fn paint_canvas(bounds: Bounds<Pixels>, state: &CanvasState, window: &mut Window) {
    // Canvas background behind the image (always dark-neutral, both schemes).
    window.paint_quad(gpui::quad(
        bounds,
        px(0.),
        state.backdrop,
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
    let image = state
        .flattened_preview
        .clone()
        .unwrap_or_else(|| state.image.clone());
    window
        .paint_image(
            bounds,
            image_bounds,
            Corners::<Pixels>::default(),
            image,
            0,
            false,
        )
        .ok();

    if state.flattened_preview.is_some() {
        if let Some(preview) = &state.preview {
            paint_shape(bounds, state, window, preview, false);
        }
    } else {
        for shape in state.shapes.iter().chain(state.preview.as_ref()) {
            let selected = state.selected.iter().any(|id| id == shape.id());
            paint_shape(bounds, state, window, shape, selected);
        }
    }

    if state.selected.len() == 1 {
        if let Some(shape) = state
            .shapes
            .iter()
            .find(|shape| shape.id() == &state.selected[0])
        {
            if let Shape::Arrow(arrow) = shape {
                paint_arrow_selection_ui(bounds, state, window, arrow);
            } else {
                let screen = shape_bounds_to_screen(shape, state, bounds);
                paint_selection_ui(window, &screen, true);
            }
        }
    } else if state.selected.len() > 1 {
        for shape in state
            .shapes
            .iter()
            .filter(|shape| state.selected.iter().any(|id| id == shape.id()))
        {
            let screen = shape_bounds_to_screen(shape, state, bounds);
            paint_selection_ui(window, &screen, false);
        }
    }

    if let Some(marquee) = state.marquee {
        paint_marquee(bounds, state, window, marquee);
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
        Shape::Effect(effect) => {
            let bounds = effect.bounds;
            let screen = bounds_of_canvas(bounds.x, bounds.y, bounds.w, bounds.h, _bounds, state);
            window.paint_quad(gpui::quad(
                screen,
                px(0.0),
                color.alpha(0.2),
                px(0.0),
                gpui::transparent_black(),
                Default::default(),
            ));
            if let Some(path) = rect_path(screen, px(1.5)) {
                window.paint_path(path, color);
            }
        }
        Shape::Icon(icon) => {
            if let Some(image) = state.icon_images.get(&icon.id) {
                let screen = Bounds {
                    origin: image_to_screen(state, _bounds, icon.x, icon.y),
                    size: Size {
                        width: px(icon.size * state.zoom),
                        height: px(icon.size * state.zoom),
                    },
                };
                window
                    .paint_image(
                        _bounds,
                        screen,
                        Corners::<Pixels>::default(),
                        image.clone(),
                        0,
                        false,
                    )
                    .ok();
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

fn paint_selection_ui(window: &mut Window, screen_bounds: &Rect, handles: bool) {
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
    if !handles {
        return;
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

fn paint_arrow_selection_ui(
    bounds: Bounds<Pixels>,
    state: &CanvasState,
    window: &mut Window,
    arrow: &ArrowShape,
) {
    let handle_size = px(HANDLE_SCREEN_SIZE + 2.0);
    for (x, y) in [(arrow.x1, arrow.y1), (arrow.x2, arrow.y2)] {
        let center = image_to_screen(state, bounds, x, y);
        let handle_bounds = Bounds {
            origin: Point {
                x: center.x - handle_size / 2.0,
                y: center.y - handle_size / 2.0,
            },
            size: Size {
                width: handle_size,
                height: handle_size,
            },
        };
        window.paint_quad(gpui::quad(
            handle_bounds,
            handle_size / 2.0,
            rgb(0xffffff),
            px(1.5),
            rgb(0x3B82F6),
            Default::default(),
        ));
    }
}

fn paint_marquee(bounds: Bounds<Pixels>, state: &CanvasState, window: &mut Window, rect: Rect) {
    let screen_bounds = Bounds {
        origin: image_to_screen(state, bounds, rect.x, rect.y),
        size: Size {
            width: px(rect.w * state.zoom),
            height: px(rect.h * state.zoom),
        },
    };
    window.paint_quad(gpui::quad(
        screen_bounds,
        px(0.0),
        rgb(0x3B82F6).alpha(0.12),
        px(1.0),
        rgb(0x3B82F6),
        Default::default(),
    ));
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
    paint_selection_ui(window, &crop_bounds, true);
}

fn shape_color(shape: &Shape) -> gpui::Hsla {
    match shape {
        Shape::Effect(effect) => match &effect.kind {
            EffectKind::Blur { .. } => rgb(0x3b82f6).into(),
            EffectKind::Pixelate { .. } => rgb(0xf08c00).into(),
            EffectKind::Redact => rgb(0x111827).into(),
        },
        Shape::Rectangle(shape) => rgb(parse_hex(&shape.color)).into(),
        Shape::Ellipse(shape) => rgb(parse_hex(&shape.color)).into(),
        Shape::Arrow(shape) => rgb(parse_hex(&shape.color)).into(),
        Shape::Pen(shape) => rgb(parse_hex(&shape.color)).into(),
        Shape::Text(shape) => rgb(parse_hex(&shape.color)).into(),
        Shape::Counter(shape) => rgb(parse_hex(&shape.color)).into(),
        Shape::Icon(shape) => rgb(parse_hex(&shape.color)).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorSource, EditorWindow, open, shortcut_from_keys};
    use crate::editor::model::{ARROW_STROKE_WIDTHS, ArrowShape, RectShape, Shape};
    use crate::library::{AnnotationSource, CaptureMetadata, ImageSize};
    use gpui::{
        MouseButton, MouseMoveEvent, MouseUpEvent, TestAppContext, VisualTestContext, point, px,
    };

    fn source(title: &str, width: u32, height: u32) -> EditorSource {
        EditorSource {
            png: crate::capture::encode_png(
                width,
                height,
                &vec![0x5a; width as usize * height as usize * 4],
            )
            .unwrap(),
            title: title.to_string(),
            annotations: AnnotationSource::Transient,
        }
    }

    fn rectangle(id: &str, x: f32, y: f32) -> Shape {
        Shape::Rectangle(RectShape {
            id: id.to_string(),
            x,
            y,
            w: 20.0,
            h: 20.0,
            color: "#e03131".to_string(),
            stroke_width: 2.0,
        })
    }

    #[test]
    fn shortcut_recording_requires_a_modifier_and_supported_key() {
        assert_eq!(
            shortcut_from_keys("4", true, false, false, true).as_deref(),
            Some("ctrl+shift+4")
        );
        assert_eq!(
            shortcut_from_keys("F5", false, false, true, false).as_deref(),
            Some("alt+f5")
        );
        assert!(shortcut_from_keys("4", false, false, false, false).is_none());
        assert!(shortcut_from_keys("escape", true, false, false, false).is_none());
    }

    #[gpui::test]
    async fn opening_a_bad_capture_keeps_the_existing_editor_unchanged() {
        let cx = TestAppContext::single();
        cx.update(|cx| open(cx, source("First", 2, 2)).unwrap());
        let handle = cx.windows()[0].downcast::<EditorWindow>().unwrap();

        let initial = handle
            .read_with(&cx, |editor, _| {
                (editor.title.clone(), editor.rgba.dimensions())
            })
            .unwrap();
        assert_eq!(initial, ("First".to_string(), (2, 2)));

        let error = cx.update(|cx| {
            open(
                cx,
                EditorSource {
                    png: b"not a PNG".to_vec(),
                    title: "Broken".to_string(),
                    annotations: AnnotationSource::Transient,
                },
            )
        });
        assert!(error.is_err());

        let after_failure = handle
            .read_with(&cx, |editor, _| {
                (editor.title.clone(), editor.rgba.dimensions())
            })
            .unwrap();
        assert_eq!(after_failure, initial);

        cx.update(|cx| open(cx, source("Second", 3, 1)).unwrap());
        assert_eq!(cx.windows().len(), 1);
        let after_reuse = handle
            .read_with(&cx, |editor, _| {
                (editor.title.clone(), editor.rgba.dimensions())
            })
            .unwrap();
        assert_eq!(after_reuse, ("Second".to_string(), (3, 1)));
    }

    #[gpui::test]
    async fn deleting_the_active_capture_is_blocked() {
        let cx = TestAppContext::single();
        cx.update(|cx| open(cx, source("Current", 2, 2)).unwrap());
        let handle = cx.windows()[0].downcast::<EditorWindow>().unwrap();

        cx.update(|cx| {
            handle
                .update(cx, |editor, _, cx| {
                    editor.doc.image_id = Some("active-capture".to_string());
                    editor.delete_library_capture("active-capture", cx);
                    assert_eq!(
                        editor.library_notice.as_deref(),
                        Some("Close this capture before deleting it.")
                    );
                })
                .unwrap();
        });
    }

    #[gpui::test]
    async fn marquee_tracks_the_cursor_selects_intersections_and_moves_them_together() {
        let cx = TestAppContext::single();
        cx.update(|cx| open(cx, source("Marquee", 200, 200)).unwrap());
        let handle = cx.windows()[0].downcast::<EditorWindow>().unwrap();

        cx.update(|cx| {
            handle
                .update(cx, |editor, _, _| {
                    editor.zoom = 1.0;
                    editor.pan = Default::default();
                    editor.doc.shapes.push(rectangle("one", 10.0, 10.0));
                    editor.doc.shapes.push(rectangle("two", 45.0, 45.0));
                    editor.doc.shapes.push(rectangle("outside", 110.0, 110.0));

                    editor.select_mouse_down(0.0, 0.0);
                    editor.on_canvas_mouse_move(&MouseMoveEvent {
                        position: point(px(70.0), px(70.0)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Default::default(),
                    });
                    assert!(matches!(
                        &editor.drag,
                        super::Drag::Marquee { current, .. } if *current == (70.0, 70.0)
                    ));
                    editor.on_canvas_mouse_up(&MouseUpEvent {
                        position: point(px(70.0), px(70.0)),
                        button: MouseButton::Left,
                        ..Default::default()
                    });
                    assert_eq!(editor.selected, vec!["one", "two"]);

                    editor.select_mouse_down(10.0, 20.0);
                    assert!(matches!(editor.drag, super::Drag::MoveMany { .. }));
                    editor.on_canvas_mouse_move(&MouseMoveEvent {
                        position: point(px(20.0), px(30.0)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Default::default(),
                    });
                    editor.on_canvas_mouse_up(&MouseUpEvent {
                        position: point(px(20.0), px(30.0)),
                        button: MouseButton::Left,
                        ..Default::default()
                    });
                    assert_eq!(editor.doc.shapes[0].bounds().x, 20.0);
                    assert_eq!(editor.doc.shapes[0].bounds().y, 20.0);
                    assert_eq!(editor.doc.shapes[1].bounds().x, 55.0);
                    assert_eq!(editor.doc.shapes[1].bounds().y, 55.0);
                })
                .unwrap();
        });
    }

    #[gpui::test]
    async fn arrow_selection_uses_endpoint_handles_and_shaft_drag_not_a_box() {
        let cx = TestAppContext::single();
        cx.update(|cx| open(cx, source("Arrow", 200, 200)).unwrap());
        let handle = cx.windows()[0].downcast::<EditorWindow>().unwrap();

        cx.update(|cx| {
            handle
                .update(cx, |editor, _, _| {
                    editor.zoom = 1.0;
                    editor.pan = Default::default();
                    editor.doc.shapes.push(Shape::Arrow(ArrowShape {
                        id: "arrow".to_string(),
                        x1: 20.0,
                        y1: 20.0,
                        x2: 120.0,
                        y2: 80.0,
                        color: "#1a1a1a".to_string(),
                        stroke_width: 4.0,
                    }));
                    editor.selected = vec!["arrow".to_string()];
                    editor.apply_width_to_selection(3);
                    let Shape::Arrow(arrow) = &editor.doc.shapes[0] else {
                        unreachable!();
                    };
                    assert_eq!(arrow.stroke_width, ARROW_STROKE_WIDTHS[3]);

                    editor.select_mouse_down(20.0, 20.0);
                    assert!(matches!(
                        editor.drag,
                        super::Drag::ArrowEndpoint {
                            endpoint: super::ArrowEndpoint::Start,
                            ..
                        }
                    ));
                    editor.on_canvas_mouse_move(&MouseMoveEvent {
                        position: point(px(30.0), px(40.0)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Default::default(),
                    });
                    editor.on_canvas_mouse_up(&MouseUpEvent {
                        position: point(px(30.0), px(40.0)),
                        button: MouseButton::Left,
                        ..Default::default()
                    });
                    let Shape::Arrow(arrow) = &editor.doc.shapes[0] else {
                        unreachable!();
                    };
                    assert_eq!(
                        (arrow.x1, arrow.y1, arrow.x2, arrow.y2),
                        (30.0, 40.0, 120.0, 80.0)
                    );

                    editor.select_mouse_down(75.0, 60.0);
                    assert!(matches!(editor.drag, super::Drag::Move { .. }));
                    editor.on_canvas_mouse_move(&MouseMoveEvent {
                        position: point(px(85.0), px(70.0)),
                        pressed_button: Some(MouseButton::Left),
                        modifiers: Default::default(),
                    });
                    editor.on_canvas_mouse_up(&MouseUpEvent {
                        position: point(px(85.0), px(70.0)),
                        button: MouseButton::Left,
                        ..Default::default()
                    });
                    let Shape::Arrow(arrow) = &editor.doc.shapes[0] else {
                        unreachable!();
                    };
                    assert_eq!(
                        (arrow.x1, arrow.y1, arrow.x2, arrow.y2),
                        (40.0, 50.0, 130.0, 90.0)
                    );

                    editor.select_mouse_down(85.0, 50.0);
                    assert!(matches!(editor.drag, super::Drag::Marquee { .. }));
                })
                .unwrap();
        });
    }

    #[gpui::test]
    async fn history_delete_action_does_not_bubble_into_the_row_open_action() {
        let cx = TestAppContext::single();
        cx.update(|cx| open(cx, source("History", 200, 200)).unwrap());
        let window = cx.windows()[0];
        let handle = window.downcast::<EditorWindow>().unwrap();
        cx.update(|cx| {
            handle
                .update(cx, |editor, _, cx| {
                    editor.captures = vec![CaptureMetadata {
                        id: "row-test".to_string(),
                        title: "History row".to_string(),
                        created_at: 1,
                        updated_at: 2,
                        image: ImageSize {
                            width: 20,
                            height: 20,
                        },
                    }];
                    editor.panel = Some(super::EditorPanel::Library);
                    cx.notify();
                })
                .unwrap();
        });

        let mut visual = VisualTestContext::from_window(window, &cx);
        visual.run_until_parked();
        let bounds = visual
            .debug_bounds("history-action-delete-row-test")
            .expect("delete action should be laid out");
        let center = point(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );
        visual.simulate_mouse_move(center, None, Default::default());
        visual.run_until_parked();
        visual.simulate_mouse_down(center, MouseButton::Left, Default::default());
        visual.simulate_mouse_up(center, MouseButton::Left, Default::default());
        visual.run_until_parked();

        let state = handle
            .read_with(&cx, |editor, _| {
                (editor.pending_delete.clone(), editor.library_notice.clone())
            })
            .unwrap();
        assert_eq!(state.0.as_deref(), Some("row-test"));
        assert_eq!(state.1, None);

        let open_bounds = visual
            .debug_bounds("history-action-open-row-test")
            .expect("open action should be laid out");
        let open_center = point(
            open_bounds.origin.x + open_bounds.size.width / 2.0,
            open_bounds.origin.y + open_bounds.size.height / 2.0,
        );
        visual.simulate_mouse_move(open_center, None, Default::default());
        visual.run_until_parked();
        visual.simulate_mouse_down(open_center, MouseButton::Left, Default::default());
        visual.simulate_mouse_up(open_center, MouseButton::Left, Default::default());
        visual.run_until_parked();
        let open_result = handle
            .read_with(&cx, |editor, _| editor.library_notice.clone())
            .unwrap();
        assert!(open_result.is_some());
    }

    #[gpui::test]
    async fn zoom_controls_work_and_toolbar_does_not_shift_with_the_history_panel() {
        let cx = TestAppContext::single();
        cx.update(|cx| open(cx, source("Toolbar", 200, 200)).unwrap());
        let window = cx.windows()[0];
        let handle = window.downcast::<EditorWindow>().unwrap();
        cx.update(|cx| {
            handle
                .update(cx, |editor, _, cx| {
                    editor.zoom = 1.0;
                    editor.pan = Default::default();
                    cx.notify();
                })
                .unwrap();
        });

        let mut visual = VisualTestContext::from_window(window, &cx);
        visual.run_until_parked();
        let before = visual
            .debug_bounds("editor-toolbar")
            .expect("toolbar should be laid out");
        let zoom_in = visual
            .debug_bounds("zoom-in")
            .expect("zoom control should be laid out");
        let center = point(
            zoom_in.origin.x + zoom_in.size.width / 2.0,
            zoom_in.origin.y + zoom_in.size.height / 2.0,
        );
        visual.simulate_mouse_down(center, MouseButton::Left, Default::default());
        visual.simulate_mouse_up(center, MouseButton::Left, Default::default());
        visual.run_until_parked();
        assert_eq!(handle.read_with(&cx, |editor, _| editor.zoom).unwrap(), 1.2);

        cx.update(|cx| {
            handle
                .update(cx, |editor, _, cx| {
                    editor.panel = Some(super::EditorPanel::Library);
                    cx.notify();
                })
                .unwrap();
        });
        visual.run_until_parked();
        let after = visual
            .debug_bounds("editor-toolbar")
            .expect("toolbar should remain laid out with the rail open");
        assert_eq!(before.origin.x, after.origin.x);
        assert_eq!(before.size.width, after.size.width);
    }
}
