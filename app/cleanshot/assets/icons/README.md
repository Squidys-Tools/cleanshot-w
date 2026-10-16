# Bundled editor icons

These are 12 monochrome symbols for each of the Lucide, Phosphor, Tabler,
Heroicons, and Remix packs. The app bundles them as transparent PNGs and tints
their alpha channel at render time; no icon package or SVG renderer is loaded
at runtime.

Run `node scripts/generate-editor-icons.mjs` from the repository root to
regenerate the PNGs and the Rust asset table. The generator uses the icon
packages already declared in `package.json` and the repository's Playwright
Chromium install.

Each pack's license notice is under `licenses/`. The Remix assets remain under
the Remix Icon License v1.0 and are included only as functional annotations in
the CleanShot W application, not as a standalone icon product.
