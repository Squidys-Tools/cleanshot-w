import { useCallback, useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactElement, type RefObject } from "react";
import {
  ArrowShapeArrowheadEndStyle,
  ArrowShapeArrowheadStartStyle,
  ArrowShapeKindStyle,
  Box,
  DefaultColorStyle,
  DefaultDashStyle,
  DefaultFillStyle,
  DefaultFontStyle,
  DefaultSizeStyle,
  GeoShapeGeoStyle,
  getColorValue,
  useValue,
  type Editor,
  type TLShapeId,
} from "@tldraw/editor";
import { Tldraw, useEditor } from "tldraw";
import { BlurShapeUtil, CounterShapeUtil, PixelateShapeUtil, RedactShapeUtil } from "./customShapes";
import { BlurTool, CounterTool, PixelateTool, RedactTool } from "./customTools";
import {
  type IconLibraryId,
  LIBRARY_LABELS,
  cycleIconLib,
  getIconComponent,
  useIconLib,
} from "./IconLibrary";
import { dataUrlFromUrl, ensureBackgroundImage, getBackgroundImageAsset, serializeTldraw, type EditorController, type Exporter } from "../../lib/tldrawDoc";
import { applyEditorPreferences, persistEditorPreferences, readEditorPreferences } from "../../lib/preferences";
import type { Rect, RegionMode, TldrawState } from "../../types";
import { cropImage, loadImage } from "../../lib/storage";

const CUSTOM_SHAPE_UTILS = [CounterShapeUtil, BlurShapeUtil, PixelateShapeUtil, RedactShapeUtil];
const CUSTOM_TOOLS = [CounterTool, BlurTool, PixelateTool, RedactTool];

const SWATCH_COLORS = ["red", "orange", "yellow", "green", "blue", "violet", "black", "grey"] as const;

/* One-click colors in the dock; the caret opens the full palette. */
const QUICK_COLORS = ["red", "blue", "black"] as const;

const SIZES = [
  { id: "s", label: "S" },
  { id: "m", label: "M" },
  { id: "l", label: "L" },
  { id: "xl", label: "XL" },
] as const;

const GEO_SHAPES: { id: string; label: string; icon: string }[] = [
  { id: "rectangle", label: "Rectangle", icon: "rect" },
  { id: "oval", label: "Oval", icon: "ellipse" },
  { id: "diamond", label: "Diamond", icon: "diamond" },
  { id: "triangle", label: "Triangle", icon: "triangle" },
  { id: "trapezoid", label: "Trapezoid", icon: "trapezoid" },
  { id: "rhombus", label: "Rhombus", icon: "rhombus" },
  { id: "rhombus-2", label: "Rhombus 2", icon: "rhombus-2" },
  { id: "pentagon", label: "Pentagon", icon: "pentagon" },
  { id: "hexagon", label: "Hexagon", icon: "hexagon" },
  { id: "octagon", label: "Octagon", icon: "octagon" },
  { id: "star", label: "Star", icon: "star" },
  { id: "heart", label: "Heart", icon: "heart" },
  { id: "cloud", label: "Cloud", icon: "cloud" },
  { id: "check-box", label: "Check-box", icon: "check-box" },
  { id: "arrow-right", label: "Right arrow", icon: "arrow-right" },
  { id: "arrow-left", label: "Left arrow", icon: "arrow-left" },
  { id: "arrow-up", label: "Up arrow", icon: "arrow-up" },
  { id: "arrow-down", label: "Down arrow", icon: "arrow-down" },
  { id: "x-box", label: "X-box", icon: "x-box" },
];

const FILLS = [
  { id: "none", label: "No fill", icon: "fill-none" },
  { id: "semi", label: "Semi fill", icon: "fill-semi" },
  { id: "solid", label: "Solid fill", icon: "fill-solid" },
] as const;

const DASHES = [
  { id: "solid", label: "Solid" },
  { id: "dashed", label: "Dashed" },
  { id: "dotted", label: "Dotted" },
  { id: "draw", label: "Hand drawn" },
] as const;

const FONTS = [
  { id: "sans", label: "Sans" },
  { id: "serif", label: "Serif" },
  { id: "mono", label: "Mono" },
  { id: "draw", label: "Hand drawn" },
] as const;

const OPACITIES = [0.1, 0.25, 0.5, 0.75, 1] as const;

const ARROWHEADS = [
  { id: "none", label: "None" },
  { id: "arrow", label: "Arrow" },
  { id: "triangle", label: "Triangle" },
  { id: "bar", label: "Bar" },
  { id: "dot", label: "Dot" },
  { id: "pipe", label: "Pipe" },
  { id: "diamond", label: "Diamond" },
  { id: "inverted", label: "Inverted" },
  { id: "square", label: "Square" },
] as const;

const ARROW_KINDS = [
  { id: "arc", label: "Curved" },
  { id: "elbow", label: "Elbow" },
] as const;

type TldrawCanvasProps = {
  imageUrl: string;
  imgW: number;
  imgH: number;
  initialDoc: TldrawState;
  onChange: (doc: TldrawState) => void;
  controllerRef: { current: EditorController | null };
  onHistoryState: (state: { canUndo: boolean; canRedo: boolean }) => void;
  saveState: "saved" | "saving" | "error";
  saveError?: string | null;
  onRetrySave?: () => void;
  regionMode: RegionMode | null;
  onRegionModeChange: (mode: RegionMode | null) => void;
  onCrop: (imageBlob: Blob, image: { width: number; height: number }, annotations: TldrawState) => Promise<void>;
  onOcrRegion: (rect: Rect) => void;
};

