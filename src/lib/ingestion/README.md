# Ingestion utilities

These modules are pure, UI-independent building blocks for the local ingestion
pipeline.

- `file-classification.ts` classifies a File-like value as `image`, `pdf`,
  `video`, or `other`, using MIME type first and the filename extension as a
  fallback.
- `asset-paths.ts` provides the deterministic thumbnail policy and safe,
  app-relative asset paths. Generated previews are `webp` for images/videos and
  `png` for the first PDF page; other files do not get a generated thumbnail.
- `safe-embeds.ts` canonicalizes YouTube and Vimeo URLs to fixed provider
  origins. Direct video files require an exact HTTPS origin in
  `allowedDirectMediaOrigins`. `javascript:`, `data:`, `blob:`, HTTP, URLs with
  credentials/ports, unknown hosts, and malformed provider URLs are rejected.

No function in this folder fetches network content, executes embeds, performs
OCR, or exposes AI-derived metadata. A later ingestion service can call these
helpers after Defuddle/Tesseract extraction and before persisting local assets.

