import type { CaptureRecord, TldrawState } from "../types";
import { normalizeCaptureTitle } from "./history";

const DB_NAME = "cleanshotw";
const STORE = "captures";
const VERSION = 1;

let dbPromise: Promise<IDBDatabase> | null = null;

function open(): Promise<IDBDatabase> {
  if (!dbPromise) {
    dbPromise = new Promise((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, VERSION);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains(STORE)) {
          db.createObjectStore(STORE, { keyPath: "id" });
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }
  return dbPromise;
}

async function tx<T>(mode: IDBTransactionMode, fn: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  const db = await open();
  return new Promise<T>((resolve, reject) => {
    const t = db.transaction(STORE, mode);
    const req = fn(t.objectStore(STORE));
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

export async function saveCapture(record: CaptureRecord): Promise<void> {
  await tx("readwrite", (s) => s.put(record));
}

export async function listCaptures(): Promise<CaptureRecord[]> {
  const all = await tx<CaptureRecord[]>("readonly", (s) => s.getAll() as IDBRequest<CaptureRecord[]>);
  return all.sort((a, b) => b.updatedAt - a.updatedAt);
}

export async function getCapture(id: string): Promise<CaptureRecord | undefined> {
  return tx<CaptureRecord | undefined>("readonly", (s) => s.get(id) as IDBRequest<CaptureRecord | undefined>);
}

export async function deleteCapture(id: string): Promise<void> {
  await tx("readwrite", (s) => s.delete(id));
}

/**
 * Merge annotations into an existing capture atomically. Reads and writes in a
 * single transaction so a save can never clobber a newer snapshot that landed
 * in between, and a deleted capture can never be resurrected. Returns false
 * when the capture no longer exists.
 */
export async function updateCaptureAnnotations(id: string, annotations: TldrawState): Promise<boolean> {
  const db = await open();
  return new Promise<boolean>((resolve, reject) => {
    const t = db.transaction(STORE, "readwrite");
    const store = t.objectStore(STORE);
    const get = store.get(id) as IDBRequest<CaptureRecord | undefined>;
    get.onsuccess = () => {
      const rec = get.result;
      if (!rec) {
        resolve(false);
        return;
      }
      const put = store.put({ ...rec, annotations, updatedAt: Date.now() });
      put.onsuccess = () => resolve(true);
      put.onerror = () => reject(put.error);
    };
    get.onerror = () => reject(get.error);
  });
}

/** Update only the title while preserving the image, thumbnail, and annotations. */
export async function updateCaptureTitle(id: string, title: string): Promise<boolean> {
  const nextTitle = normalizeCaptureTitle(title);
  if (!nextTitle || nextTitle.length > 500) return false;

  const db = await open();
  return new Promise<boolean>((resolve, reject) => {
    const t = db.transaction(STORE, "readwrite");
    const store = t.objectStore(STORE);
    const get = store.get(id) as IDBRequest<CaptureRecord | undefined>;
    get.onsuccess = () => {
      const rec = get.result;
      if (!rec) {
        resolve(false);
        return;
      }
      const put = store.put({ ...rec, title: nextTitle, updatedAt: Date.now() });
      put.onsuccess = () => resolve(true);
      put.onerror = () => reject(put.error);
    };
    get.onerror = () => reject(get.error);
  });
}

export async function clearCaptures(): Promise<void> {
  await tx("readwrite", (s) => s.clear());
}

export function loadImage(blob: Blob): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(blob);
    const img = new Image();
    img.onload = () => {
      resolve({ width: img.naturalWidth, height: img.naturalHeight });
      URL.revokeObjectURL(url);
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("Could not read image"));
    };
    img.src = url;
  });
}

export async function makeThumb(blob: Blob, maxWidth = 320): Promise<Blob> {
  const dims = await loadImage(blob);
  const scale = Math.min(1, maxWidth / dims.width);
  const w = Math.max(1, Math.round(dims.width * scale));
  const h = Math.max(1, Math.round(dims.height * scale));
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d")!;
  const url = URL.createObjectURL(blob);
  const img = new Image();
  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error("thumb load failed"));
    img.src = url;
  });
  ctx.drawImage(img, 0, 0, w, h);
  URL.revokeObjectURL(url);
  return new Promise((resolve) => canvas.toBlob((b) => resolve(b ?? blob), "image/png"));
}

export async function fileToBlob(file: File): Promise<Blob> {
  return new Blob([await file.arrayBuffer()], { type: file.type || "image/png" });
}
