import { BaseBoxShapeTool, DefaultColorStyle, StateNode, createShapeId, type TLPointerEventInfo } from "@tldraw/editor";
import type { CounterShape } from "./customShapes";

/* ------------------------------- counter ------------------------------ */

class CounterIdle extends StateNode {
  static override id = "idle";

  override onEnter(): void {
    this.editor.setCursor({ type: "cross" });
  }

  override onPointerDown(info: TLPointerEventInfo): void {
    if (info.button !== 0) return;
    const { currentPagePoint } = this.editor.inputs;
    const counters = this.editor.getCurrentPageShapes().filter((s): s is CounterShape => s.type === "cs-counter");
    const n = counters.reduce((max, s) => Math.max(max, s.props.n), 0) + 1;
    const color = this.editor.getStyleForNextShape(DefaultColorStyle) ?? "red";
    const size = 48;
    this.editor.createShape({
      id: createShapeId(),
      type: "cs-counter",
      x: currentPagePoint.x - size / 2,
      y: currentPagePoint.y - size / 2,
      props: { w: size, h: size, n, color },
    });
  }
}

export class CounterTool extends StateNode {
  static override id = "cs-counter";
  static override initial = "idle";
  static override children = () => [CounterIdle];
}

/* ---------------------------- box effect tools ------------------------ */

export class BlurTool extends BaseBoxShapeTool {
  static override id = "cs-blur";
  override shapeType = "cs-blur" as const;
}

export class PixelateTool extends BaseBoxShapeTool {
  static override id = "cs-pixelate";
  override shapeType = "cs-pixelate" as const;
}

export class RedactTool extends BaseBoxShapeTool {
  static override id = "cs-redact";
  override shapeType = "cs-redact" as const;
}
