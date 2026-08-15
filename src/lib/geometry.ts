import type { Annotation, Point, Rect } from "../types";

export function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

export function dist(a: Point, b: Point): number {
  return Math.hypot(b.x - a.x, b.y - a.y);
}

export function normalizeRect(a: Point, b: Point): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  return { x, y, width: Math.abs(b.x - a.x), height: Math.abs(b.y - a.y) };
}

export function snapAngle(a: Point, b: Point): Point {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const angle = (Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * Math.PI) / 4;
  const len = Math.hypot(dx, dy);
  return { x: a.x + Math.cos(angle) * len, y: a.y + Math.sin(angle) * len };
}

export function pointInRect(p: Point, r: Rect, pad = 0): boolean {
  return p.x >= r.x - pad && p.x <= r.x + r.width + pad && p.y >= r.y - pad && p.y <= r.y + r.height + pad;
}

export function pointInEllipse(p: Point, r: Rect, pad = 0): boolean {
  const rx = (r.width + pad * 2) / 2;
  const ry = (r.height + pad * 2) / 2;
  if (rx <= 0 || ry <= 0) return false;
  const cx = r.x + r.width / 2;
  const cy = r.y + r.height / 2;
  return ((p.x - cx) * (p.x - cx)) / (rx * rx) + ((p.y - cy) * (p.y - cy)) / (ry * ry) <= 1;
}

export function pointOnSegment(p: Point, a: Point, b: Point, tol: number): boolean {
  const d = dist(a, b);
  if (d === 0) return dist(p, a) <= tol;
  const t = clamp(((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / (d * d), 0, 1);
  const proj = { x: a.x + t * (b.x - a.x), y: a.y + t * (b.y - a.y) };
  return dist(p, proj) <= tol;
}

export function bbox(ann: Annotation): Rect {
  switch (ann.kind) {
    case "rect":
    case "ellipse":
    case "highlight":
    case "redact":
    case "blur":
    case "pixelate":
      return { x: ann.x, y: ann.y, width: ann.width, height: ann.height };
    case "line":
    case "arrow":
      return normalizeRect({ x: ann.x1, y: ann.y1 }, { x: ann.x2, y: ann.y2 });
    case "counter": {
      const r = ann.radius;
      return { x: ann.x - r, y: ann.y - r, width: r * 2, height: r * 2 };
    }
    case "text": {
      const w = estimateTextWidth(ann.text, ann.fontSize);
      return { x: ann.x, y: ann.y - ann.fontSize * 0.8, width: w, height: ann.fontSize * 1.3 };
    }
  }
}

export function estimateTextWidth(text: string, fontSize: number): number {
  return Math.max(fontSize, text.length * fontSize * 0.62);
}

export function hitTest(ann: Annotation, p: Point, pad: number): boolean {
  switch (ann.kind) {
    case "rect":
    case "highlight":
    case "redact":
    case "blur":
    case "pixelate":
      return pointInRect(p, { x: ann.x, y: ann.y, width: ann.width, height: ann.height }, pad);
    case "ellipse":
      return pointInEllipse(p, { x: ann.x, y: ann.y, width: ann.width, height: ann.height }, pad);
    case "line":
    case "arrow":
      return pointOnSegment(p, { x: ann.x1, y: ann.y1 }, { x: ann.x2, y: ann.y2 }, pad);
    case "counter": {
      const r = ann.radius;
      return pointInEllipse(p, { x: ann.x - r, y: ann.y - r, width: r * 2, height: r * 2 }, pad);
    }
    case "text":
      return pointInRect(p, bbox(ann), pad);
  }
}

export function hitTestTop(annotations: Annotation[], p: Point, pad = 6): Annotation | null {
  for (let i = annotations.length - 1; i >= 0; i--) {
    if (hitTest(annotations[i], p, pad)) return annotations[i];
  }
  return null;
}

export function moveAnnotation(ann: Annotation, dx: number, dy: number): Annotation {
  switch (ann.kind) {
    case "rect":
    case "ellipse":
    case "highlight":
    case "redact":
    case "blur":
    case "pixelate":
      return { ...ann, x: ann.x + dx, y: ann.y + dy };
    case "line":
    case "arrow":
      return { ...ann, x1: ann.x1 + dx, y1: ann.y1 + dy, x2: ann.x2 + dx, y2: ann.y2 + dy };
    case "counter":
      return { ...ann, x: ann.x + dx, y: ann.y + dy };
    case "text":
      return { ...ann, x: ann.x + dx, y: ann.y + dy };
  }
}

export type HandleId = "nw" | "ne" | "se" | "sw";

export function handlePoint(r: Rect, id: HandleId): Point {
  switch (id) {
    case "nw":
      return { x: r.x, y: r.y };
    case "ne":
      return { x: r.x + r.width, y: r.y };
    case "se":
      return { x: r.x + r.width, y: r.y + r.height };
    case "sw":
      return { x: r.x, y: r.y + r.height };
  }
}

export function resizeRect(r: Rect, id: HandleId, p: Point, min = 8): Rect {
  let { x, y, width, height } = r;
  const right = x + width;
  const bottom = y + height;
  switch (id) {
    case "nw":
      x = Math.min(p.x, right - min);
      y = Math.min(p.y, bottom - min);
      break;
    case "ne":
      x = x;
      y = Math.min(p.y, bottom - min);
      break;
    case "se":
      x = x;
      y = y;
      break;
    case "sw":
      x = Math.min(p.x, right - min);
      y = y;
      break;
  }
  switch (id) {
    case "nw":
      width = right - x;
      height = bottom - y;
      break;
    case "ne":
      width = Math.max(min, p.x - x);
      height = bottom - y;
      break;
    case "se":
      width = Math.max(min, p.x - x);
      height = Math.max(min, p.y - y);
      break;
    case "sw":
      width = right - x;
      height = Math.max(min, p.y - y);
      break;
  }
  return { x, y, width, height };
}
