# CleanShot W

A native Windows screenshot tool — capture, annotate, OCR, export. Local-first,
no account, no telemetry, no cloud.

## What it does

- **Capture:** area, window, or full screen (via global hotkeys)
- **Annotate:** rect, ellipse, arrow, line, text, numbered steps, highlight,
  blur, pixelate, redact
- **OCR:** select and copy text out of screenshots (tesseract.js, bundled locally)
- **Export:** copy image to clipboard (alpha-safe), save PNG, or copy text
- **Library:** disk-backed capture history with thumbnails and re-editing

## Tech stack

| Layer | Technology |
|---|---|
| App shell | Tauri 2 (Rust + WebView2) |
| Editor | React 19 + TypeScript 7 + Vite 8 |
| Canvas | tldraw |
| OCR | tesseract.js (bundled wasm + traineddata) |
| Clipboard | arboard (Rust) |
| Capture | Win32 GDI + DXGI |

## Quick start

```bash
# Browser-only editor (no native deps)
bun install
bun run dev

# Full Tauri build
bun install
bun run tauri build
```

See [BUILDING.md](./BUILDING.md) for full dev setup and toolchain details.

## Docs

| Document | What's in it |
|---|---|
| [STATUS.md](./STATUS.md) | Current milestone, per-item checklist, what's done and next up |
| [ROADMAP.md](./ROADMAP.md) | Full plan, milestones, constraints, engineering notes |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | hostBridge contract, data model, rendering, DPI, capture pipeline |
| [CHANGELOG.md](./CHANGELOG.md) | Every notable change, Keep a Changelog format |
| [BUILDING.md](./BUILDING.md) | Dev/build/test commands, toolchain, risk spikes |
| [DISTRIBUTION.md](./DISTRIBUTION.md) | Packaging, release checklist, target directory layout |

## Project structure

```
├── src/                    React + TypeScript editor
│   ├── components/         UI components (Editor, QuickAccess, etc.)
│   │   └── editor/         tldraw canvas, custom shapes/tools
│   └── lib/                Bridge, export, geometry, OCR, storage
├── src-tauri/              Rust/Tauri shell
│   ├── src/                capture, clipboard, hotkeys, library
│   └── capabilities/       Tauri permissions
├── tests/                  Unit tests (geometry, preferences)
├── scripts/                OCR asset bundler, smoke tests
└── docs/                   Project documentation
```

## License

MIT
