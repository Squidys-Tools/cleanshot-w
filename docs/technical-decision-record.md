# Technical Decision Record

## Status

Proposed for the Windows-first implementation.

## Decision summary

Build a local-first Windows desktop application with Tauri, React, TypeScript, Rust, and SQLite. Keep the frontend portable enough for a future web application, but let the desktop application own local storage, background processing, asset management, and model execution.

## Application stack

| Area | Decision | Reason |
|---|---|---|
| Desktop shell | Tauri 2 | Windows desktop packaging with a reusable web frontend and native integration |
| UI | React + TypeScript + Vite | Mature component ecosystem and future web portability |
| Styling | Tailwind CSS plus accessible headless primitives | Fast iteration without locking the product to a heavy component theme |
| Client state | Zustand | Small, explicit state layer for UI state |
| Server-style data state | TanStack Query | Caching and invalidation for asynchronous repository calls |
| Native core | Rust | File, job queue, database, and model-process orchestration |
| Database | SQLite through rusqlite | Direct control over migrations, transactions, FTS5, and extensions |
| Exact search | SQLite FTS5 | Local full-text search across content, notes, OCR, and summaries |
| Asset storage | Content-addressed local files | Avoid duplicate assets and make backup/restore predictable |
| Background work | Rust job queue persisted in SQLite | Retryable processing without blocking the UI |

## Article extraction

Use Defuddle as the initial article extraction engine rather than implementing a new readability parser or forking Defuddle.

Defuddle was created for Obsidian Web Clipper and provides cleaned HTML or Markdown plus metadata such as title, author, domain, favicon, image, publication date, and schema.org data. It also exposes configurable extraction steps and a Node bundle.

### Integration approach

1. Fetch or receive the original HTML.
2. Run Defuddle.
3. Store cleaned HTML, Markdown, and metadata.
4. Run a post-processing media normalizer.
5. Cache eligible images locally.
6. Preserve supported video embeds through an allowlist.
7. Sanitize all URLs, attributes, and embedded content.

Do not modify Defuddle until real corpus testing shows a repeatable problem. Prefer configuration, site-specific extractors, and a post-processing layer.

## OCR

Use Tesseract as the default OCR engine. OCR output is stored as searchable text and is not presented as a separately managed AI artifact. The user should simply be able to search for words that appear in an image or screenshot.

The OCR interface should remain replaceable so language packs, preprocessing, or a future alternative engine can be added without changing the item model.

## Local AI architecture

AI is a background service, not a frontend concern.

```text
Capture
  -> Normalize
  -> Extract text and metadata
  -> OCR / PDF extraction / article parsing
  -> Generate derived concepts and summaries
  -> Generate text and image embeddings
  -> Update search indexes
  -> Recalculate Smart Space membership
```

### Runtime choices

| Capability | Initial choice |
|---|---|
| OCR | Tesseract |
| Text embeddings | Small ONNX model through ONNX Runtime |
| Image embeddings | CLIP-compatible ONNX model |
| Image concepts | Deterministic image analysis plus vision/embedding model |
| Summaries | Optional quantized local LLM through llama.cpp |
| PDF extraction | Local PDF text extraction, followed by Tesseract for scanned pages |
| Vector search | Start with a simple local implementation; evaluate sqlite-vec after benchmarking |

ONNX Runtime is appropriate for Windows CPU/GPU inference. llama.cpp is appropriate for a bundled or downloaded quantized local language model and provides Windows builds and a local server interface.

`sqlite-vec` is worth evaluating, but should not be a hard dependency before benchmarking because it is pre-1.0 and may introduce breaking changes.

## AI behavior rules

- AI-derived concepts are invisible implementation details.
- Do not label tags as AI-generated.
- Do not show confidence scores.
- Do not expose tag correction or deletion workflows.
- Do not overwrite user-authored titles, notes, or tags.
- AI failures should degrade gracefully to ordinary metadata and text search.
- Processing should happen asynchronously after capture.
- A user should be able to use the app while processing is incomplete.

## Model selection policy

Models will be selected using a representative local test corpus, not by reputation alone. Evaluate OCR accuracy, concept usefulness, summary quality, semantic search relevance, image similarity relevance, CPU time, RAM usage, GPU acceleration, download size, cold-start time, and batch processing time.

Support three operating modes:

1. Baseline CPU mode.
2. Standard mode for typical 16 GB machines.
3. Accelerated mode when a supported GPU is available.

## Data, backup, and security

The database and asset store should be exportable together. Backups should include the SQLite database, local assets, original source URLs, cleaned article content, user notes and tags, and Spaces. Derived AI data may be regenerated, but exporting it is useful for faster restore and future migration.

Treat all fetched HTML as untrusted. Sanitize article HTML before rendering, allowlist iframe providers, strip scripts/event handlers/unsafe URLs/untrusted `srcdoc`, keep remote content isolated from privileged Tauri APIs, and do not expose arbitrary filesystem or shell commands to the webview.

## First technical milestone

Implement a vertical slice that accepts an image, URL, and PDF; creates a card immediately; runs Tesseract on images and scanned PDFs; extracts an article with Defuddle; stores local assets and extracted content; indexes text with SQLite FTS5; produces one text embedding and one image embedding; supports semantic search and image similarity; and saves a search as a Smart Space.

The model benchmark corpus should be created immediately before this milestone. It will include articles, screenshots, PDFs, images with text, handwriting, video embeds, and visually similar image sets.
