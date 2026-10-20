# Bundled editor icons

One curated set of HugeIcons (free, MIT — see `hugeicons.LICENSE`) for Snipsy:

- `<name>.svg` — chrome icons (toolbars, buttons, nav), rendered by gpui's
  `svg()` element and tinted via its text color.
- `<name>.png` — canvas sticker icons, pre-rasterized at 512px because the
  canvas painter and the tiny-skia flatten step need pixels; tinted at render
  time by alpha-channel replacement.

Regenerate with `npm install --prefix scripts && node
scripts/generate-hugeicons-icons.mjs` after changing the mapping in
`scripts/generate-hugeicons-icons.mjs`; the outputs and the Rust tables
(`src/editor/icon_assets.rs`, `src/ui_chrome.rs`) are committed.

scripts/check-no-legacy-icons.mjs fails CI if any retired icon pack
reappears anywhere in the repo (purge list in
mockups/snipsy-ui-redesign/icon-mapping.md).
