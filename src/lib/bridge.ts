import type { CaptureRecord } from "../types";
import { copyImageToClipboard, copyText } from "./export";
import { recognizeText, type OcrResult } from "./ocr";
import { deleteCapture, getCapture, listCaptures, saveCapture } from "./storage";

export type OcrProgress = { status: string; progress: number };

export interface HostBridge {
  captureArea(): Promise<Blob | null>;
  saveCapture(record: CaptureRecord): Promise<void>;
  listCaptures(): Promise<CaptureRecord[]>;
  getCapture(id: string): Promise<CaptureRecord | undefined>;
  deleteCapture(id: string): Promise<void>;
  copyImage(blob: Blob): Promise<void>;
  copyText(text: string): Promise<void>;
  recognize(blob: Blob, onProgress?: (p: OcrProgress) => void): Promise<OcrResult>;
}

export const browserHost: HostBridge = {
  captureArea: async () => null,
  saveCapture,
  listCaptures,
  getCapture,
  deleteCapture,
  copyImage: copyImageToClipboard,
  copyText,
  recognize: recognizeText,
};

export let host: HostBridge = browserHost;

export function setHost(next: HostBridge): void {
  host = next;
}
