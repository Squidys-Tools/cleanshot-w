import { invoke } from "@tauri-apps/api/core";
import type { CaptureRecord, TldrawState } from "../types";
import { recognizeText } from "./ocr";
import type { HostBridge } from "./bridge";

type NativeCaptureRecord = {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  image: { width: number; height: number };
  imageBase64: string;
  thumbBase64: string;
  annotations: TldrawState;
};

async function blobToBase64(blob: Blob): Promise<string> {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return btoa(binary);
}

async function base64ToBlob(value: string): Promise<Blob> {
  const response = await fetch(`data:image/png;base64,${value}`);
  if (!response.ok) {
    throw new Error(`Could not decode the local capture (${response.status}).`);
  }
  return response.blob();
}

export async function readNativeClipboardImage(): Promise<Blob | null> {
  const value = await invoke<string | null>("read_image_from_clipboard");
  return value ? base64ToBlob(value) : null;
}

async function toRecord(wire: NativeCaptureRecord): Promise<CaptureRecord> {
  const imageBlob = await base64ToBlob(wire.imageBase64);
  const thumbBlob = await base64ToBlob(wire.thumbBase64);
  return {
    id: wire.id,
    title: wire.title,
    createdAt: wire.createdAt,
    updatedAt: wire.updatedAt,
    imageBlob,
    thumbBlob,
    image: wire.image,
    annotations: wire.annotations,
  };
}

async function toWire(record: CaptureRecord): Promise<NativeCaptureRecord> {
  return {
    id: record.id,
    title: record.title,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
    image: record.image,
    imageBase64: await blobToBase64(record.imageBlob),
    thumbBase64: await blobToBase64(record.thumbBlob),
    annotations: record.annotations,
  };
}

export const nativeHost: HostBridge = {
  captureArea: async () => null,
  saveCapture: async (record) => {
    await invoke<void>("library_save_capture", { record: await toWire(record) });
  },
  listCaptures: async () => {
    const records = await invoke<NativeCaptureRecord[]>("library_list_captures");
    return Promise.all(records.map(toRecord));
  },
  getCapture: async (id) => {
    const record = await invoke<NativeCaptureRecord | null>("library_get_capture", { id });
    return record ? toRecord(record) : undefined;
  },
  updateCaptureAnnotations: async (id, annotations) =>
    invoke<boolean>("library_update_annotations", { id, annotations }),
  deleteCapture: async (id) => {
    await invoke<void>("library_delete_capture", { id });
  },
  copyImage: async (blob) => {
    await invoke<void>("copy_image_to_clipboard", { pngBase64: await blobToBase64(blob) });
  },
  copyFile: async (blob) => {
    await invoke<void>("copy_file_to_clipboard", { pngBase64: await blobToBase64(blob) });
  },
  copyText: async (text) => {
    await invoke<void>("copy_text_to_clipboard", { text });
  },
  recognize: (blob, onProgress) => recognizeText(blob, onProgress),
};
