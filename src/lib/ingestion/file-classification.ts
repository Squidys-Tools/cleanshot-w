/** Pure file classification helpers used by the ingestion pipeline. */

export type LibraryFileKind = "image" | "pdf" | "video" | "other";

export interface FileClassificationInput {
  name?: string | null;
  type?: string | null;
}

const IMAGE_EXTENSIONS = new Set([
  "avif",
  "bmp",
  "gif",
  "heic",
  "heif",
  "jpeg",
  "jpg",
  "png",
  "svg",
  "tif",
  "tiff",
  "webp",
]);

const VIDEO_EXTENSIONS = new Set([
  "avi",
  "m4v",
  "mkv",
  "mov",
  "mp4",
  "mpeg",
  "mpg",
  "ogv",
  "webm",
]);

function fileExtension(name: string | null | undefined): string {
  const lastSegment = (name ?? "").split(/[\\/]/u).pop() ?? "";
  const dot = lastSegment.lastIndexOf(".");
  return dot > -1 ? lastSegment.slice(dot + 1).toLowerCase() : "";
}

/** Classifies a browser File or any object with File-like name/type fields. */
export function classifyFile(input: FileClassificationInput): LibraryFileKind {
  const mimeType = (input.type ?? "").split(";", 1)[0].trim().toLowerCase();
  const extension = fileExtension(input.name);

  // Prefer a recognized MIME type. A misleading filename must not make an
  // image or video upload behave like a PDF (or vice versa).
  if (mimeType === "application/pdf") {
    return "pdf";
  }

  if (mimeType.startsWith("image/")) {
    return "image";
  }

  if (mimeType.startsWith("video/")) {
    return "video";
  }

  if (extension === "pdf") return "pdf";
  if (IMAGE_EXTENSIONS.has(extension)) return "image";
  if (VIDEO_EXTENSIONS.has(extension)) return "video";

  return "other";
}

export function extensionForFile(input: FileClassificationInput): string | null {
  const extension = fileExtension(input.name);
  return extension.length > 0 ? extension : null;
}
