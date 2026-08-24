export type ToolKind =
  | "select"
  | "rect"
  | "ellipse"
  | "arrow"
  | "line"
  | "text"
  | "counter"
  | "highlight"
  | "blur"
  | "pixelate"
  | "redact";

export type Point = { x: number; y: number };
export type Rect = { x: number; y: number; width: number; height: number };
export type RegionMode = "crop" | "ocr";
export type Stroke = { color: string; width: number };

export type Annotation =
  | {
      id: string;
      kind: "rect" | "ellipse";
      x: number;
      y: number;
      width: number;
      height: number;
      stroke: Stroke;
      fill: string;
    }
  | { id: string; kind: "line"; x1: number; y1: number; x2: number; y2: number; stroke: Stroke }
  | { id: string; kind: "arrow"; x1: number; y1: number; x2: number; y2: number; stroke: Stroke }
  | {
      id: string;
      kind: "text";
      x: number;
      y: number;
      text: string;
      fontSize: number;
      color: string;
      fontFamily: string;
    }
  | { id: string; kind: "counter"; x: number; y: number; n: number; radius: number; color: string }
  | { id: string; kind: "highlight"; x: number; y: number; width: number; height: number; color: string }
  | { id: string; kind: "redact"; x: number; y: number; width: number; height: number }
  | { id: string; kind: "blur" | "pixelate"; x: number; y: number; width: number; height: number; strength: number };

export type EffectKind = "highlight" | "redact" | "blur" | "pixelate";

/** Serialized tldraw document state (the editor store). */
export type TldrawState = { snapshot: import("@tldraw/tlschema").TLStoreSnapshot } | null;

export type CaptureRecord = {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  imageBlob: Blob;
  thumbBlob: Blob;
  image: { width: number; height: number };
  annotations: TldrawState;
};

export type CaptureDoc = Omit<CaptureRecord, "imageBlob" | "thumbBlob">;

export type ToolProps = {
  color: string;
  width: number;
  fontSize: number;
};

export const COLORS = ["#ef4444", "#f97316", "#eab308", "#22c55e", "#3b82f6", "#8b5cf6", "#111827", "#ffffff"];
export const WIDTHS = [2, 4, 6, 10];
export const FONT_SIZES = [14, 18, 24, 32];

export function uid(): string {
  return crypto.randomUUID();
}

export function nextCounter(annotations: Annotation[]): number {
  const max = annotations
    .filter((a) => a.kind === "counter")
    .reduce((m, a) => (a.kind === "counter" ? Math.max(m, a.n) : m), 0);
  return max + 1;
}

export const EFFECT_KINDS: EffectKind[] = ["highlight", "redact", "blur", "pixelate"];

export function isEffect(a: Annotation): a is Extract<Annotation, { kind: EffectKind }> {
  return EFFECT_KINDS.includes(a.kind as EffectKind);
}