export default function TldrawCanvas({
  imageUrl,
  imgW,
  imgH,
  initialDoc,
  onChange,
  controllerRef,
  onHistoryState,
  regionMode,
  onRegionModeChange,
  onCrop,
  onOcrRegion,
}: TldrawCanvasProps) {
  const [editor, setEditor] = useState<Editor | null>(null);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const initialRef = useRef(initialDoc);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const onHistoryStateRef = useRef(onHistoryState);
  onHistoryStateRef.current = onHistoryState;
  const saveTimer = useRef<number | null>(null);
  const pendingRef = useRef<TldrawState>(null);

  const scheduleSave = useCallback(() => {
    pendingRef.current = serializeTldraw(editor!);
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveTimer.current = null;
      const snap = pendingRef.current;
      pendingRef.current = null;
      if (snap) onChangeRef.current(snap);
    }, 350);
  }, [editor]);

  const onMount = useCallback(
    (ed: Editor) => {
      setEditor(ed);
      // Keep the chosen tool active after each stroke; the user switches
      // tools deliberately, not after every action.
      ed.updateInstanceState({ isToolLocked: true });
      void ensureBackgroundImage(ed, imageUrl, imgW, imgH);
    },
    [imageUrl, imgW, imgH],
  );

  useEffect(() => {
    if (!editor) return;
    restoreOnce(editor, initialRef.current);
    applyEditorPreferences(editor, readEditorPreferences());
    editor.setCurrentTool("select");
    const notifyHistoryState = () => {
      onHistoryStateRef.current({ canUndo: editor.canUndo(), canRedo: editor.canRedo() });
    };
    notifyHistoryState();
    const unsub = editor.store.listen(() => {
      scheduleSave();
      notifyHistoryState();
    });
    return () => {
      unsub();
      if (saveTimer.current) {
        window.clearTimeout(saveTimer.current);
        saveTimer.current = null;
        const snap = pendingRef.current;
        pendingRef.current = null;
        if (snap) onChangeRef.current(snap);
      }
    };
  }, [editor, scheduleSave]);

  useEffect(() => {
    if (!editor) return;
    // Frame the capture once layout has settled. Early frames can run while
    // the canvas, topbar, or dock are still laying out, so keep re-framing
    // until the container geometry is stable for several consecutive frames.
    let raf = 0;
    let lastW = -1;
    let lastH = -1;
    let stableFrames = 0;
    const start = performance.now();
    const tick = () => {
      const target = rootRef.current;
      const w = target?.getBoundingClientRect().width ?? -1;
      const h = target?.getBoundingClientRect().height ?? -1;
      if (w === lastW && h === lastH) stableFrames += 1;
      else {
        stableFrames = 0;
        lastW = w;
        lastH = h;
      }
      frameImage(editor, imgW, imgH);
      if (stableFrames < 5 && performance.now() - start < 2000) {
        raf = requestAnimationFrame(tick);
      }
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [editor, imgW, imgH]);

  useEffect(() => {
    if (!regionMode) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onRegionModeChange(null);
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, [onRegionModeChange, regionMode]);

  useEffect(() => {
    if (!editor) return;
    const exporter: Exporter = async () => {
      const shapes = editor.getCurrentPageShapes().filter((s) => s.parentId === editor.getCurrentPageId());
      const res = await editor.toImage(shapes, {
        format: "png",
        background: false,
        padding: 0,
        pixelRatio: 1,
        bounds: new Box(0, 0, imgW, imgH),
      });
      return { blob: res.blob, width: res.width, height: res.height };
    };
    controllerRef.current = {
      exportImage: exporter,
      undo: () => editor.undo(),
      redo: () => editor.redo(),
      canUndo: () => editor.canUndo(),
      canRedo: () => editor.canRedo(),
    };
    return () => {
      controllerRef.current = null;
    };
  }, [editor, imgW, imgH, controllerRef]);

  useEffect(() => {
    if (!editor) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (editor.getEditingShapeId()) return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;

      const mod = e.ctrlKey || e.metaKey;
      if (mod || e.altKey) return;

      const TOOL_KEYS: Record<string, string> = {
        v: "select",
        d: "draw",
        e: "eraser",
        h: "hand",
        l: "line",
        a: "arrow",
        t: "text",
        n: "note",
        f: "frame",
        k: "highlight",
        c: "cs-counter",
        b: "cs-blur",
        p: "cs-pixelate",
        x: "cs-redact",
        r: "rectangle",
        o: "ellipse",
      };
      const tool = TOOL_KEYS[e.key.toLowerCase()];
      if (!tool) return;
      e.preventDefault();
      e.stopPropagation();
      if (tool === "rectangle" || tool === "ellipse") {
        editor.run(() => {
          editor.setStyleForNextShapes(GeoShapeGeoStyle, tool as "rectangle" | "ellipse");
          editor.setCurrentTool("geo");
        });
        return;
      }
      editor.setCurrentTool(tool);
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, [editor]);

  return (
    <div className="cs-tldraw" ref={rootRef}>
      <Tldraw hideUi shapeUtils={CUSTOM_SHAPE_UTILS} tools={CUSTOM_TOOLS} onMount={onMount} options={{ maxShapesPerPage: 2000 }}>
        <div className="cs-ui">
          {regionMode && (
            <RegionOverlay
              mode={regionMode}
              editor={editor}
              rootRef={rootRef}
              imgW={imgW}
              imgH={imgH}
              onCancel={() => onRegionModeChange(null)}
              onApply={async (rect) => {
                if (regionMode === "ocr") {
                  onOcrRegion(rect);
                  onRegionModeChange(null);
                  return;
                }
                if (!editor) return;
                const asset = getBackgroundImageAsset(editor);
                const imageShape = editor.getCurrentPageShapes().find((shape) => shape.type === "image");
                if (!asset || asset.type !== "image" || !asset.props.src || !imageShape || imageShape.type !== "image") return;
                const nextBlob = await cropImage(new Blob([await (await fetch(asset.props.src)).blob()], { type: asset.props.mimeType || "image/png" }), rect);
                const nextUrl = URL.createObjectURL(nextBlob);
                try {
                  const nextSrc = await dataUrlFromUrl(nextUrl);
                  const nextDims = await loadImage(nextBlob);
                  const pageId = editor.getCurrentPageId();
                  const topLevelShapes = editor.getCurrentPageShapes().filter((shape) => shape.parentId === pageId && shape.type !== "image");
                  const deleted = topLevelShapes.filter((shape) => {
                    const bounds = editor.getShapePageBounds(shape);
                    return !bounds || bounds.maxX <= rect.x || bounds.maxY <= rect.y || bounds.minX >= rect.x + rect.width || bounds.minY >= rect.y + rect.height;
                  });
                  editor.run(() => {
                    editor.markHistoryStoppingPoint();
                    if (deleted.length) editor.deleteShapes(deleted.map((shape) => shape.id));
                    editor.updateShapes(
                      topLevelShapes
                        .filter((shape) => !deleted.some((item) => item.id === shape.id))
                        .map((shape) => ({ id: shape.id, type: shape.type, x: shape.x - rect.x, y: shape.y - rect.y })),
                    );
                    editor.updateAssets([{ id: asset.id, type: "image", props: { ...asset.props, src: nextSrc, w: nextDims.width, h: nextDims.height } }] as never);
                    editor.updateShapes([{ id: imageShape.id, type: "image", x: 0, y: 0, props: { w: nextDims.width, h: nextDims.height } }]);
                  });
                  frameImage(editor, nextDims.width, nextDims.height);
                  await onCrop(nextBlob, nextDims, serializeTldraw(editor));
                  onRegionModeChange(null);
                } finally {
                  URL.revokeObjectURL(nextUrl);
                }
              }}
            />
          )}
          <Toolbar cropActive={regionMode === "crop"} onCrop={() => onRegionModeChange("crop")} />
          <SelectionBar />
          <ZoomControls imgW={imgW} imgH={imgH} />
        </div>
      </Tldraw>
    </div>
  );
}

function restoreOnce(editor: Editor, state: TldrawState): void {
  if (!state) return;
  try {
    editor.loadSnapshot(state.snapshot);
  } catch (err) {
    console.error("Could not restore document", err);
  }
}

/* Fit the capture inside the app chrome: clear of the top chips, the dock,
   and the window edges, centered in what remains. */
function frameImage(ed: Editor, imgW: number, imgH: number): void {
  const vp = ed.getViewportScreenBounds();
  const M = 24; // breathing room between image and any UI
  let insetT = M;
  let insetB = M;
  // Measure real chrome (topbar, dock, zoom controls) so the image can
  // never touch it, whatever the window size.
  document.querySelectorAll<HTMLElement>("[data-chrome]").forEach((el) => {
    const r = el.getBoundingClientRect();
    if (r.width === 0 && r.height === 0) return;
    const midY = r.top + r.height / 2;
    if (midY < window.innerHeight * 0.45) {
      insetT = Math.max(insetT, r.bottom + M);
    } else if (midY > window.innerHeight * 0.55) {
      insetB = Math.max(insetB, window.innerHeight - r.top + M);
    }
  });
  const availW = Math.max(64, vp.width - 2 * M);
  const availH = Math.max(64, vp.height - insetT - insetB);

  // Frame the image shape's real page bounds so placement is correct even if
  // the shape is not at the page origin.
  const img = ed.getCurrentPageShapes().find((s) => s.type === "image");
  const b = img ? ed.getShapePageBounds(img.id) : null;
  const bw = b?.width ?? imgW;
  const bh = b?.height ?? imgH;
  const zoom = Math.min(Math.max(Math.min(availW / bw, availH / bh), 0.05), 8);

  // Desired on-screen top-left of the image, then solve the camera that
  // puts it there: screenPos = (pagePos - camera) * zoom.
  const sx = M + (availW - bw * zoom) / 2;
  const sy = insetT + (availH - bh * zoom) / 2;
  if (b) {
    ed.setCamera({ x: b.x - sx / zoom, y: b.y - sy / zoom, z: zoom });
  } else {
    ed.setCamera({ x: -sx / zoom, y: -sy / zoom, z: zoom });
  }
}

function runAction(editor: Editor, fn: () => void): void {
  editor.run(() => {
    editor.markHistoryStoppingPoint();
    fn();
  });
}

function editableShapeIds(editor: Editor): TLShapeId[] {
  return editor.getSelectedShapeIds().filter((id) => editor.getShape(id)?.type !== "image");
}

function setStyleForNextAndSelected(editor: Editor, style: unknown, value: unknown): void {
  editor.setStyleForNextShapes(style as never, value as never);
  // Only restyle the selection when the user is deliberately in Select mode.
  // After drawing, the new shape stays selected (tool lock), and style
  // changes must not reach back and alter it.
  if (editor.getCurrentToolId() === "select" && editableShapeIds(editor).length > 0) {
    editor.setStyleForSelectedShapes(style as never, value as never);
  }
}

function usePopover(): { ref: RefObject<HTMLDivElement | null>; open: boolean; toggle: () => void; close: () => void } {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [open]);
  return { ref, open, toggle: () => setOpen((o) => !o), close: () => setOpen(false) };
}

/* Opens after a short hover intent and closes shortly after the pointer
   leaves, so tool-specific options appear in place without a click. */
function useHoverPopover(openDelay = 400, closeDelay = 150): {
  ref: RefObject<HTMLDivElement | null>;
  open: boolean;
  show: () => void;
  hide: () => void;
  keep: () => void;
} {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);
  const timer = useRef<number | null>(null);
  const clearTimer = () => {
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
  };
  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [open]);
  useEffect(() => clearTimer, []);
  return {
    ref,
    open,
    show: () => {
      clearTimer();
      timer.current = window.setTimeout(() => setOpen(true), openDelay);
    },
    hide: () => {
      clearTimer();
      timer.current = window.setTimeout(() => setOpen(false), closeDelay);
    },
    keep: clearTimer,
  };
}

