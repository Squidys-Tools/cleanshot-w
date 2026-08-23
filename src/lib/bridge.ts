import type { CaptureRecord, TldrawState } from "../types";
import { copyImageToClipboard, copyText, downloadBlob } from "./export";
import { recognizeText, type OcrResult } from "./ocr";
import { deleteCapture, getCapture, listCaptures, saveCapture, updateCaptureAnnotations, updateCaptureTitle } from "./storage";

export type OcrProgress = { status: string; progress: number };

export interface HostBridge {
  captureArea(): Promise<Blob | null>;
  saveCapture(record: CaptureRecord): Promise<void>;
  listCaptures(): Promise<CaptureRecord[]>;
  getCapture(id: string): Promise<CaptureRecord | undefined>;
  updateCaptureAnnotations(id: string, annotations: TldrawState): Promise<boolean>;
  updateCaptureTitle(id: string, title: string): Promise<boolean>;
  deleteCapture(id: string): Promise<void>;
  copyImage(blob: Blob): Promise<void>;
  copyFile(blob: Blob, fileName?: string): Promise<void>;
  copyText(text: string): Promise<void>;
  recognize(blob: Blob, onProgress?: (p: OcrProgress) => void): Promise<OcrResult>;
}

export const browserHost: HostBridge = {
  captureArea: async () => null,
  saveCapture,
  listCaptures,
  getCapture,
  updateCaptureAnnotations,
  updateCaptureTitle,
  deleteCapture,
  copyImage: copyImageToClipboard,
  copyFile: async (blob, fileName = "capture.png") => downloadBlob(blob, fileName),
  copyText,
  recognize: recognizeText,
};

export let host: HostBridge = browserHost;

export function setHost(next: HostBridge): void {
  host = next;
}
