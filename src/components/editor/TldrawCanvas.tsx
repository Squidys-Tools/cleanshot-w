import { useCallback, useEffect, useRef, useState, type ReactElement, type RefObject } from "react";
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
import { ensureBackgroundImage, serializeTldraw, type Exporter } from "../../lib/tldrawDoc";
import type { TldrawState } from "../../types";

const CUSTOM_SHAPE_UTILS = [CounterShapeUtil, BlurShapeUtil, PixelateShapeUtil, RedactShapeUtil];
const CUSTOM_TOOLS = [CounterTool, BlurTool, PixelateTool, RedactTool];

const SWATCH_COLORS = ["red", "orange", "yellow", "green", "blue", "violet", "black", "grey"] as const;

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
  title: string;
  initialDoc: TldrawState;
  onChange: (doc: TldrawState) => void;
  exportRef: { current: Exporter | null };
};

export default function TldrawCanvas({ imageUrl, imgW, imgH, title, initialDoc, onChange, exportRef }: TldrawCanvasProps) {
  const [editor, setEditor] = useState<Editor | null>(null);
  const initialRef = useRef(initialDoc);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const saveTimer = useRef<number | null>(null);

  const scheduleSave = useCallback(() => {
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      onChangeRef.current(serializeTldraw(editor!));
    }, 350);
  }, [editor]);

  const onMount = useCallback(
    (ed: Editor) => {
      ed.user.updateUserPreferences({ colorScheme: "dark", isSnapMode: false });
      ed.updateInstanceState({ isGridMode: false });
      ed.setStyleForNextShapes(DefaultColorStyle, "red");
      ed.setStyleForNextShapes(DefaultSizeStyle, "m");
      ed.setStyleForNextShapes(DefaultDashStyle, "solid");
      ed.setStyleForNextShapes(DefaultFillStyle, "none");
      ed.setStyleForNextShapes(DefaultFontStyle, "sans");
      setEditor(ed);
      void ensureBackgroundImage(ed, imageUrl, imgW, imgH).then(() => {
        ed.zoomToBounds(new Box(0, 0, imgW, imgH), { inset: 48, animation: { duration: 200 } });
      });
    },
    [imageUrl, imgW, imgH],
  );

  useEffect(() => {
    if (!editor) return;
    restoreOnce(editor, initialRef.current);
    editor.setCurrentTool("select");
    editor.zoomToBounds(new Box(0, 0, imgW, imgH), { inset: 48 });
    const unsub = editor.store.listen(() => scheduleSave());
    return () => unsub();
  }, [editor, imgW, imgH, scheduleSave]);

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
    exportRef.current = exporter;
    return () => {
      exportRef.current = null;
    };
  }, [editor, imgW, imgH, exportRef]);

  useEffect(() => {
    if (!editor) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (editor.getEditingShapeId()) return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;

      const mod = e.ctrlKey || e.metaKey;
      if (mod || e.altKey) return;

      const TOOL_KEYS: Record<string, string> = {
        c: "cs-counter",
        m: "cs-blur",
        g: "cs-pixelate",
        q: "cs-redact",
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
    <div className="cs-tldraw">
      <Tldraw hideUi shapeUtils={CUSTOM_SHAPE_UTILS} tools={CUSTOM_TOOLS} onMount={onMount} options={{ maxShapesPerPage: 2000 }}>
        <div className="cs-ui">
          <Toolbar />
          <SelectionBar />
          <ZoomControls />
          <StatusBar title={title} imgW={imgW} imgH={imgH} />
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

function runAction(editor: Editor, fn: () => void): void {
  editor.run(() => {
    editor.markHistoryStoppingPoint();
    fn();
  });
}

function editableShapeIds(editor: Editor): TLShapeId[] {
  return editor.getSelectedShapeIds().filter((id) => editor.getShape(id)?.type !== "image");
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
  { id: "cs-blur", label: "Blur", icon: "blur", kbd: "M" },
  { id: "cs-pixelate", label: "Mosaic", icon: "pixelate", kbd: "G" },
  { id: "cs-redact", label: "Redact", icon: "redact", kbd: "Q" },
  { id: "laser", label: "Laser", icon: "laser" },
];

function Toolbar() {
  const editor = useEditor();
  const { toolId, geo, color, size, fill, dash, canUndo, canRedo } = useValue(
    "cs-toolbar",
    () => ({
      toolId: editor.getCurrentToolId(),
      geo: editor.getStyleForNextShape(GeoShapeGeoStyle),
      color: editor.getStyleForNextShape(DefaultColorStyle),
      size: editor.getStyleForNextShape(DefaultSizeStyle),
      fill: editor.getStyleForNextShape(DefaultFillStyle),
      dash: editor.getStyleForNextShape(DefaultDashStyle),
      canUndo: editor.canUndo(),
      canRedo: editor.canRedo(),
    }),
    [editor],
  );
  const theme = editor.getCurrentTheme();
  const palette = theme.colors[editor.getColorMode()];

  const geoPopover = usePopover();
  const stylePopover = usePopover();

  const setTool = (id: string) => {
    if (id === "rectangle" || id === "ellipse") {
      editor.run(() => {
        editor.setStyleForNextShapes(GeoShapeGeoStyle, id as "rectangle" | "ellipse");
        editor.setCurrentTool("geo");
      });
      return;
    }
    editor.setCurrentTool(id);
  };

  const activeGeo = GEO_SHAPES.find((g) => g.id === geo) ?? GEO_SHAPES[0];

  return (
    <div className="cs-toolbar">
      <div className="tool-group">
        {TOOLS.map((t) => (
          <button
            key={t.id}
            className={`tool-btn ${toolId === t.id ? "active" : ""}`}
            title={`${t.label}${t.kbd ? ` (${t.kbd})` : ""}`}
            onClick={() => setTool(t.id)}
          >
            <ToolIcon name={t.icon} />
          </button>
        ))}
      </div>

      <div className="divider" />

      <div className="prop-group">
        <div className="popover-wrap" ref={geoPopover.ref}>
          <button
            className={`tool-btn ${toolId === "geo" ? "active" : ""}`}
            title={`Shape: ${activeGeo.label}`}
            onClick={geoPopover.toggle}
          >
            <ToolIcon name={activeGeo.icon} />
          </button>
          {geoPopover.open && (
            <div className="popover geo-popover">
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
                    geoPopover.close();
                  }}
                >
                  <ToolIcon name={g.icon} />
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="divider" />

      <div className="prop-group">
        <div className="swatches">
          {SWATCH_COLORS.map((c) => (
            <button
              key={c}
              className={`swatch ${color === c ? "active" : ""}`}
              style={{ background: getColorValue(palette, c, "solid") }}
              title={c}
              onClick={() => editor.setStyleForNextShapes(DefaultColorStyle, c)}
            />
          ))}
        </div>
      </div>

      <div className="divider" />

      <div className="prop-group">
        <div className="sizes">
          {SIZES.map((s) => (
            <button
              key={s.id}
              className={`size-btn ${size === s.id ? "active" : ""}`}
              title={`Size ${s.label}`}
              onClick={() => editor.setStyleForNextShapes(DefaultSizeStyle, s.id)}
            >
              {s.label}
            </button>
          ))}
        </div>
      </div>

      <div className="divider" />

      <div className="prop-group">
        <div className="segmented" role="group" aria-label="Fill">
          {FILLS.map((f) => (
            <button
              key={f.id}
              className={`seg-btn ${fill === f.id ? "active" : ""}`}
              title={f.label}
              onClick={() => editor.setStyleForNextShapes(DefaultFillStyle, f.id)}
            >
              <ToolIcon name={f.icon} />
            </button>
          ))}
        </div>
      </div>

      <div className="divider" />

      <div className="prop-group">
        <div className="segmented" role="group" aria-label="Dash">
          {DASHES.map((d) => (
            <button
              key={d.id}
              className={`seg-btn ${dash === d.id ? "active" : ""}`}
              title={d.label}
              onClick={() => editor.setStyleForNextShapes(DefaultDashStyle, d.id)}
            >
              <span className={`dash-mark dash-${d.id}`} />
            </button>
          ))}
        </div>
      </div>

      <div className="divider" />

      <div className="prop-group">
        <div className="popover-wrap" ref={stylePopover.ref}>
          <button className="tool-btn" title="More styles" onClick={stylePopover.toggle}>
            <ToolIcon name="more" />
          </button>
          {stylePopover.open && <StylePopover editor={editor} onClose={stylePopover.close} />}
        </div>
      </div>

      <div className="divider" />

      <div className="prop-group">
        <button className="icon-btn" title="Undo (Ctrl+Z)" disabled={!canUndo} onClick={() => editor.undo()}>
          ↩
        </button>
        <button className="icon-btn" title="Redo (Ctrl+Y)" disabled={!canRedo} onClick={() => editor.redo()}>
          ↪
        </button>
      </div>
    </div>
  );
}

function StylePopover({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const { font, opacity, ahStart, ahEnd, ahKind, snap, grid, dark } = useValue(
    "cs-styles",
    () => ({
      font: editor.getStyleForNextShape(DefaultFontStyle),
      opacity: editor.getSharedOpacity(),
      ahStart: editor.getStyleForNextShape(ArrowShapeArrowheadStartStyle),
      ahEnd: editor.getStyleForNextShape(ArrowShapeArrowheadEndStyle),
      ahKind: editor.getStyleForNextShape(ArrowShapeKindStyle),
      snap: editor.user.getIsSnapMode(),
      grid: editor.getInstanceState().isGridMode,
      dark: editor.user.getUserPreferences().colorScheme === "dark",
    }),
    [editor],
  );

  return (
    <div className="popover style-popover" onClick={(e) => e.stopPropagation()}>
      <div className="popover-section">
        <div className="popover-label">Font</div>
        <div className="segmented">
          {FONTS.map((f) => (
            <button
              key={f.id}
              className={`seg-btn ${font === f.id ? "active" : ""}`}
              title={f.label}
              onClick={() => editor.setStyleForNextShapes(DefaultFontStyle, f.id)}
            >
              <span className={`font-sample font-${f.id}`}>Aa</span>
            </button>
          ))}
        </div>
      </div>

      <div className="popover-section">
        <div className="popover-label">Opacity</div>
        <div className="segmented">
          {OPACITIES.map((o) => (
            <button
              key={o}
              className={`seg-btn ${opacity !== undefined && opacity.type !== "mixed" && Math.abs(opacity.value - o) < 0.001 ? "active" : ""}`}
              title={`${Math.round(o * 100)}%`}
              onClick={() => {
                editor.setOpacityForSelectedShapes(o);
                editor.setOpacityForNextShapes(o);
              }}
            >
              {Math.round(o * 100)}
            </button>
          ))}
        </div>
      </div>

      <div className="popover-section">
        <div className="popover-label">Arrowheads</div>
        <div className="popover-row">
          <select value={ahStart} onChange={(e) => editor.setStyleForNextShapes(ArrowShapeArrowheadStartStyle, e.target.value as never)}>
            {ARROWHEADS.map((a) => (
              <option key={a.id} value={a.id}>
                Start · {a.label}
              </option>
            ))}
          </select>
          <select value={ahEnd} onChange={(e) => editor.setStyleForNextShapes(ArrowShapeArrowheadEndStyle, e.target.value as never)}>
            {ARROWHEADS.map((a) => (
              <option key={a.id} value={a.id}>
                End · {a.label}
              </option>
            ))}
          </select>
          <select value={ahKind} onChange={(e) => editor.setStyleForNextShapes(ArrowShapeKindStyle, e.target.value as never)}>
            {ARROW_KINDS.map((k) => (
              <option key={k.id} value={k.id}>
                {k.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="popover-section">
        <div className="popover-row">
          <label className="cs-toggle">
            <input type="checkbox" checked={snap} onChange={(e) => editor.user.updateUserPreferences({ isSnapMode: e.target.checked })} />
            Snap
          </label>
          <label className="cs-toggle">
            <input type="checkbox" checked={grid} onChange={(e) => editor.updateInstanceState({ isGridMode: e.target.checked })} />
            Grid
          </label>
          <label className="cs-toggle">
            <input
              type="checkbox"
              checked={dark}
              onChange={(e) => editor.user.updateUserPreferences({ colorScheme: e.target.checked ? "dark" : "light" })}
            />
            Dark
          </label>
        </div>
      </div>
      <button className="popover-close" onClick={onClose}>
        Close
      </button>
    </div>
  );
}

/* -------------------------- Selection bar ---------------------------- */

function SelectionBar() {
  const editor = useEditor();
  const alignPopover = usePopover();
  const orderPopover = usePopover();
  const { hasSel, canUngroup } = useValue(
    "cs-selection",
    () => {
      const ids = editor.getSelectedShapeIds().filter((id) => editor.getShape(id)?.type !== "image");
      return { hasSel: ids.length > 0, canUngroup: ids.some((id) => editor.getShape(id)?.type === "group") };
    },
    [editor],
  );
  if (!hasSel) return null;

  const ids = () => editableShapeIds(editor);

  return (
    <div className="cs-selection">
      <div className="sel-group">
        <button className="icon-btn" title="Duplicate (Ctrl+D)" onClick={() => runAction(editor, () => editor.duplicateShapes(ids()))}>
          <ToolIcon name="duplicate" />
        </button>
        <button className="icon-btn" title="Delete" onClick={() => runAction(editor, () => editor.deleteShapes(ids()))}>
          <ToolIcon name="trash" />
        </button>
      </div>
      <div className="divider" />
      <div className="sel-group">
        <button className="icon-btn" title="Group (Ctrl+G)" onClick={() => runAction(editor, () => editor.groupShapes(ids()))}>
          <ToolIcon name="group" />
        </button>
        <button
          className="icon-btn"
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
          <button className="icon-btn" title="Align & distribute" onClick={alignPopover.toggle}>
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

function ZoomControls() {
  const editor = useEditor();
  const zoom = useValue("cs-zoom", () => editor.getCamera().z, [editor]);
  return (
    <div className="cs-zoom">
      <button className="icon-btn" title="Zoom out" onClick={() => editor.zoomOut()}>
        −
      </button>
      <span className="zoom-label">{Math.round(zoom * 100)}%</span>
      <button className="icon-btn" title="Zoom in" onClick={() => editor.zoomIn()}>
        +
      </button>
      <button className="icon-btn" title="Fit to window" onClick={() => editor.zoomToFit()}>
        Fit
      </button>
    </div>
  );
}

/* ------------------------------ StatusBar ---------------------------- */

function StatusBar({ title, imgW, imgH }: { title: string; imgW: number; imgH: number }) {
  const editor = useEditor();
  const count = useValue(
    "cs-count",
    () => editor.getCurrentPageShapes().filter((s) => s.type !== "image" && s.parentId === editor.getCurrentPageId()).length,
    [editor],
  );
  return (
    <div className="cs-statusbar">
      <span className="cs-title">{title}</span>
      <span>
        {imgW} × {imgH} · {count} markup{count === 1 ? "" : "s"}
      </span>
    </div>
  );
}

/* ------------------------------- Icons ------------------------------- */

function ToolIcon({ name }: { name: string }) {
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
