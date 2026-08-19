# Building and Development

## Toolchain

```
Node 24 + bun          already present
Rust + GNU toolchain   present — builds Tauri on Windows without MSVC (verified)
TypeScript 7.x         upgraded (native compiler)
WebView2               Evergreen runtime on Win10/11 (startup check + clear error if missing)
Tesseract              none system-wide — bundled wasm (M1) / native later (M3)
```

**Tauri builds with the GNU toolchain — MSVC is not required.**
Verified 2026-08-14: `x86_64-pc-windows-gnu` + MinGW GCC produced
`src-tauri/target/release/cleanshot-w.exe` (~15 min first build).

## Editor dev loop (browser-only, M1)

```bash
bun install
bun run dev          # plain Vite in a browser, no native dependencies
```

## Tauri shell dev loop (M2+)

```bash
bun install
bun run tauri dev    # builds Rust + launches Tauri window
```

## Building

```bash
bun run build        # TypeScript + Vite production build
bun run tauri build  # Full Tauri release build
```

## Testing

- **Unit tests:** `bun test` — geometry and preferences tests
- **Browser smoke tests:** `bun run smoke` — Playwright against Vite dev server
- **OCR assets:** `bun run ocr:assets` — bundles tesseract.js wasm + traineddata locally

## Key scripts

| Script | Command | Description |
|---|---|---|
| `dev` | `bun run dev` | Start Vite dev server |
| `build` | `bun run build` | TypeScript + Vite production build |
| `test` | `bun test` | Run unit tests |
| `smoke` | `bun run smoke` | Browser smoke tests (32 checks) |
| `ocr:assets` | `bun run ocr:assets` | Bundle tesseract.js assets |
| `tauri` | `bun run tauri` | Tauri CLI passthrough |

## Risks and spikes (pre-M2)

| Risk | Spike | Pass criteria |
|---|---|---|
| Transparent Tauri overlay across mixed-DPI monitors | 2-monitor (100% + 150%) test: rect must match cursor under loupe | no drift, input routed correctly, no flicker |
| `xcap`/GDI capture on mixed DPI | Capture both monitors, crop by monitor | per-monitor pixels correct |
| Copy image with alpha into Word/Slack | Paste test after `arboard::set_image` | transparent bg survives |
| Tesseract worker + wasm under Tauri asset protocol | Run OCR in a production build | no CORS/path errors |
| `PrintWindow` window capture on occluded/windowed apps | Capture VS Code + browser | correct pixels, no black window |
| tesseract.js speed/quality on screenshots | Run benchmark harness | decision recorded: fast vs best |