function normalizeRect(start: { x: number; y: number }, end: { x: number; y: number }): Rect {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  };
}

function RegionOverlay({
  mode,
  editor,
  rootRef,
  imgW,
  imgH,
  onCancel,
  onApply,
}: {
  mode: RegionMode;
  editor: Editor | null;
  rootRef: RefObject<HTMLDivElement | null>;
  imgW: number;
  imgH: number;
  onCancel: () => void;
  onApply: (rect: Rect) => Promise<void>;
}) {
  const [start, setStart] = useState<{ x: number; y: number } | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);
  const [busy, setBusy] = useState(false);
  const camera = useValue("cs-region-camera", () => editor?.getCamera(), [editor]);

  useEffect(() => {
    if (!editor) return;
    const target = rootRef.current;
    target?.focus();
  }, [editor, rootRef]);

  if (!editor) return null;
  void camera;

  const pagePointFromEvent = (event: ReactPointerEvent<HTMLDivElement>) => {
    const point = editor.screenToPage({ x: event.clientX, y: event.clientY });
    return {
      x: Math.max(0, Math.min(imgW, point.x)),
      y: Math.max(0, Math.min(imgH, point.y)),
    };
  };

  const localScreenPoint = (point: { x: number; y: number }) => {
    const screen = editor.pageToScreen(point);
    const bounds = rootRef.current?.getBoundingClientRect();
    return { x: screen.x - (bounds?.left ?? 0), y: screen.y - (bounds?.top ?? 0) };
  };

  const screenRect = rect
    ? (() => {
        const topLeft = localScreenPoint(rect);
        const bottomRight = localScreenPoint({ x: rect.x + rect.width, y: rect.y + rect.height });
        return { x: topLeft.x, y: topLeft.y, width: bottomRight.x - topLeft.x, height: bottomRight.y - topLeft.y };
      })()
    : null;

  return (
    <div
      className="region-overlay"
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onCancel();
        }
      }}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        const point = pagePointFromEvent(event);
        event.currentTarget.setPointerCapture(event.pointerId);
        setStart(point);
        setRect({ x: point.x, y: point.y, width: 0, height: 0 });
      }}
      onPointerMove={(event) => {
        if (!start) return;
        setRect(normalizeRect(start, pagePointFromEvent(event)));
      }}
      onPointerUp={(event) => {
        if (!start) return;
        setRect(normalizeRect(start, pagePointFromEvent(event)));
        setStart(null);
        event.currentTarget.releasePointerCapture(event.pointerId);
      }}
    >
      {!screenRect && <div className="region-overlay-wash" />}
      {screenRect && <div className="region-overlay-selection" style={screenRect}>
        <span>{Math.round(rect?.width ?? 0)} × {Math.round(rect?.height ?? 0)}</span>
      </div>}
      <div className="region-overlay-guide">
        <strong>{mode === "crop" ? "Crop screenshot" : "Read selected text"}</strong>
        <span>{screenRect ? "Adjust the selection or apply it" : "Drag over the image to select a region"}</span>
      </div>
      <div className="region-overlay-actions" onPointerDown={(event) => event.stopPropagation()}>
        <button className="command-btn quiet" onClick={onCancel} disabled={busy}>Cancel</button>
        <button
          className="command-btn primary"
          onClick={() => {
            if (!rect || rect.width < 8 || rect.height < 8 || busy) return;
            setBusy(true);
            void onApply(rect).finally(() => setBusy(false));
          }}
          disabled={!rect || rect.width < 8 || rect.height < 8 || busy}
        >
          {busy ? "Working…" : mode === "crop" ? "Crop image" : "Read text"}
        </button>
      </div>
    </div>
  );
}

