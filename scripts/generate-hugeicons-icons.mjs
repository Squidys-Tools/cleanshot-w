// Generates the Snipsy icon assets from HugeIcons' free MIT set.
//
// Outputs, all committed:
//   app/cleanshot/assets/icons/hugeicons/<name>.svg      chrome icons (gpui svg element)
//   app/cleanshot/assets/icons/hugeicons/<name>.png      sticker PNGs at 512px (canvas + flatten)
//   app/cleanshot/src/editor/icon_assets.rs              the sticker include_bytes! table
//   app/cleanshot/src/ui_chrome.rs                       the chrome svg include_bytes! table
//
// Usage: `npm install --prefix scripts` (installs this script's two dev
// dependencies) then `node scripts/generate-hugeicons-icons.mjs`.
// PNG rasterization uses @resvg/resvg-js (no browser needed). Icons here are
// MIT-licensed (see app/cleanshot/assets/icons/hugeicons.LICENSE); never point
// this script at HugeIcons Pro data.

import { mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const HugeIcons = require("@hugeicons/core-free-icons");
const { Resvg } = require("@resvg/resvg-js");

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(root, "app/cleanshot/assets/icons/hugeicons");
const rustTable = resolve(root, "app/cleanshot/src/editor/icon_assets.rs");
const chromeTable = resolve(root, "app/cleanshot/src/ui_chrome.rs");

// The single source of truth for what Snipsy bundles.
// Tool chrome:
const chrome = {
  select: "CursorRectangleSelectionIcon",
  rectangle: "SquareIcon",
  ellipse: "CircleIcon",
  arrow: "ArrowUpRight01Icon",
  pen: "Pen01Icon",
  text: "TypeIcon",
  counter: "OneSquareIcon",
  crop: "CropIcon",
  blur: "BlurIcon",
  pixelate: "Grid2x2Icon",
  redact: "MaskIcon",
  sticker: "StickerIcon",
  undo: "UndoIcon",
  redo: "RedoIcon",
  copy: "Copy01Icon",
  save: "Download04Icon",
  history: "HistoryIcon",
  preferences: "Settings01Icon",
  send_backward: "LayerSendBackwardIcon",
  bring_forward: "LayerBringForwardIcon",
  delete: "Delete02Icon",
  close: "Cancel01Icon",
  zoom_in: "ZoomInIcon",
  zoom_out: "ZoomOutIcon",
  fit: "Maximize01Icon",
  apply: "TickIcon",
  refresh: "RefreshIcon",
  search: "Search01Icon",
  monitor: "MonitorIcon",
  webcam: "WebcamIcon",
  opacity_up: "AddCircleIcon",
  opacity_down: "MinusSignIcon",
  click_through: "MouseIcon",
  pin: "PinIcon",
};
// Canvas sticker library (also user-visible labels in the picker):
const stickers = {
  arrow_right: ["ArrowRight01Icon", "Arrow right"],
  check: ["TickIcon", "Check"],
  close: ["Cancel01Icon", "Close"],
  info: ["InformationCircleIcon", "Information"],
  warning: ["Alert02Icon", "Warning"],
  question: ["HelpCircleIcon", "Question"],
  star: ["StarIcon", "Star"],
  heart: ["HeartIcon", "Heart"],
  lock: ["LockIcon", "Lock"],
  eye: ["ViewIcon", "Eye"],
  user: ["UserIcon", "Person"],
  camera: ["Camera01Icon", "Camera"],
  flag: ["Flag01Icon", "Flag"],
  pin_sticker: ["PinIcon", "Pin"],
  thumbs_up: ["ThumbsUpIcon", "Thumbs up"],
  clock: ["Clock01Icon", "Clock"],
  mail: ["Mail01Icon", "Mail"],
  smile: ["SmileIcon", "Smile"],
  bookmark: ["BookmarkIcon", "Bookmark"],
  check_circle: ["CheckmarkCircleIcon", "Check circle"],
  cross_circle: ["CancelCircleIcon", "Cross circle"],
  plus: ["AddCircleIcon", "Plus"],
  minus: ["MinusSignIcon", "Minus"],
  scissors: ["ScissorsIcon", "Scissors"],
};

function toSvg(elements) {
  const body = elements
    .map(([tag, props]) => {
      const attrs = Object.entries(props)
        .filter(([k]) => k !== "key")
        .map(([k, v]) => `${k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase())}="${v}"`)
        .join(" ");
      return `<${tag} ${attrs}/>`;
    })
    .join("");
  return `<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">${body}</svg>`;
}

function resolveIcon(name) {
  const value = HugeIcons[name];
  if (!value) throw new Error(`Icon ${name} is not in @hugeicons/core-free-icons`);
  return value;
}

function toPng(svg, size) {
  const resvg = new Resvg(svg.replace('width="24" height="24"', `width="${size}" height="${size}"`), {
    fitTo: "original",
  });
  return resvg.render().asPng();
}

await mkdir(output, { recursive: true });
const written = [];

for (const [slug, name] of Object.entries({ ...chrome, ...Object.fromEntries(Object.entries(stickers).map(([k, [n]]) => [k, n])) })) {
  await writeFile(resolve(output, `${slug}.svg`), toSvg(resolveIcon(name)));
  written.push([slug, name]);
}

// Sticker PNGs at 512px: currentColor resolves to black, and the runtime tint
// keeps only the alpha channel, so black-on-transparent is what we want.
const pngTargets = Object.entries(stickers);
for (const [slug] of pngTargets) {
  await writeFile(resolve(output, `${slug}.png`), toPng(toSvg(resolveIcon(stickers[slug][0])), 512));
}

// Rust tables: stickers are the only runtime-rasterized assets; chrome icons
// render through gpui's svg() element straight from the bundled bytes.
let rust = `// Generated by scripts/generate-hugeicons-icons.mjs — do not edit by hand.\n`;
rust += `pub(super) const ICON_ASSETS: &[(IconGlyph, &[u8])] = &[\n`;
for (const [slug] of pngTargets) {
  const variant = slug
    .split("_")
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join("");
  rust += `    (IconGlyph::${variant}, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/hugeicons/${slug}.png"))),\n`;
}
rust += "];\n";
await writeFile(rustTable, rust);

let chromeRust = `// Generated by scripts/generate-hugeicons-icons.mjs — do not edit by hand.\n`;
chromeRust += `pub(super) const CHROME_ICONS: &[(&str, &[u8])] = &[\n`;
for (const [slug] of Object.entries(chrome)) {
  chromeRust += `    ("${slug}", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/hugeicons/${slug}.svg"))),\n`;
}
chromeRust += "];\n";
await writeFile(chromeTable, chromeRust);

console.log(`Wrote ${written.length} SVGs, ${pngTargets.length} sticker PNGs, and both Rust tables.`);
