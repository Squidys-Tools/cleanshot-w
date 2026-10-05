/* Pure camera math for "fit this capture in the window, clear of the chrome".
 *
 * This exists as a separate module for two reasons:
 *
 *  1. It is testable. frameImage() in the editor canvas reaches into the DOM
 *     and into a live tldraw Editor, so it had no unit coverage at all.
 *
 *  2. It is total. Every input is sanitised before it is used, so this can
 *     never hand a NaN or Infinity to tldraw. That matters more than it looks:
 *     tldraw silently coerced a non-finite camera in 5.4.x, but 5.5.0 throws
 *     on one instead, which turns a wrong-looking canvas into a hard crash of
 *     the editor subtree.
 *
 * For every combination of finite, positive inputs this returns exactly what
 * the original inline expression returned. The only difference is that hostile
 * input now yields null (caller skips) instead of a corrupt camera.
 */

/** Smallest usable drawing area, in CSS px. Below this the fit is meaningless. */
const MIN_AVAILABLE = 64;

/** Zoom bounds. Matches the historical inline clamp. */
export const MIN_ZOOM = 0.05;
export const MAX_ZOOM = 8;

export interface Size {
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface Camera {
  x: number;
  y: number;
  z: number;
}

export interface FitCameraInput {
  /** The drawing surface, normally window.innerWidth/innerHeight. */
  viewport: Size;
  /** Chrome reserved above and below the image, in CSS px. */
  insets: { top: number; bottom: number };
  /** Breathing room between the image and any UI, in CSS px. */
  margin: number;
  /** Where the image shape actually sits on the page. */
  bounds: Point & Size;
  /** Top-left of the viewport in screen coords, from the editor. */
  screenBounds: Point;
  /** Fallback size when the shape has no usable measured bounds yet. */
  fallbackSize?: Size;
}

/** True for a real, finite, strictly positive measurement. */
function isPositiveFinite(n: unknown): n is number {
  return typeof n === "number" && Number.isFinite(n) && n > 0;
}

/** True for any real, finite number (zero and negatives allowed). */
function isFiniteNumber(n: unknown): n is number {
  return typeof n === "number" && Number.isFinite(n);
}

/**
 * Solve for the camera that centres `bounds` in the space left over between
 * the chrome insets, at the zoom that fits it.
 *
 * tldraw maps page points to screen points as
 * `screen = (page + camera) * zoom + screenBounds.xy`, so
 * `camera = (target - screenBounds.xy) / zoom - page`.
 *
 * Returns null when the inputs cannot describe a usable camera. Callers must
 * treat null as "leave the camera alone" rather than substituting a default;
 * guessing a camera is what produced the original blank-canvas report.
 */
export function computeFitCamera(input: FitCameraInput): Camera | null {
  const { viewport, insets, margin, bounds, screenBounds, fallbackSize } = input;

  // The viewport must be measurable. A hidden, minimised or not-yet-laid-out
  // window can report 0, and 0 would collapse the fit to nothing.
  if (!isPositiveFinite(viewport?.width) || !isPositiveFinite(viewport?.height)) return null;

  const m = isFiniteNumber(margin) ? margin : 0;
  const insetTop = isFiniteNumber(insets?.top) ? insets.top : 0;
  const insetB = isFiniteNumber(insets?.bottom) ? insets.bottom : 0;

  const availW = Math.max(MIN_AVAILABLE, viewport.width - 2 * m);
  const availH = Math.max(MIN_AVAILABLE, viewport.height - insetTop - insetB);

  // Prefer the shape's measured page bounds; fall back to the known image size
  // when the shape does not exist yet or has not been measured.
  let bw: number | undefined = bounds?.width;
  let bh: number | undefined = bounds?.height;
  if (!isPositiveFinite(bw) || !isPositiveFinite(bh)) {
    bw = fallbackSize?.width;
    bh = fallbackSize?.height;
  }
  if (!isPositiveFinite(bw) || !isPositiveFinite(bh)) return null;

  const zoom = Math.min(Math.max(Math.min(availW / bw, availH / bh), MIN_ZOOM), MAX_ZOOM);
  if (!isFiniteNumber(zoom) || zoom <= 0) return null;

  // Desired on-screen top-left of the image, in window coordinates.
  const tx = m + (availW - bw * zoom) / 2;
  const ty = insetTop + (availH - bh * zoom) / 2;

  // A non-finite viewport origin would poison the camera even though the zoom
  // is sound, so require both axes to be real numbers.
  const sbx = isFiniteNumber(screenBounds?.x) ? screenBounds.x : 0;
  const sby = isFiniteNumber(screenBounds?.y) ? screenBounds.y : 0;
  const pageX = isFiniteNumber(bounds?.x) ? bounds.x : 0;
  const pageY = isFiniteNumber(bounds?.y) ? bounds.y : 0;

  const camera: Camera = {
    x: (tx - sbx) / zoom - pageX,
    y: (ty - sby) / zoom - pageY,
    z: zoom,
  };

  // Final gate. Nothing non-finite may leave this function.
  if (!isFiniteNumber(camera.x) || !isFiniteNumber(camera.y) || !isFiniteNumber(camera.z)) return null;
  return camera;
}
