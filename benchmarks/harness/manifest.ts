import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";

export interface ExpectedSearch {
  title?: string;
  author?: string;
  search_terms?: string[];
  must_not_match?: string[];
  ocr_text_file?: string;
  image_urls_min?: number;
  safe_embeds?: { provider: string; min?: number }[];
  safe_embeds_max?: number;
  embedded_images_min?: number;
}

export interface ManifestItem {
  id: string;
  path: string;
  type: string;
  language: string;
  similarity_group?: string;
  expected?: ExpectedSearch;
  notes?: string;
}

export interface Manifest {
  version: number;
  items: ManifestItem[];
}

/** benchmarks/ directory, resolved from this file. */
export const benchmarksRoot = resolve(import.meta.dir, "..");

export function corpusPath(relative: string): string {
  return resolve(benchmarksRoot, relative);
}

export function expectedPath(relative: string): string {
  return resolve(benchmarksRoot, relative);
}

export function loadManifest(): Manifest {
  const manifestPath = resolve(benchmarksRoot, "manifest.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as Manifest;
  const missing = manifest.items.filter((item) => !existsSync(corpusPath(item.path)));
  if (missing.length > 0) {
    throw new Error(
      `Manifest references missing files: ${missing.map((item) => item.path).join(", ")}`,
    );
  }
  return manifest;
}
