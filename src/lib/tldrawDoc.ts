import { AssetRecordType, createShapeId, type Editor, type TLAsset } from "@tldraw/editor";
import type { TldrawState } from "../types";

/** Custom tool ids registered with tldraw. */
export const TOOL_IDS = {
  select: "select",
  hand: "hand",
  eraser: "eraser",
  draw: "draw",
  rectangle: "rectangle",
  ellipse: "ellipse",
  arrow: "arrow",
  line: "line",
  text: "text",
  highlight: "highlight",
  counter: "cs-counter",
  blur: "cs-blur",
  pixelate: "cs-pixelate",
  redact: "cs-redact",
} as const;

/** Custom shape types registered with tldraw. */
export const SHAPE_TYPES = {
  counter: "cs-counter",
  blur: "cs-blur",
  pixelate: "cs-pixelate",
  redact: "cs-redact",
} as const;

export type ExportResult = { blob: Blob; width: number; height: number };
export type Exporter = () => Promise<ExportResult>;

export function serializeTldraw(editor: Editor): TldrawState {
  return { snapshot: editor.getSnapshot().document };
}

export function restoreTldraw(editor: Editor, state: TldrawState): void {
  if (!state) return;
  try {
    editor.loadSnapshot(state.snapshot);
  } catch (err) {
    console.error("Could not restore tldraw document", err);
  }
}

/** Number of top-level markup shapes in a saved doc (excludes the locked screenshot). */
export function countMarkups(state: TldrawState): number {
  if (!state) return 0;
  const store = state.snapshot.store;
  const pageIds = new Set(
    Object.values(store)
      .filter((r) => r.typeName === "page")
      .map((r) => r.id as string),
  );
  return Object.values(store).filter(
    (r) => r.typeName === "shape" && r.type !== "image" && !!r.parentId && pageIds.has(r.parentId as string),
  ).length;
}

/** The image asset holding the screenshot, or null. */
export function getBackgroundImageAsset(editor: Editor): TLAsset | null {
  const records = editor.store.allRecords();
  return records.find((r): r is TLAsset => r.typeName === "asset" && r.type === "image") ?? null;
}

export function getBackgroundImageSrc(editor: Editor): string | null {
  return getBackgroundImageAsset(editor)?.props.src ?? null;
}

/**
 * Add the screenshot as a locked image shape at the origin. Idempotent so it is
 * safe to run twice (React StrictMode / re-mount), including when two async
 * calls race each other.
 */
export async function ensureBackgroundImage(editor: Editor, imageUrl: string, w: number, h: number): Promise<void> {
  const existing = getBackgroundImageAsset(editor);
  const hasShape = editor.getCurrentPageShapes().some((s) => s.type === "image");
  if (existing && hasShape) return;

  const src = existing && existing.type === "image" ? existing.props.src : await dataUrlFromUrl(imageUrl);

  if (!getBackgroundImageAsset(editor)) {
    const assetId = AssetRecordType.createId("image");
    const asset = AssetRecordType.create({
      id: assetId,
      type: "image",
      props: { name: "capture.png", src, w, h, mimeType: "image/png", isAnimated: false },
      meta: {},
    });
    editor.store.put([asset]);
  }

  if (!editor.getCurrentPageShapes().some((s) => s.type === "image")) {
    const asset = getBackgroundImageAsset(editor)!;
    const props = asset.type === "image" ? asset.props : { w, h };
    editor.createShape({
      id: createShapeId(),
      type: "image",
      x: 0,
      y: 0,
      isLocked: true,
      props: { w: props.w, h: props.h, assetId: asset.id },
    });
  }
}

export async function dataUrlFromUrl(url: string): Promise<string> {
  if (url.startsWith("data:")) return url;
  const res = await fetch(url);
  const blob = await res.blob();
  return await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(new Error("could not read image"));
    reader.readAsDataURL(blob);
  });
}
