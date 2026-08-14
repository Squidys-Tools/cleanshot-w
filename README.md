# mymind library

A Windows-first, local-first visual library for saving and rediscovering ideas, images, articles, notes, PDFs, quotes, and other things worth remembering.

The project is inspired by the idea of saving without organizing. Content is captured quickly, understood in the background, and retrieved through search, visual browsing, semantic connections, and automatically updated Spaces.

> Early development. The current repository contains the Tauri application scaffold and product/architecture planning documents.

## Stack

- Tauri 2 for the Windows desktop shell
- React, TypeScript 7, and Vite for the interface
- Bun for JavaScript dependencies and scripts
- Rust for the native application core
- SQLite and FTS5 for local storage and full-text search
- Tesseract for local OCR
- Defuddle for readable article extraction
- ONNX Runtime and local models for embeddings and image understanding
- llama.cpp for optional local summaries and structured analysis

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

Check the Rust application:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
```

## Project documents

- [Product behavior specification](docs/product-behavior-spec.md)
- [Technical decision record](docs/technical-decision-record.md)
- [AI and extraction benchmark plan](benchmarks/README.md)
- [Benchmark manifest](benchmarks/manifest.json)

## Direction

The first technical milestone is a vertical slice that can accept an image, URL, and PDF; create cards immediately; extract readable content; run local OCR; index content; support semantic and image similarity search; and save a search as a Smart Space.

The application is designed for Windows first while keeping the frontend and domain boundaries portable enough for a future web version.
