# CleanShot W

A native Windows screenshot tool — capture, annotate, OCR, export. Local-first, no account, no telemetry, no cloud.

Built with Tauri 2 (Rust + WebView2), React 19, TypeScript 7, and tldraw.

## Quick start

```bash
bun install
bun run dev        # browser-only editor
bun run tauri dev  # full Tauri shell
```

## Documentation

All docs live in [`docs/`](./docs/):

| Doc | Description |
|---|---|
| [Status](docs/STATUS.md) | **Start here** — current milestone, what's done, what's next |
| [Roadmap](docs/ROADMAP.md) | Full plan and milestones |
| [Architecture](docs/ARCHITECTURE.md) | hostBridge, data model, rendering, capture pipeline |
| [Changelog](docs/CHANGELOG.md) | Every notable change |
| [Building](docs/BUILDING.md) | Dev, build, and test commands |
| [Distribution](docs/DISTRIBUTION.md) | Packaging and release process |

## License

MIT
