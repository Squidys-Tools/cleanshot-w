import { describe, expect, test } from "bun:test";
import { computeFitCamera, MAX_ZOOM, MIN_ZOOM } from "../src/lib/fitCamera";

/* The original inline expression from frameImage(), copied verbatim from
   src/components/editor/TldrawCanvas.tsx as it stood before this change. It is
   the oracle: for inputs a real browser produces, the extracted function must
   agree with it exactly. That is what makes the refactor safe. */
function legacyFrameImage(
  imgW: number,
  imgH: number,
  viewportW: number,
  viewportH: number,
  insetT: number,
  insetB: number,
  sbx: number,
  sby: number,
  bx: number,
  by: number,
  bw: number,
  bh: number,
) {
  const M = 24;
  const availW = Math.max(64, viewportW - 2 * M);
  const availH = Math.max(64, viewportH - insetT - insetB);
  const zoom = Math.min(Math.max(Math.min(availW / bw, availH / bh), 0.05), 8);
  const tx = M + (availW - bw * zoom) / 2;
  const ty = insetT + (availH - bh * zoom) / 2;
  return {
    x: (tx - sbx) / zoom - bx,
    y: (ty - sby) / zoom - by,
    z: zoom,
  };
}

describe("computeFitCamera", () => {
  test("matches the legacy inline expression across a grid of real-world inputs", () => {
    const viewports = [
      { width: 1180, height: 760 },
      { width: 960, height: 620 },
      { width: 1400, height: 900 },
      { width: 3840, height: 2160 },
      { width: 640, height: 480 },
    ];
    const insets = [
      { top: 24, bottom: 24 },
      { top: 68, bottom: 120 },
      { top: 0, bottom: 0 },
      { top: 300, bottom: 300 },
    ];
    const origins = [
      { x: 0, y: 0 },
      { x: -1920, y: -120 },
      { x: 37.5, y: 12.25 },
    ];
    const bounds = [
      { x: 0, y: 0, width: 900, height: 300 },
      { x: 120.5, y: -40.25, width: 3840, height: 2160 },
      { x: 0, y: 0, width: 1, height: 1 },
      { x: 0, y: 0, width: 20000, height: 20000 },
    ];

    let compared = 0;
    for (const viewport of viewports) {
      for (const inset of insets) {
        for (const origin of origins) {
          for (const bound of bounds) {
            const legacy = legacyFrameImage(
              bound.width,
              bound.height,
              viewport.width,
              viewport.height,
              inset.top,
              inset.bottom,
              origin.x,
              origin.y,
              bound.x,
              bound.y,
              bound.width,
              bound.height,
            );
            const next = computeFitCamera({
              viewport,
              insets: inset,
              margin: 24,
              bounds: bound,
              screenBounds: origin,
            });
            expect(next).not.toBeNull();
            expect(next!.x).toBe(legacy.x);
            expect(next!.y).toBe(legacy.y);
            expect(next!.z).toBe(legacy.z);
            compared += 1;
          }
        }
      }
    }
    expect(compared).toBe(viewports.length * insets.length * origins.length * bounds.length);
  });

  test("returns a finite camera and clamps zoom into range", () => {
    const camera = computeFitCamera({
      viewport: { width: 1180, height: 760 },
      insets: { top: 24, bottom: 24 },
      margin: 24,
      bounds: { x: 0, y: 0, width: 900, height: 300 },
      screenBounds: { x: 0, y: 0 },
    });
    expect(camera).not.toBeNull();
    expect(Number.isFinite(camera!.x)).toBe(true);
    expect(Number.isFinite(camera!.y)).toBe(true);
    expect(camera!.z).toBeGreaterThanOrEqual(MIN_ZOOM);
    expect(camera!.z).toBeLessThanOrEqual(MAX_ZOOM);
  });

  test("never emits a non-finite component, whatever it is fed", () => {
    const hostile = [0, -1, NaN, Infinity, -Infinity, null, undefined, "900", {}];
    for (const bad of hostile) {
      const camera = computeFitCamera({
        viewport: { width: bad as number, height: 760 },
        insets: { top: 24, bottom: 24 },
        margin: 24,
        bounds: { x: 0, y: 0, width: 900, height: 300 },
        screenBounds: { x: 0, y: 0 },
      });
      if (camera) {
        expect(Number.isFinite(camera.x)).toBe(true);
        expect(Number.isFinite(camera.y)).toBe(true);
        expect(Number.isFinite(camera.z)).toBe(true);
      }
    }
  });

  test("refuses to guess when the viewport is unmeasurable", () => {
    // A hidden or minimised window can report 0. Returning a camera here is
    // what put the image off-screen in the first place.
    for (const viewport of [
      { width: 0, height: 760 },
      { width: 1180, height: 0 },
      { width: 0, height: 0 },
      { width: NaN, height: NaN },
      { width: Infinity, height: Infinity },
    ]) {
      expect(
        computeFitCamera({
          viewport,
          insets: { top: 24, bottom: 24 },
          margin: 24,
          bounds: { x: 0, y: 0, width: 900, height: 300 },
          screenBounds: { x: 0, y: 0 },
        }),
      ).toBeNull();
    }
  });

  test("falls back to the known image size when the shape has no usable bounds", () => {
    const base = {
      viewport: { width: 1180, height: 760 },
      insets: { top: 24, bottom: 24 },
      margin: 24,
      screenBounds: { x: 0, y: 0 },
      fallbackSize: { width: 900, height: 300 },
    };
    // Shape not yet created / not yet measured.
    const fresh = computeFitCamera({ ...base, bounds: { x: 0, y: 0, width: 0, height: 0 } });
    expect(fresh).not.toBeNull();
    expect(Number.isFinite(fresh!.z)).toBe(true);

    // Garbage bounds also fall back rather than propagating NaN.
    for (const bounds of [
      { x: 0, y: 0, width: NaN, height: NaN },
      { x: 0, y: 0, width: Infinity, height: Infinity },
      { x: NaN, y: NaN, width: 900, height: 300 },
    ]) {
      const camera = computeFitCamera({ ...base, bounds });
      expect(camera).not.toBeNull();
      expect(Number.isFinite(camera!.x)).toBe(true);
      expect(Number.isFinite(camera!.y)).toBe(true);
      expect(Number.isFinite(camera!.z)).toBe(true);
    }
  });

  test("returns null when there is no usable size at all", () => {
    const camera = computeFitCamera({
      viewport: { width: 1180, height: 760 },
      insets: { top: 24, bottom: 24 },
      margin: 24,
      bounds: { x: 0, y: 0, width: 0, height: 0 },
      screenBounds: { x: 0, y: 0 },
    });
    expect(camera).toBeNull();
  });

  test("sanitises a hostile viewport origin instead of poisoning the camera", () => {
    const camera = computeFitCamera({
      viewport: { width: 1180, height: 760 },
      insets: { top: 24, bottom: 24 },
      margin: 24,
      bounds: { x: 0, y: 0, width: 900, height: 300 },
      screenBounds: { x: NaN, y: Infinity },
    });
    expect(camera).not.toBeNull();
    expect(Number.isFinite(camera!.x)).toBe(true);
    expect(Number.isFinite(camera!.y)).toBe(true);
  });

  test("a 1x1 capture is clamped down to MAX_ZOOM, not sent to infinity", () => {
    // availW/1 and availH/1 are both far above MAX_ZOOM, so the fit clamps to
    // the top of the range rather than producing an unbounded zoom.
    const camera = computeFitCamera({
      viewport: { width: 1180, height: 760 },
      insets: { top: 0, bottom: 0 },
      margin: 24,
      bounds: { x: 0, y: 0, width: 1, height: 1 },
      screenBounds: { x: 0, y: 0 },
    });
    expect(camera!.z).toBe(MAX_ZOOM);
  });

  test("an enormous capture is clamped up to MIN_ZOOM, not sent to zero", () => {
    // Without the floor this would zoom to ~0.0076 and show nothing useful.
    const camera = computeFitCamera({
      viewport: { width: 1180, height: 760 },
      insets: { top: 0, bottom: 0 },
      margin: 24,
      bounds: { x: 0, y: 0, width: 100000, height: 100000 },
      screenBounds: { x: 0, y: 0 },
    });
    expect(camera!.z).toBe(MIN_ZOOM);
  });

  test("honours a negative virtual-screen origin", () => {
    // A monitor left of / above the primary display produces a negative origin.
    const camera = computeFitCamera({
      viewport: { width: 1180, height: 760 },
      insets: { top: 24, bottom: 24 },
      margin: 24,
      bounds: { x: 0, y: 0, width: 900, height: 300 },
      screenBounds: { x: -1920, y: -120 },
    });
    expect(camera).not.toBeNull();
    expect(Number.isFinite(camera!.x)).toBe(true);
    // The camera compensates for the negative origin, so it moves right/down.
    const atOrigin = computeFitCamera({
      viewport: { width: 1180, height: 760 },
      insets: { top: 24, bottom: 24 },
      margin: 24,
      bounds: { x: 0, y: 0, width: 900, height: 300 },
      screenBounds: { x: 0, y: 0 },
    });
    expect(camera!.x).toBeGreaterThan(atOrigin!.x);
    expect(camera!.y).toBeGreaterThan(atOrigin!.y);
  });
});
