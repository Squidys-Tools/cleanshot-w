import { describe, expect, test } from "bun:test";
import * as geo from "../src/lib/geometry";
import type { Annotation, Point } from "../src/types";

describe("geometry", () => {
  test("normalizeRect handles drag in any direction", () => {
    const r = geo.normalizeRect({ x: 100, y: 50 }, { x: 40, y: 80 });
    expect(r).toEqual({ x: 40, y: 50, width: 60, height: 30 });
  });

  test("pointInRect with padding", () => {
    expect(geo.pointInRect({ x: 5, y: 5 }, { x: 0, y: 0, width: 10, height: 10 })).toBe(true);
    expect(geo.pointInRect({ x: 15, y: 5 }, { x: 0, y: 0, width: 10, height: 10 })).toBe(false);
    expect(geo.pointInRect({ x: 12, y: 5 }, { x: 0, y: 0, width: 10, height: 10 }, 3)).toBe(true);
  });

  test("pointInEllipse", () => {
    const r = { x: 0, y: 0, width: 100, height: 60 };
    expect(geo.pointInEllipse({ x: 50, y: 30 }, r)).toBe(true);
    expect(geo.pointInEllipse({ x: 50, y: 10 }, r)).toBe(true);
    expect(geo.pointInEllipse({ x: 101, y: 30 }, r)).toBe(false);
    expect(geo.pointInEllipse({ x: 50, y: -5 }, r)).toBe(false);
  });

  test("pointOnSegment tolerance", () => {
    const a: Point = { x: 0, y: 0 };
    const b: Point = { x: 100, y: 0 };
    expect(geo.pointOnSegment({ x: 50, y: 2 }, a, b, 4)).toBe(true);
    expect(geo.pointOnSegment({ x: 50, y: 6 }, a, b, 4)).toBe(false);
    expect(geo.pointOnSegment({ x: 200, y: 0 }, a, b, 4)).toBe(false);
  });

  test("snapAngle snaps to 45-degree steps", () => {
    const a: Point = { x: 0, y: 0 };
    const snapped = geo.snapAngle(a, { x: 100, y: 99 });
    expect(Math.abs(Math.atan2(snapped.y, snapped.x) - Math.PI / 4)).toBeLessThan(0.001);
  });

  test("bbox for line and text", () => {
    const line: Annotation = { id: "l", kind: "line", x1: 0, y1: 0, x2: 100, y2: 50, stroke: { color: "#000", width: 2 } };
    expect(geo.bbox(line)).toEqual({ x: 0, y: 0, width: 100, height: 50 });
    const text: Annotation = { id: "t", kind: "text", x: 10, y: 40, text: "hi", fontSize: 20, color: "#000", fontFamily: "sans-serif" };
    const b = geo.bbox(text);
    expect(b.y).toBe(40 - 20 * 0.8);
    expect(b.width).toBeGreaterThan(20);
  });

  test("hitTest respects kind-specific shape", () => {
    const ellipse: Annotation = { id: "e", kind: "ellipse", x: 0, y: 0, width: 100, height: 60, stroke: { color: "#000", width: 2 }, fill: "transparent" };
    expect(geo.hitTest(ellipse, { x: 50, y: 30 }, 0)).toBe(true);
    expect(geo.hitTest(ellipse, { x: 50, y: -5 }, 0)).toBe(false);
  });

  test("hitTestTop returns topmost (last drawn)", () => {
    const r1: Annotation = { id: "r1", kind: "rect", x: 0, y: 0, width: 100, height: 100, stroke: { color: "#000", width: 2 }, fill: "transparent" };
    const r2: Annotation = { id: "r2", kind: "rect", x: 0, y: 0, width: 100, height: 100, stroke: { color: "#f00", width: 2 }, fill: "transparent" };
    expect(geo.hitTestTop([r1, r2], { x: 50, y: 50 }, 0)?.id).toBe("r2");
    expect(geo.hitTestTop([r2, r1], { x: 50, y: 50 }, 0)?.id).toBe("r1");
  });

  test("moveAnnotation moves all point types", () => {
    const arrow: Annotation = { id: "a", kind: "arrow", x1: 1, y1: 2, x2: 10, y2: 20, stroke: { color: "#000", width: 2 } };
    const moved = geo.moveAnnotation(arrow, 5, -2);
    expect(moved.x1).toBe(6);
    expect(moved.y2).toBe(18);
  });

  test("resizeRect enforces minimum and anchors opposite corner", () => {
    const r = { x: 0, y: 0, width: 100, height: 100 };
    const resized = geo.resizeRect(r, "se", { x: 40, y: 30 });
    expect(resized).toEqual({ x: 0, y: 0, width: 40, height: 30 });
    const tooSmall = geo.resizeRect(r, "se", { x: 3, y: 3 });
    expect(tooSmall.width).toBe(8);
    expect(tooSmall.height).toBe(8);
    const nw = geo.resizeRect(r, "nw", { x: 20, y: 10 });
    expect(nw.x).toBe(20);
    expect(nw.y).toBe(10);
    expect(nw.width).toBe(80);
    expect(nw.height).toBe(90);
  });
});