/* ------------------------------ Toolbar ------------------------------ */

const TOOLS: { id: string; label: string; icon: string; kbd?: string }[] = [
  { id: "select", label: "Select", icon: "select", kbd: "V" },
  { id: "highlight", label: "Highlight", icon: "highlight", kbd: "K" },
  { id: "draw", label: "Draw", icon: "draw", kbd: "D" },
  { id: "eraser", label: "Eraser", icon: "eraser", kbd: "E" },
  { id: "hand", label: "Pan", icon: "hand", kbd: "H" },
  { id: "line", label: "Line", icon: "line", kbd: "L" },
  { id: "arrow", label: "Arrow", icon: "arrow", kbd: "A" },
  { id: "text", label: "Text", icon: "text", kbd: "T" },
  { id: "note", label: "Note", icon: "note", kbd: "N" },
  { id: "frame", label: "Frame", icon: "frame", kbd: "F" },
  { id: "cs-counter", label: "Step", icon: "counter", kbd: "C" },
  { id: "cs-blur", label: "Blur", icon: "blur", kbd: "B" },
  { id: "cs-pixelate", label: "Mosaic", icon: "pixelate", kbd: "P" },
  { id: "cs-redact", label: "Redact", icon: "redact", kbd: "X" },
  { id: "laser", label: "Laser", icon: "laser" },
];

const PRIMARY_TOOL_IDS = ["select", "highlight", "draw", "arrow", "text", "cs-counter", "cs-blur", "cs-redact"];

