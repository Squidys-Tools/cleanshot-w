import { useCallback, useEffect, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import {
  cancelAreaCapture,
  captureFrameUrl,
  capturePointFromClient,
  completeAreaCapture,
  getActiveCapture,
  nativeErrorMessage,
  type NativeCaptureFrame,
  type SelectionRect,
} from "../lib/nativeCapture";

type Point = { x: number; y: number };

type DragState =
  | { kind: "idle" }
  | { kind: "dragging"; start: Point; current: Point };

type OverlayPhase =
  | { kind: "loading" }
  | { kind: "ready"; frame: NativeCaptureFrame }
  | { kind: "error"; message: string };

function normalizeSelection(start: Point, end: Point): SelectionRect {
  const left = Math.min(start.x, end.x);
  const top = Math.min(start.y, end.y);
  const right = Math.max(start.x, end.x);
  const bottom = Math.max(start.y, end.y);
  return {
    x: Math.round(left),
    y: Math.round(top),
    width: Math.round(right - left),
    height: Math.round(bottom - top),
  };
}

function selectionStyle(selection: SelectionRect, frame: NativeCaptureFrame): CSSProperties {
  return {
    left: `${(selection.x / frame.width) * 100}%`,
    top: `${(selection.y / frame.height) * 100}%`,
    width: `${(selection.width / frame.width) * 100}%`,
    height: `${(selection.height / frame.height) * 100}%`,
  };
}

function CaptureOverlay() {
  const [phase, setPhase] = useState<OverlayPhase>({ kind: "loading" });
  const [drag, setDrag] = useState<DragState>({ kind: "idle" });
  const [pointer, setPointer] = useState<Point | null>(null);
  const viewportRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let mounted = true;
    void getActiveCapture()
      .then((frame) => {
        if (mounted) setPhase({ kind: "ready", frame });
      })
      .catch((error: unknown) => {
        if (mounted) setPhase({ kind: "error", message: nativeErrorMessage(error, "Could not prepare the screen capture.") });
      });
    return () => {
      mounted = false;
    };
  }, []);

  const cancel = useCallback(() => {
    void cancelAreaCapture().catch(() => undefined);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        cancel();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [cancel]);

  const pointFromEvent = useCallback(
    (event: PointerEvent<HTMLDivElement>, frame: NativeCaptureFrame): Point => {
      const viewport = viewportRef.current?.getBoundingClientRect();
      if (!viewport || viewport.width <= 0 || viewport.height <= 0) {
        return { x: 0, y: 0 };
      }
      return capturePointFromClient(event.clientX, event.clientY, viewport, frame);
    },
    [],
  );

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (phase.kind !== "ready" || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const point = pointFromEvent(event, phase.frame);
    setPointer(point);
    setDrag({ kind: "dragging", start: point, current: point });
  };

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (phase.kind !== "ready") return;
    const point = pointFromEvent(event, phase.frame);
    setPointer(point);
    if (drag.kind === "dragging") setDrag({ ...drag, current: point });
  };

  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    if (phase.kind !== "ready" || drag.kind !== "dragging") return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    const selection = normalizeSelection(drag.start, pointFromEvent(event, phase.frame));
    setDrag({ kind: "idle" });
    if (selection.width < 4 || selection.height < 4) return;
    void completeAreaCapture(selection).catch((error: unknown) => {
      setPhase({ kind: "error", message: nativeErrorMessage(error, "Could not complete the screen capture.") });
    });
  };

  if (phase.kind === "loading") {
    return <div className="capture-overlay-loading">Preparing capture...</div>;
  }

  if (phase.kind === "error") {
    return (
      <div className="capture-overlay-error">
        <strong>Capture failed</strong>
        <span>{phase.message}</span>
        <button className="btn" onClick={cancel}>
          Close
        </button>
      </div>
    );
  }

  const selection = drag.kind === "dragging" ? normalizeSelection(drag.start, drag.current) : null;
  const screenStyle: CSSProperties = {
    backgroundImage: `url("${captureFrameUrl(phase.frame)}")`,
  };

  return (
    <div
      ref={viewportRef}
      className="capture-overlay-screen"
      style={screenStyle}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={() => setDrag({ kind: "idle" })}
    >
      <div className="capture-overlay-tint" />
      {selection && selection.width > 0 && selection.height > 0 && (
        <div className="capture-overlay-selection" style={selectionStyle(selection, phase.frame)}>
          <span>
            {selection.width} × {selection.height}
          </span>
        </div>
      )}
      {pointer && (
        <div
          className="capture-overlay-loupe"
          style={{
            left: `${(pointer.x / phase.frame.width) * 100}%`,
            top: `${(pointer.y / phase.frame.height) * 100}%`,
          }}
          aria-hidden="true"
        >
          <div
            className="capture-overlay-loupe-image"
            style={{
              backgroundImage: `url("${captureFrameUrl(phase.frame)}")`,
              backgroundPosition: `${(pointer.x / phase.frame.width) * 100}% ${(pointer.y / phase.frame.height) * 100}%`,
            }}
          />
          <span className="capture-overlay-loupe-crosshair" />
          <span className="capture-overlay-loupe-label">{Math.round(pointer.x)} × {Math.round(pointer.y)}</span>
        </div>
      )}
      <button className="capture-overlay-cancel" onPointerDown={(event) => event.stopPropagation()} onClick={cancel}>
        Cancel
      </button>
    </div>
  );
}

export default CaptureOverlay;
