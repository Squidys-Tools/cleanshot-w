import { useEffect, useRef, type ReactElement } from "react";
import {
  BaseBoxShapeUtil,
  DefaultColorStyle,
  SVGContainer,
  T,
  getColorValue,
  type Editor,
  type SvgExportContext,
  type TLDefaultColorStyle,
  type TLShape,
} from "@tldraw/editor";
import { getBackgroundImageSrc } from "../../lib/tldrawDoc";

/* Register the custom shape props in the global type map so that TLShape<"cs-…">
   and TLBaseBoxShape structurally include them. */
declare module "@tldraw/tlschema" {
  export interface TLGlobalShapePropsMap {
    "cs-counter": { w: number; h: number; n: number; color: TLDefaultColorStyle };
    "cs-blur": { w: number; h: number; strength: number };
    "cs-pixelate": { w: number; h: number; block: number };
    "cs-redact": { w: number; h: number };
  }
}

/* ------------------------------- types ------------------------------- */

export type CounterShape = TLShape<"cs-counter">;

export type BlurShape = TLShape<"cs-blur">;

export type PixelateShape = TLShape<"cs-pixelate">;

export type RedactShape = TLShape<"cs-redact">;

/* ----------------------------- color util ---------------------------- */

function cssColor(editor: Editor, color: TLDefaultColorStyle): string {
  const theme = editor.getCurrentTheme();
  return getColorValue(theme.colors[editor.getColorMode()], color, "solid");
}

/* ------------------------- effect image helper ------------------------ */

const imgCache = new Map<string, Promise<HTMLImageElement>>();

function loadImage(src: string): Promise<HTMLImageElement> {
  let p = imgCache.get(src);
  if (!p) {
    p = new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error("could not load image"));
      img.src = src;
    });
    imgCache.set(src, p);
  }
  return p;
}

function drawEffect(
  canvas: HTMLCanvasElement,
  img: HTMLImageElement,
  sx: number,
  sy: number,
  sw: number,
  sh: number,
  kind: "blur" | "pixelate",
  strength: number,
): void {
  const cw = Math.max(1, Math.round(sw));
  const ch = Math.max(1, Math.round(sh));
  canvas.width = cw;
  canvas.height = ch;
  const ctx = canvas.getContext("2d")!;
  ctx.clearRect(0, 0, cw, ch);
  if (kind === "blur") {
    ctx.save();
    ctx.filter = `blur(${strength}px)`;
    ctx.drawImage(img, sx, sy, sw, sh, 0, 0, cw, ch);
    ctx.restore();
  } else {
    const block = Math.max(2, Math.round(strength));
    const sw2 = Math.max(1, Math.round(sw / block));
    const sh2 = Math.max(1, Math.round(sh / block));
    const small = document.createElement("canvas");
    small.width = sw2;
    small.height = sh2;
    const sctx = small.getContext("2d")!;
    sctx.imageSmoothingEnabled = false;
    sctx.drawImage(img, sx, sy, sw, sh, 0, 0, sw2, sh2);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(small, 0, 0, sw2, sh2, 0, 0, cw, ch);
    ctx.imageSmoothingEnabled = true;
  }
}

async function effectDataUrl(
  editor: Editor,
  x: number,
  y: number,
  w: number,
  h: number,
  kind: "blur" | "pixelate",
  strength: number,
): Promise<string | null> {
  const src = getBackgroundImageSrc(editor);
  if (!src) return null;
  const img = await loadImage(src);
  const canvas = document.createElement("canvas");
  drawEffect(canvas, img, x, y, w, h, kind, strength);
  return canvas.toDataURL("image/png");
}

function EffectCanvas({
  editor,
  x,
  y,
  w,
  h,
  kind,
  strength,
}: {
  editor: Editor;
  x: number;
  y: number;
  w: number;
  h: number;
  kind: "blur" | "pixelate";
  strength: number;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    let alive = true;
    const src = getBackgroundImageSrc(editor);
    if (!src) return;
    loadImage(src)
      .then((img) => {
        if (!alive) return;
        drawEffect(c, img, x, y, w, h, kind, strength);
      })
      .catch(() => {
        /* keep canvas blank */
      });
    return () => {
      alive = false;
    };
  }, [editor, x, y, w, h, kind, strength]);
  return <canvas ref={ref} className="cs-effect-canvas" style={{ width: "100%", height: "100%" }} />;
}

/* ------------------------------ counter ------------------------------ */

export class CounterShapeUtil extends BaseBoxShapeUtil<CounterShape> {
  static override type = "cs-counter" as const;
  static override props = {
    w: T.number,
    h: T.number,
    n: T.number,
    color: DefaultColorStyle,
  };

  override getDefaultProps(): CounterShape["props"] {
    return { w: 48, h: 48, n: 1, color: "red" };
  }

  override isAspectRatioLocked(): boolean {
    return true;
  }

  override canResize(): boolean {
    return true;
  }

  override getIndicatorPath(shape: CounterShape) {
    const r = Math.min(shape.props.w, shape.props.h) / 2;
    const path = new Path2D();
    path.arc(shape.props.w / 2, shape.props.h / 2, r, 0, Math.PI * 2);
    return path;
  }