function Toolbar({ cropActive, onCrop }: { cropActive: boolean; onCrop: () => void }) {
  const editor = useEditor();
  const { toolId, geo, color, size, fill, dash, font, opacity, ahStart, ahEnd, ahKind } = useValue(
    "cs-toolbar",
    () => ({
      toolId: editor.getCurrentToolId(),
      geo: editor.getStyleForNextShape(GeoShapeGeoStyle),
      color: editor.getStyleForNextShape(DefaultColorStyle),
      size: editor.getStyleForNextShape(DefaultSizeStyle),
      fill: editor.getStyleForNextShape(DefaultFillStyle),
      dash: editor.getStyleForNextShape(DefaultDashStyle),
      font: editor.getStyleForNextShape(DefaultFontStyle),
      opacity: editor.getSharedOpacity(),
      ahStart: editor.getStyleForNextShape(ArrowShapeArrowheadStartStyle),
      ahEnd: editor.getStyleForNextShape(ArrowShapeArrowheadEndStyle),
      ahKind: editor.getStyleForNextShape(ArrowShapeKindStyle),
    }),
    [editor],
  );
  const theme = editor.getCurrentTheme();
  const palette = theme.colors[editor.getColorMode()];

  const geoPopover = usePopover();
  const colorPopover = usePopover();
  const morePopover = usePopover();
  const arrowPopover = useHoverPopover();
  const textPopover = useHoverPopover();

  // Icon library cycler — Shift+I to cycle libraries; shared via useIconLib
  const [iconLib, setIconLibState] = useIconLib();
  const [iconLibToast, setIconLibToast] = useState<string | null>(null);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.shiftKey && !e.ctrlKey && !e.metaKey && e.key.toLowerCase() === "i") {
        const tag = (e.target as HTMLElement)?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA") return;
        setIconLibState(cycleIconLib(iconLib));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [iconLib, setIconLibState]);
  // Toast when library changes
  useEffect(() => {
    if (iconLib === "svg") {
      setIconLibToast(null);
      return;
    }
    setIconLibToast(LIBRARY_LABELS[iconLib]);
    if (toastTimer.current) clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setIconLibToast(null), 1500);
    return () => { if (toastTimer.current) clearTimeout(toastTimer.current); };
  }, [iconLib]);

  const setTool = (id: string) => {
    if (id === "rectangle" || id === "ellipse") {
      editor.run(() => {
        editor.setStyleForNextShapes(GeoShapeGeoStyle, id as "rectangle" | "ellipse");
        editor.setCurrentTool("geo");
      });
      persistEditorPreferences(editor);
      return;
    }
    editor.setCurrentTool(id);
  };

  const activeGeo = GEO_SHAPES.find((g) => g.id === geo) ?? GEO_SHAPES[0];

  return (
    <>
    <div className="cs-toolbar" data-chrome>
      <div className="tool-group toolbar-primary-tools">
        {TOOLS.filter((t) => PRIMARY_TOOL_IDS.includes(t.id)).map((t) => {
          if (t.id === "arrow") {
            return (
              <div
                key={t.id}
                className="popover-wrap"
                ref={arrowPopover.ref}
                onMouseEnter={arrowPopover.show}
                onMouseLeave={arrowPopover.hide}
              >
                <button
                  className={`tool-btn ${toolId === t.id ? "active" : ""}`}
                  title={`${t.label}${t.kbd ? ` (${t.kbd})` : ""}`}
                  onClick={() => setTool(t.id)}
                >
                  <ToolIcon name={t.icon} iconLib={iconLib} />
                </button>
                <div className={arrowPopover.open ? "popover arrow-popover open" : "popover arrow-popover"} onMouseEnter={arrowPopover.keep} aria-hidden={!arrowPopover.open}>
                    <div className="popover-label">Arrowheads</div>
                    <div className="popover-row">
                      <select
                        value={ahStart}
                        onChange={(e) => {
                          setStyleForNextAndSelected(editor, ArrowShapeArrowheadStartStyle, e.target.value);
                          persistEditorPreferences(editor);
                        }}
                      >
                        {ARROWHEADS.map((a) => (
                          <option key={a.id} value={a.id}>
                            Start · {a.label}
                          </option>
                        ))}
                      </select>
                      <select
                        value={ahEnd}
                        onChange={(e) => {
                          setStyleForNextAndSelected(editor, ArrowShapeArrowheadEndStyle, e.target.value);
                          persistEditorPreferences(editor);
                        }}
                      >
                        {ARROWHEADS.map((a) => (
                          <option key={a.id} value={a.id}>
                            End · {a.label}
                          </option>
                        ))}
                      </select>
                      <select
                        value={ahKind}
                        onChange={(e) => {
                          setStyleForNextAndSelected(editor, ArrowShapeKindStyle, e.target.value);
                          persistEditorPreferences(editor);
                        }}
                      >
                        {ARROW_KINDS.map((k) => (
                          <option key={k.id} value={k.id}>
                            {k.label}
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>
                </div>
              );
            }
            if (t.id === "text") {
            return (
              <div
                key={t.id}
                className="popover-wrap"
                ref={textPopover.ref}
                onMouseEnter={textPopover.show}
                onMouseLeave={textPopover.hide}
              >
                <button
                  className={`tool-btn ${toolId === t.id ? "active" : ""}`}
                  title={`${t.label}${t.kbd ? ` (${t.kbd})` : ""}`}
                  onClick={() => setTool(t.id)}
                >
                  <ToolIcon name={t.icon} iconLib={iconLib} />
                </button>
                <div className={textPopover.open ? "popover font-popover open" : "popover font-popover"} onMouseEnter={textPopover.keep} aria-hidden={!textPopover.open}>
                  <div className="popover-label">Font</div>
                  <div className="segmented">
                    {FONTS.map((f) => (
                      <button
                        key={f.id}
                        className={`seg-btn ${font === f.id ? "active" : ""}`}
                        title={f.label}
                        onClick={() => {
                          setStyleForNextAndSelected(editor, DefaultFontStyle, f.id);
                            persistEditorPreferences(editor);
                          }}
                        >
                          <span className={`font-sample font-${f.id}`}>Aa</span>
                        </button>
                      ))}
                  </div>
                </div>
              </div>
            );
          }
          return (
            <button
              key={t.id}
              className={`tool-btn ${toolId === t.id ? "active" : ""}`}
              title={`${t.label}${t.kbd ? ` (${t.kbd})` : ""}`}
              onClick={() => setTool(t.id)}
            >
              <ToolIcon name={t.icon} iconLib={iconLib} />
            </button>
          );
        })}
        <button className={`tool-btn ${cropActive ? "active" : ""}`} title="Crop screenshot" onClick={onCrop}>
          <ToolIcon name="crop" iconLib={iconLib} />
        </button>
        <div className="popover-wrap" ref={morePopover.ref}>
          <button className={`tool-btn ${TOOLS.some((tool) => tool.id === toolId && !PRIMARY_TOOL_IDS.includes(tool.id)) ? "active" : ""}`} title="More tools" onClick={morePopover.toggle}>
            <ToolIcon name="more" iconLib={iconLib} />
          </button>
          {morePopover.open && (
            <div className="popover tools-popover">
              <div className="popover-label">More tools</div>
              {TOOLS.filter((t) => !PRIMARY_TOOL_IDS.includes(t.id)).map((t) => (
                <button
                  key={t.id}
                  className={`more-tool-btn ${toolId === t.id ? "active" : ""}`}
                  title={`${t.label}${t.kbd ? ` (${t.kbd})` : ""}`}
                  onClick={() => {
                    setTool(t.id);
                    morePopover.close();
                  }}
                >
                  <ToolIcon name={t.icon} iconLib={iconLib} />
                  <span>{t.label}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="divider" />

      <div className="prop-group">
        <div className="popover-wrap" ref={geoPopover.ref}>
          <button
            className={`tool-btn ${toolId === "geo" ? "active" : ""}`}
            title={`Shape: ${activeGeo.label}`}
            onClick={geoPopover.toggle}
          >
            <ToolIcon name={activeGeo.icon} iconLib={iconLib} />
          </button>
          <div className={geoPopover.open ? "popover geo-menu open" : "popover geo-menu"} aria-hidden={!geoPopover.open}>
              <div className="geo-grid">
                {GEO_SHAPES.map((g) => (
                  <button
                    key={g.id}
                    className={`geo-opt ${geo === g.id ? "active" : ""}`}
                    title={g.label}
                    onClick={() => {
                      editor.run(() => {
                        editor.setStyleForNextShapes(GeoShapeGeoStyle, g.id as never);
                        editor.setCurrentTool("geo");
                      });
                      persistEditorPreferences(editor);
                      geoPopover.close();
                    }}
                  >
                    <ToolIcon name={g.icon} iconLib={iconLib} />
                  </button>
                ))}
              </div>
              <div className="popover-label">Fill</div>
              <div className="segmented" role="group" aria-label="Fill">
                {FILLS.map((f) => (
                  <button
                    key={f.id}
                    className={`seg-btn ${fill === f.id ? "active" : ""}`}
                    title={f.label}
                    onClick={() => {
                      setStyleForNextAndSelected(editor, DefaultFillStyle, f.id);
                      persistEditorPreferences(editor);
                    }}
                  >
                    <ToolIcon name={f.icon} iconLib={iconLib} />
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>

      <div className="divider" />

      <div className="prop-group">
        {QUICK_COLORS.map((c) => (
          <button
            key={c}
            className={`color-chip ${color === c ? "active" : ""}`}
            title={`Color: ${c}`}
            aria-label={`Use ${c}`}
            onClick={() => {
              setStyleForNextAndSelected(editor, DefaultColorStyle, c);
              persistEditorPreferences(editor);
            }}
          >
            <span style={{ background: getColorValue(palette, c, "solid") }} />
          </button>
        ))}
        <div className="popover-wrap" ref={colorPopover.ref}>
          <button
            className="color-chip color-more"
            title="Color and stroke"
            aria-label="Color and stroke"
            onClick={colorPopover.toggle}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="m6 9 6 6 6-6" />
            </svg>
          </button>
          <div className={colorPopover.open ? "popover color-popover open" : "popover color-popover"} aria-hidden={!colorPopover.open}>
            <div className="color-grid">
              {SWATCH_COLORS.map((c) => (
                <button
                  key={c}
                  className={`swatch ${color === c ? "active" : ""}`}
                  style={{ background: getColorValue(palette, c, "solid") }}
                  title={c}
                  onClick={() => {
                    setStyleForNextAndSelected(editor, DefaultColorStyle, c);
                    persistEditorPreferences(editor);
                  }}
                />
              ))}
            </div>
            <div className="popover-label">Size</div>
            <div className="segmented" role="group" aria-label="Size">
              {SIZES.map((s) => (
                <button
                  key={s.id}
                  className={`size-btn ${size === s.id ? "active" : ""}`}
                  title={`Size ${s.label}`}
                  onClick={() => {
                    setStyleForNextAndSelected(editor, DefaultSizeStyle, s.id);
                    persistEditorPreferences(editor);
                  }}
                >
                  {s.label}
                </button>
              ))}
            </div>
            <div className="popover-label">Dash</div>
            <div className="segmented" role="group" aria-label="Dash">
              {DASHES.map((d) => (
                <button
                  key={d.id}
                  className={`seg-btn ${dash === d.id ? "active" : ""}`}
                  title={d.label}
                  onClick={() => {
                    setStyleForNextAndSelected(editor, DefaultDashStyle, d.id);
                    persistEditorPreferences(editor);
                  }}
                >
                  <span className={`dash-mark dash-${d.id}`} />
                </button>
              ))}
            </div>
            <div className="popover-label">Opacity</div>
            <div className="segmented" role="group" aria-label="Opacity">
              {OPACITIES.map((o) => (
                <button
                  key={o}
                  className={`seg-btn ${opacity !== undefined && opacity.type !== "mixed" && Math.abs(opacity.value - o) < 0.001 ? "active" : ""}`}
                  title={`${Math.round(o * 100)}%`}
                  onClick={() => {
                    if (editor.getCurrentToolId() === "select") {
                      editor.setOpacityForSelectedShapes(o);
                    }
                    editor.setOpacityForNextShapes(o);
                    persistEditorPreferences(editor);
                  }}
                >
                  {Math.round(o * 100)}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

    </div>
    {iconLibToast && <div className="icon-lib-toast">{iconLibToast}</div>}
    </>
  );
}

/* -------------------------- Selection bar ---------------------------- */

function SelectionBar() {
  const editor = useEditor();
  const alignPopover = usePopover();
  const orderPopover = usePopover();
  const { hasSel, canUngroup, selectedCount } = useValue(
    "cs-selection",
    () => {
      const ids = editor.getSelectedShapeIds().filter((id) => editor.getShape(id)?.type !== "image");
      return { hasSel: ids.length > 0, canUngroup: ids.some((id) => editor.getShape(id)?.type === "group"), selectedCount: ids.length };
    },
    [editor],
  );
  if (!hasSel) return null;

  const ids = () => editableShapeIds(editor);

  return (
    <div className="cs-selection">
      <span className="selection-label">{selectedCount} selected</span>
      <div className="divider" />
      <div className="sel-group">
        <button className="icon-btn" aria-label="Duplicate selection" title="Duplicate (Ctrl+D)" onClick={() => runAction(editor, () => editor.duplicateShapes(ids()))}>
          <ToolIcon name="duplicate" />
        </button>
        <button className="icon-btn" aria-label="Delete selection" title="Delete" onClick={() => runAction(editor, () => editor.deleteShapes(ids()))}>
          <ToolIcon name="trash" />
        </button>
      </div>
      <div className="divider" />
      <div className="sel-group">
        <button className="icon-btn" aria-label="Group selection" title="Group (Ctrl+G)" onClick={() => runAction(editor, () => editor.groupShapes(ids()))}>
          <ToolIcon name="group" />
        </button>
        <button
          className="icon-btn"
          aria-label="Ungroup selection"
          title="Ungroup (Ctrl+Shift+G)"
          disabled={!canUngroup}
          onClick={() => runAction(editor, () => editor.ungroupShapes(ids()))}
        >
          <ToolIcon name="ungroup" />
        </button>
      </div>
      <div className="divider" />
      <div className="sel-group">
        <div className="popover-wrap" ref={alignPopover.ref}>
            <button className="icon-btn" aria-label="Align and distribute" title="Align & distribute" onClick={alignPopover.toggle}>
            <ToolIcon name="align" />
          </button>
          {alignPopover.open && (
            <div className="popover align-popover">
              {(
                [
                  ["left", "Align left"],
                  ["center-horizontal", "Align center"],
                  ["right", "Align right"],
                  ["top", "Align top"],
                  ["center-vertical", "Align middle"],
                  ["bottom", "Align bottom"],
                ] as const
              ).map(([a, label]) => (
                <button key={a} className="popover-item" onClick={() => { runAction(editor, () => editor.alignShapes(ids(), a)); alignPopover.close(); }}>
                  {label}
                </button>
              ))}
              <div className="popover-divider" />
              <button className="popover-item" onClick={() => { runAction(editor, () => editor.distributeShapes(ids(), "horizontal")); alignPopover.close(); }}>
                Distribute horizontally
              </button>
              <button className="popover-item" onClick={() => { runAction(editor, () => editor.distributeShapes(ids(), "vertical")); alignPopover.close(); }}>
                Distribute vertically
              </button>
            </div>
          )}
        </div>
        <button className="icon-btn" title="Flip horizontally" onClick={() => runAction(editor, () => editor.flipShapes(ids(), "horizontal"))}>
          <ToolIcon name="flip-h" />
        </button>
        <button className="icon-btn" title="Flip vertically" onClick={() => runAction(editor, () => editor.flipShapes(ids(), "vertical"))}>
          <ToolIcon name="flip-v" />
        </button>
      </div>
      <div className="divider" />
      <div className="sel-group">
        <div className="popover-wrap" ref={orderPopover.ref}>
          <button className="icon-btn" title="Arrange" onClick={orderPopover.toggle}>
            <ToolIcon name="order" />
          </button>
          {orderPopover.open && (
            <div className="popover align-popover">
              <button className="popover-item" onClick={() => { runAction(editor, () => editor.bringToFront(ids())); orderPopover.close(); }}>
                Bring to front
              </button>
              <button className="popover-item" onClick={() => { runAction(editor, () => editor.bringForward(ids())); orderPopover.close(); }}>
                Bring forward
              </button>
              <button className="popover-item" onClick={() => { runAction(editor, () => editor.sendBackward(ids())); orderPopover.close(); }}>
                Send backward
              </button>
              <button className="popover-item" onClick={() => { runAction(editor, () => editor.sendToBack(ids())); orderPopover.close(); }}>
                Send to back
              </button>
            </div>
          )}
        </div>
        <button className="icon-btn" title="Toggle lock" onClick={() => runAction(editor, () => editor.toggleLock(ids()))}>
          <ToolIcon name="lock" />
        </button>
      </div>
    </div>
  );
}

/* ---------------------------- Zoom controls -------------------------- */

function ZoomControls({ imgW, imgH }: { imgW: number; imgH: number }) {
  const editor = useEditor();
  const zoom = useValue("cs-zoom", () => editor.getCamera().z, [editor]);
  return (
    <div className="cs-zoom" data-chrome>
      <button className="icon-btn" title="Zoom out" aria-label="Zoom out" onClick={() => editor.zoomOut()}>
        <svg viewBox="0 0 24 24" className="tool-icon" aria-hidden="true">
          <path d="M5 12h14" />
        </svg>
      </button>
      <span className="zoom-label">{Math.round(zoom * 100)}%</span>
      <button className="icon-btn" title="Zoom in" aria-label="Zoom in" onClick={() => editor.zoomIn()}>
        <svg viewBox="0 0 24 24" className="tool-icon" aria-hidden="true">
          <path d="M12 5v14M5 12h14" />
        </svg>
      </button>
      <button className="icon-btn" title="Fit to window" onClick={() => frameImage(editor, imgW, imgH)}>
        Fit
      </button>
    </div>
  );
}

/* ------------------------------- Icons ------------------------------- */

function ToolIcon({ name, iconLib = "svg" }: { name: string; iconLib?: IconLibraryId }) {
  // Try the active library first; fall back to hand-drawn SVG
  const LibIcon = iconLib !== "svg" ? getIconComponent(iconLib, name) : null;
  if (LibIcon) {
    return (
      <LibIcon
        size={20}
        strokeWidth={1.75}
        weight={iconLib === "phosphor" ? "bold" : undefined}
        className="lib-icon"
        aria-hidden
      />
    );
  }

  const p: Record<string, ReactElement> = {
    select: <path d="M5 3l6 14 2-5 5-1-13-8z" />,
    rect: <rect x="3.5" y="5" width="17" height="14" rx="1" />,
    ellipse: <ellipse cx="12" cy="12" rx="9" ry="6.5" />,
    diamond: <path d="M12 3l9 9-9 9-9-9 9-9z" />,
    triangle: <path d="M12 4l9 16H3l9-16z" />,
    trapezoid: <path d="M8 6l-6 12h20L16 6H8z" />,
    rhombus: <path d="M7 5h10l6 7-6 7H7l-6-7 6-7z" />,
    "rhombus-2": <path d="M8 5h9l4 7-4 7H8l-4-7 4-7z" />,
    pentagon: <path d="M12 3.5 21 9l-3.5 10h-11L3 9l9-5.5z" />,
    hexagon: <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9L12 3z" />,
    octagon: <path d="M8.6 3.5H15.4L20.5 8.6v6.8l-5.1 5.1H8.6L3.5 15.4V8.6l5.1-5.1z" />,
    star: <path d="M12 3l2.6 5.4 5.9.8-4.3 4.1 1 5.8L12 16.9l-5.2 2.2 1-5.8L3.5 9.2l5.9-.8L12 3z" />,
    heart: <path d="M12 20s-7-4.6-9-9c-1-2.2.3-5 3-5 1.8 0 3.4 1 4 2.6C10.6 7 12.2 6 14 6c2.7 0 4 2.8 3 5-2 4.4-5 9-5 9z" />,
    cloud: <path d="M7 18a4.5 4.5 0 0 1-.4-9 5.5 5.5 0 0 1 10.6 1.5A4 4 0 0 1 17 18H7z" />,
    "check-box": (
      <>
        <rect x="3.5" y="5" width="17" height="14" rx="1" />
        <path d="M7.5 12.5l3 3 6-6.5" />
      </>
    ),
    "arrow-right": <path d="M4 9h9V5l7 7-7 7v-4H4V9z" />,
    "arrow-left": <path d="M20 9h-9V5l-7 7 7 7v-4h9V9z" />,
    "arrow-up": <path d="M9 4h6v9h4l-7 7-7-7h4V4z" />,
    "arrow-down": <path d="M9 20h6v-9h4l-7-7-7 7h4v9z" />,
    "x-box": (
      <>
        <rect x="3.5" y="5" width="17" height="14" rx="1" />
        <path d="M8 9l8 6m0-6l-8 6" />
      </>
    ),
    arrow: <path d="M4 20 20 4M9 4h11v11" />,
    line: <path d="M4 19 20 5" />,
    text: <path d="M12 4v16M6 6h12M8 20h8" />,
    note: <path d="M4 4h16v14l-4 4H4V4zm12 18l4-4" />,
    frame: (
      <>
        <rect x="4" y="3.5" width="16" height="17" />
        <rect x="7" y="7" width="10" height="10" />
      </>
    ),
    counter: (
      <>
        <circle cx="12" cy="12" r="8" fill="#fff" />
        <text x="12" y="12" textAnchor="middle" dominantBaseline="central" fontSize="11" fontWeight="600">
          1
        </text>
      </>
    ),
    highlight: <path d="M6 13l6-6 5 5-6 6H6v-5zM3 21h8" />,
    blur: <path d="M12 3c4 4 7 6 7 10a7 7 0 1 1-14 0c0-4 3-6 7-10z" />,
    pixelate: (
      <>
        <rect x="3" y="3" width="18" height="18" />
        <path d="M9 3v6h6V3M9 15h6v6M3 9h6M15 9h6M3 15h6M15 15h6" />
      </>
    ),
    redact: <rect x="4" y="4" width="16" height="16" />,
    eraser: <path d="M4 20h16M4.5 15.5l7-7 5 5M11.5 8.5l4-4 4 4" />,
    draw: <path d="M4 20c3-1 4-3 4-6M12 4l8 8-9 9-8-1 1-8 8-8z" />,
    hand: <path d="M9 11V5a1.5 1.5 0 0 1 3 0v5m0-4a1.5 1.5 0 0 1 3 0v4m0-3a1.5 1.5 0 0 1 3 0v6a7 7 0 0 1-7 7h-2a6 6 0 0 1-4.8-2.4l-3-4.2a1.7 1.7 0 0 1 2.8-1.9L7 14" />,
    laser: (
      <>
        <path d="M4 4h16" />
        <path d="M6.5 4l5.5 8 5.5-8M12 12v8" />
      </>
    ),
    crop: (
      <>
        <path d="M8 4H4v4M16 4h4v4M20 16v4h-4M4 16v4h4" />
        <rect x="7" y="7" width="10" height="10" rx="1" />
      </>
    ),
    "fill-none": (
      <>
        <path d="M4 9l8-6 8 6v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V9z" />
        <path d="M8 9h8" />
      </>
    ),
    "fill-semi": (
      <>
        <path d="M4 9l8-6 8 6v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V9z" />
        <path d="M4 15h16M4 12h16" />
      </>
    ),
    "fill-solid": (
      <>
        <path d="M4 9l8-6 8 6v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V9z" fill="currentColor" />
      </>
    ),
    more: (
      <>
        <circle cx="5" cy="12" r="1.6" />
        <circle cx="12" cy="12" r="1.6" />
        <circle cx="19" cy="12" r="1.6" />
      </>
    ),
    duplicate: (
      <>
        <rect x="8" y="8" width="12" height="12" rx="1" />
        <path d="M16 8V4a1 1 0 0 0-1-1H4a1 1 0 0 0-1 1v11a1 1 0 0 0 1 1h4" />
      </>
    ),
    trash: (
      <>
        <path d="M5 7h14M9 7V4h6v3M6 7l1 13h10l1-13M10 11v5m4-5v5" />
      </>
    ),
    group: (
      <>
        <rect x="3" y="5" width="8" height="8" />
        <rect x="13" y="11" width="8" height="8" />
        <path d="M7 13v6h6M17 5h-4" />
      </>
    ),
    ungroup: (
      <>
        <rect x="3" y="5" width="8" height="8" />
        <rect x="13" y="11" width="8" height="8" />
        <path d="M3 21v-6h4M21 3v6h-4" />
      </>
    ),
    align: (
      <>
        <rect x="3" y="5" width="18" height="4" />
        <rect x="3" y="13" width="10" height="6" />
      </>
    ),
    "flip-h": <path d="M12 4v16M8 8l-5 4 5 4V8zm8 0l5 4-5 4V8z" />,
    "flip-v": <path d="M4 12h16M8 8l-4 4 4 4V8zm8 0l4 4-4 4V8z" />,
    order: (
      <>
        <rect x="4" y="4" width="16" height="5" />
        <rect x="7" y="9.5" width="10" height="5" />
        <rect x="10" y="15" width="4" height="5" />
      </>
    ),
    lock: (
      <>
        <rect x="5" y="11" width="14" height="9" rx="1" />
        <path d="M8 11V8a4 4 0 0 1 8 0v3" />
      </>
    ),
  };
  return <svg viewBox="0 0 24 24" className="tool-icon" aria-hidden="true">{p[name] ?? p.select}</svg>;
}



