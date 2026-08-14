import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export type StoredItemKind = "note" | "article" | "image" | "pdf" | "video" | "file" | "embed";

export type StoredLibraryItem = {
  id: string;
  kind: StoredItemKind | string;
  title: string | null;
  description: string | null;
  sourceUrl: string | null;
  sourceLabel: string | null;
  localAssetPath: string | null;
  thumbnailPath: string | null;
  metadata: Record<string, unknown>;
  createdAt: number;
  updatedAt: number;
  archived: boolean;
  favorite: boolean;
};

export type CreateNoteInput = {
  title?: string;
  body: string;
  metadata?: Record<string, unknown>;
};

export type CreateUrlInput = {
  sourceUrl: string;
  title: string;
  description: string;
  body: string;
  metadata?: Record<string, unknown>;
};

export type SaveFileInput = {
  fileName: string;
  mimeType: string;
  kind: "image" | "pdf" | "video" | "other";
  bytes: number[];
};

const runtimeIsTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function isTauriRuntime() {
  return runtimeIsTauri;
}

export async function initializeStorage() {
  if (!runtimeIsTauri) return null;
  return invoke<{ databasePath: string; fts5Enabled: boolean; schemaVersion: number }>("initialize_storage");
}

export async function listActiveItems() {
  return invoke<StoredLibraryItem[]>("list_active_items");
}

export async function searchItems(query: string) {
  return invoke<StoredLibraryItem[]>("search_items", { query, limit: 100 });
}

export async function createNote(input: CreateNoteInput) {
  return invoke<StoredLibraryItem>("create_note", { input });
}

export async function createUrl(input: CreateUrlInput) {
  return invoke<StoredLibraryItem>("create_url", { input });
}

export async function saveFile(input: SaveFileInput) {
  return invoke<StoredLibraryItem>("save_file", { input });
}

export async function assetUrl(path: string | null) {
  if (!path) return undefined;
  if (!runtimeIsTauri) return path;
  const absolutePath = await invoke<string>("resolve_asset_path", { path });
  return convertFileSrc(absolutePath);
}

export async function archiveItem(id: string) {
  return invoke<StoredLibraryItem>("archive_item", { id, archived: true });
}
