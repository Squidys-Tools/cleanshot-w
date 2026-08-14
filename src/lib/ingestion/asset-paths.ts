import type { LibraryFileKind } from "./file-classification";

export type ThumbnailMode =
  | "image-raster"
  | "pdf-first-page"
  | "video-first-frame"
  | "none";

export interface ThumbnailPolicy {
  kind: LibraryFileKind;
  generate: boolean;
  mode: ThumbnailMode;
  outputExtension: "webp" | "png" | null;
  maxWidth: number;
  maxHeight: number;
}

/**
 * The thumbnail policy is intentionally a deterministic mapping from file kind
 * to work. It does not inspect content or invoke an AI model.
 */
export function thumbnailPolicyFor(kind: LibraryFileKind): ThumbnailPolicy {
  switch (kind) {
    case "image":
      return {
        kind,
        generate: true,
        mode: "image-raster",
        outputExtension: "webp",
        maxWidth: 960,
        maxHeight: 960,
      };
    case "pdf":
      return {
        kind,
        generate: true,
        mode: "pdf-first-page",
        outputExtension: "png",
        maxWidth: 960,
        maxHeight: 1280,
      };
    case "video":
      return {
        kind,
        generate: true,
        mode: "video-first-frame",
        outputExtension: "webp",
        maxWidth: 960,
        maxHeight: 540,
      };
    case "other":
      return {
        kind,
        generate: false,
        mode: "none",
        outputExtension: null,
        maxWidth: 0,
        maxHeight: 0,
      };
  }
}

export type AssetVariant = "original" | "thumbnail" | "preview";

export interface AssetPathOptions {
  itemId: string;
  variant: AssetVariant;
  extension: string;
}

/** Produces one safe, case-insensitive path segment. */
export function sanitizeAssetSegment(value: string, fallback = "asset"): string {
  const sanitize = (candidate: string): string => candidate
    .normalize("NFKC")
    .trim()
    .toLowerCase()
    .replace(/[\\/]/gu, "-")
    .replace(/[^a-z0-9._-]/gu, "-")
    .replace(/-+/gu, "-")
    .replace(/^\.+/u, "")
    .replace(/\.+$/u, "");

  const safe = sanitize(value);
  if (safe.length > 0 && safe !== "." && safe !== "..") return safe;

  const safeFallback = sanitize(fallback);
  return safeFallback.length > 0 && safeFallback !== "." && safeFallback !== ".." ? safeFallback : "asset";
}

function safeExtension(extension: string): string {
  const normalized = extension.replace(/^\./u, "").toLowerCase();
  return /^[a-z0-9]{1,12}$/u.test(normalized) ? normalized : "bin";
}

/** Returns a stable relative filename with no user-controlled path separators. */
export function assetFileName(options: Omit<AssetPathOptions, "itemId">): string {
  return `${sanitizeAssetSegment(options.variant)}.${safeExtension(options.extension)}`;
}

/** Returns the app-relative path used beneath its managed asset directory. */
export function assetRelativePath(options: AssetPathOptions): string {
  return `items/${sanitizeAssetSegment(options.itemId)}/${assetFileName(options)}`;
}

export function thumbnailRelativePath(
  itemId: string,
  kind: LibraryFileKind,
): string | null {
  const policy = thumbnailPolicyFor(kind);
  return policy.outputExtension
    ? assetRelativePath({
        itemId,
        variant: "thumbnail",
        extension: policy.outputExtension,
      })
    : null;
}

/**
 * Small deterministic non-cryptographic key for generated asset folders.
 * It is for stable naming, not identity verification or security.
 */
export function deterministicAssetKey(seed: string): string {
  let hash = 2166136261;
  for (const character of seed.normalize("NFKC")) {
    hash ^= character.codePointAt(0) ?? 0;
    hash = Math.imul(hash, 16777619);
  }
  return `asset-${(hash >>> 0).toString(16).padStart(8, "0")}`;
}