  override component(shape: CounterShape) {
    const { w, h, n } = shape.props;
    const cx = w / 2;
    const cy = h / 2;
    const r = Math.max(6, Math.min(w, h) / 2 - 2);
    const stroke = Math.max(2, r * 0.18);
    const color = cssColor(this.editor, shape.props.color);
    return (
      <SVGContainer id={shape.id}>
        <circle cx={cx} cy={cy} r={r} fill="#ffffff" stroke={color} strokeWidth={stroke} />
        <text
          x={cx}
          y={cy + 0.5}
          textAnchor="middle"
          dominantBaseline="central"
          fontWeight="700"
          fontSize={r * 1.05}
          fontFamily="system-ui, -apple-system, sans-serif"
          fill={color}
        >
          {n}
        </text>
      </SVGContainer>
    );
  }

  override toSvg(shape: CounterShape) {
    const { w, h, n } = shape.props;
    const cx = w / 2;
    const cy = h / 2;
    const r = Math.max(6, Math.min(w, h) / 2 - 2);
    const stroke = Math.max(2, r * 0.18);
    const color = cssColor(this.editor, shape.props.color);
    return (
      <g>
        <circle cx={cx} cy={cy} r={r} fill="#ffffff" stroke={color} strokeWidth={stroke} />
        <text
          x={cx}
          y={cy + 0.5}
          textAnchor="middle"
          dominantBaseline="central"
          fontWeight="700"
          fontSize={r * 1.05}
          fontFamily="system-ui, -apple-system, sans-serif"
          fill={color}
        >
          {n}
        </text>
      </g>
    );
  }
}

/* -------------------------------- blur -------------------------------- */

export class BlurShapeUtil extends BaseBoxShapeUtil<BlurShape> {
  static override type = "cs-blur" as const;
  static override props = {
    w: T.number,
    h: T.number,
    strength: T.number,
  };

  override getDefaultProps(): BlurShape["props"] {
    return { w: 260, h: 140, strength: 14 };
  }

  override getIndicatorPath(shape: BlurShape) {
    const path = new Path2D();
    path.rect(0, 0, shape.props.w, shape.props.h);
    return path;
  }

  override component(shape: BlurShape) {
    return (
      <EffectCanvas
        editor={this.editor}
        x={shape.x}
        y={shape.y}
        w={shape.props.w}
        h={shape.props.h}
        kind="blur"
        strength={shape.props.strength}
      />
    );
  }

  override async toSvg(shape: BlurShape, ctx: SvgExportContext): Promise<ReactElement | null> {
    void ctx;
    const url = await effectDataUrl(this.editor, shape.x, shape.y, shape.props.w, shape.props.h, "blur", shape.props.strength);
    if (!url) return null;
    return (
      <image x={0} y={0} width={shape.props.w} height={shape.props.h} preserveAspectRatio="none" href={url} />
    );
  }
}

/* ------------------------------ pixelate ------------------------------ */

export class PixelateShapeUtil extends BaseBoxShapeUtil<PixelateShape> {
  static override type = "cs-pixelate" as const;
  static override props = {
    w: T.number,
    h: T.number,
    block: T.number,
  };

  override getDefaultProps(): PixelateShape["props"] {
    return { w: 260, h: 140, block: 10 };
  }

  override getIndicatorPath(shape: PixelateShape) {
    const path = new Path2D();
    path.rect(0, 0, shape.props.w, shape.props.h);
    return path;
  }

  override component(shape: PixelateShape) {
    return (
      <EffectCanvas
        editor={this.editor}
        x={shape.x}
        y={shape.y}
        w={shape.props.w}
        h={shape.props.h}
        kind="pixelate"
        strength={shape.props.block}
      />
    );
  }

  override async toSvg(shape: PixelateShape, ctx: SvgExportContext): Promise<ReactElement | null> {
    void ctx;
    const url = await effectDataUrl(this.editor, shape.x, shape.y, shape.props.w, shape.props.h, "pixelate", shape.props.block);
    if (!url) return null;
    return (
      <image x={0} y={0} width={shape.props.w} height={shape.props.h} preserveAspectRatio="none" href={url} />
    );
  }
}

/* ------------------------------- redact ------------------------------- */

export class RedactShapeUtil extends BaseBoxShapeUtil<RedactShape> {
  static override type = "cs-redact" as const;
  static override props = {
    w: T.number,
    h: T.number,
  };

  override getDefaultProps(): RedactShape["props"] {
    return { w: 260, h: 140 };
  }

  override getIndicatorPath(shape: RedactShape) {
    const path = new Path2D();
    path.rect(0, 0, shape.props.w, shape.props.h);
    return path;
  }

  override component(shape: RedactShape) {
    return <div className="cs-redact-fill" style={{ width: "100%", height: "100%" }} data-testid={shape.id} />;
  }

  override toSvg(shape: RedactShape) {
    return (
      <rect x={0} y={0} width={shape.props.w} height={shape.props.h} fill="#111827" />
    );
  }
}
