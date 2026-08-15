import type { CaptureDoc } from "../types";
import { loadImage } from "./storage";

async function imageElement(record: CaptureDoc & { imageBlob: Blob }): Promise<HTMLImageElement> {
  const dims = await loadImage(record.imageBlob);
  const url = URL.createObjectURL(record.imageBlob);
  const img = new Image();
  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error("image load failed"));
    img.src = url;
  });
  void dims;
  return img;
}

/**
 * Fallback flatten: re-encode the raw screenshot. Used when the tldraw editor
 * is not mounted; when it is, markup is baked in by the editor exporter.
 */
export async function flattenToBlob(
  record: CaptureDoc & { imageBlob: Blob },
): Promise<{ blob: Blob; width: number; height: number }> {
  const img = await imageElement(record);
  const canvas = document.createElement("canvas");
  canvas.width = img.naturalWidth;
  canvas.height = img.naturalHeight;
  canvas.getContext("2d")!.drawImage(img, 0, 0);
  const blob = await new Promise<Blob>((resolve) => canvas.toBlob((b) => resolve(b ?? new Blob()), "image/png"));
  return { blob, width: canvas.width, height: canvas.height };
}

export async function copyImageToClipboard(blob: Blob): Promise<void> {
  await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
}

export async function copyText(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}

export function downloadBlob(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function sanitizeFileName(name: string): string {
  return name.replace(/[^\w\- .]+/g, "_").trim() || "capture";
}
