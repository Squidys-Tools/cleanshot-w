# mymind library

A Windows-first, local-first visual library for saving and rediscovering ideas, images, articles, notes, PDFs, quotes, and other things worth remembering.

The product treats search and associative memory as the organizing system instead of requiring users to maintain folders and metadata. Content is captured quickly, understood in the background, and retrieved through search, visual browsing, semantic connections, and automatically updated Spaces.

## Status

Early development. The browser preview uses seed data, while the Tauri runtime reads and writes the local SQLite library.

- **Working:** URL capture with article extraction (Defuddle + fallback), local file capture (image/PDF/video), note capture, SQLite storage with FTS5 search, thumbnail generation, safe embed allowlists, the background job queue, and local OCR for saved images through Windows.Media.Ocr.
- **In progress:** PDF page OCR, text and image embeddings, semantic and visual search, and Smart Spaces are still planned work. The benchmark harness includes Windows OCR and tesseract.js adapters for comparing OCR results.

## Stack

- Tauri 2 for the Windows desktop shell
- React, TypeScript, and Vite for the interface
- Bun for JavaScript dependencies and scripts
- Rust for the native application core
- SQLite and FTS5 for local storage and full-text search
- Windows.Media.Ocr for native image OCR on Windows
- Defuddle for readable article extraction
- ONNX Runtime and local models for embeddings and image understanding (next milestone)
- llama.cpp for optional local summaries and structured analysis (next milestone)

## Development

Install dependencies:

```powershell
bun install
```

Start the desktop development app:

```powershell
bun run tauri dev
```

Build the web frontend:

```powershell
bun run build
```

Run the ingestion smoke tests:

```powershell
bun run ingest:smoke
```

Run the AI and extraction benchmark harness:

```powershell
bun run bench
```

Check the Rust application:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

## Project layout

```
src/
  App.tsx                  # Application UI (browser seed data, Tauri-backed runtime)
  main.tsx                 # React entry point
  App.css                  # Styles
  lib/
    libraryApi.ts          # Tauri invoke wrappers (storage, items, assets)
    ingestion/             # Frontend ingestion pipeline (TypeScript)
      index.ts             # Re-exports
      url.ts               # URL parsing, normalization, validation
      url-ingestion.ts     # ingestUrl: fetch, extract, sanitize, normalize
      defuddle-adapter.ts  # Defuddle article extraction + JSON-LD metadata
      fallback.ts          # Fallback metadata extraction from HTML head/dom
      html-safety.ts       # HTML sanitization, image URL collection, safe embeds
      safe-embeds.ts       # URL-only embed allowlist (YouTube, Vimeo, direct media)
      file-classification.ts  # File type detection by MIME type and extension
      asset-paths.ts       # Content-addressed asset path helpers
      json-ld.ts           # schema.org JSON-LD article metadata extraction
      errors.ts            # UrlIngestionError with typed codes
      ingestion.smoke.ts   # Smoke tests for the ingestion pipeline
src-tauri/
  Cargo.toml               # Rust dependencies (Tauri, rusqlite, image, uuid, tokio, windows)
  src/
    main.rs                # Entry point
    lib.rs                 # Tauri command handlers
    storage.rs             # SQLite storage, FTS5, asset management, thumbnails, OCR text
    jobs.rs                # Persisted background job queue and worker
    ocr.rs                 # Replaceable OCR backend interface and Windows backend
benchmarks/
  README.md                # Corpus and evaluation notes
  manifest.json            # Benchmark corpus definition (items + expected results)
  generate-corpus.ps1      # Regenerate image/screenshot/PDF fixtures
  fetch-macrumors.ts       # Fetch live MacRumors articles as fixtures
  harness/                 # Benchmark harness (TypeScript)
    index.ts               # Main benchmark entry point
    server.ts              # Local HTTP server for fixture pages
    extract.ts             # Article extraction scoring
    pdf.ts                 # Naive PDF text extractor for benchmarks
    ocr.ts                 # OCR engine abstraction (Windows OCR + tesseract.js)
    score.ts               # Scoring utilities (token recall/precision, term matching)
    manifest.ts            # Manifest type definitions and loading
    tesseract.d.ts         # Ambient types for optional tesseract.js
  corpus/                  # Test fixtures (articles, images, screenshots, PDFs, etc.)
  expected/                # Expected outputs for benchmark scoring
  results/                 # Benchmark result JSON and summaries
docs/
  product-behavior-spec.md  # Product behavior specification
  technical-decision-record.md  # Architecture and technology decisions
DESIGN.md                  # Visual design direction
PRODUCT.md                 # Product definition and principles
```

## Project documents

- [Product behavior specification](docs/product-behavior-spec.md)
- [Technical decision record](docs/technical-decision-record.md)
- [AI and extraction benchmark plan](benchmarks/README.md)
- [Benchmark manifest](benchmarks/manifest.json)
- [Visual design direction](DESIGN.md)

## Direction

The current technical slice accepts an image, URL, or PDF; creates a card immediately; stores local assets; extracts articles with Defuddle; and indexes titles, descriptions, metadata, and image OCR text with SQLite FTS5. Image OCR runs in a persisted background job and can be searched after processing finishes.

The remaining first-milestone work is scanned-PDF OCR, text and image embeddings, semantic and image-similarity search, and saved Smart Spaces.

The application is designed for Windows first while keeping the frontend and domain boundaries portable enough for a future web version.
