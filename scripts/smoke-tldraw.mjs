import { chromium } from "playwright";
import { deflateSync } from "node:zlib";

const BASE = "http://localhost:1420";
const W = 320;
const H = 180;

/* ------------------- minimal PNG (solid dark blue) ------------------- */

function crc32(buf) {
  let c;
  const table = [];
  for (let n = 0; n < 256; n++) {
    c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = table[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crc]);
}

function makePng(w, h) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  const rows = Buffer.alloc((w * 3 + 1) * h);
  for (let y = 0; y < h; y++) {
    const off = y * (w * 3 + 1);
    rows[off] = 0;
    for (let x = 0; x < w; x++) {
      const p = off + 1 + x * 3;
      rows[p] = 22;
      rows[p + 1] = 34;
      rows[p + 2] = 58;
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(rows)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

const png = makePng(W, H);

/* ------------------------------ helpers ------------------------------ */

const results = [];
function check(name, ok, extra = "") {
  results.push({ name, ok });
  console.log(`${ok ? "PASS" : "FAIL"}  ${name}${extra ? ` — ${extra}` : ""}`);
}

async function waitForSaved(page, count, timeout = 12000) {
  await page.waitForFunction(
    (expected) => {
      const el = document.querySelector(".cs-statusbar");
      return el && el.textContent.includes(`${expected} markup${expected === 1 ? "" : "s"}`);
    },
    count,
    { timeout },
  );
}

async function activeToolTitle(page) {
  return page.evaluate(
    () =>
      document.querySelector(".cs-toolbar .tool-btn.active:not(:disabled)")?.getAttribute("title") ?? "",
  );
}

async function expectTool(page, startsWith, name) {
  await page.waitForFunction(
    (prefix) =>
      document
        .querySelector(".cs-toolbar .tool-btn.active:not(:disabled)")
        ?.getAttribute("title")
        ?.startsWith(prefix),
    startsWith,
    { timeout: 5000 },
  );
  check(`${name} tool activates`, true);
}

/* --------------------------------- run -------------------------------- */

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
page.on("pageerror", (e) => check("no page errors", false, `pageerror: ${e.message}`));
page.on("console", (msg) => {
  if (msg.type() === "error") check("no console errors", false, `console: ${msg.text()}`);
});

try {
  await page.goto(BASE, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(".dropzone", { timeout: 15000 });
  check("dropzone shown", true);

  await page.setInputFiles('input[type="file"]', {
    name: "test.png",
    mimeType: "image/png",
    buffer: png,
  });

  await page.waitForSelector(".cs-tldraw .tl-container", { timeout: 15000 });
  check("tldraw canvas mounted", true);
  await page.waitForSelector(".cs-toolbar", { timeout: 10000 });
  check("overlay toolbar shown", true);

  await page.waitForFunction(
    ({ w, h }) => document.querySelector(".cs-statusbar")?.textContent.includes(`${w} × ${h}`),
    { w: W, h: H },
    { timeout: 12000 },
  );
  check("background image size in status bar", true);

  /* counter (Step) */
  await page.click('button[title^="Step"]');
  await expectTool(page, "Step", "step/counter");
  await page.mouse.click(640, 400);
  await waitForSaved(page, 1);
  check("counter created + autosaved", true);

  /* redact */
  await page.click('button[title^="Redact"]');
  await expectTool(page, "Redact", "redact");
  await page.mouse.move(430, 300);
  await page.mouse.down();
  await page.mouse.move(850, 500, { steps: 8 });
  await page.mouse.up();
  await waitForSaved(page, 2);
  check("redact box created + autosaved", true);

  /* custom-tool keyboard shortcuts (capture override for g/q) */
  await page.keyboard.press("m");
  await expectTool(page, "Blur", "blur key m");
  await page.keyboard.press("c");
  await expectTool(page, "Step", "counter key c");
  await page.keyboard.press("g");
  await expectTool(page, "Mosaic", "pixelate key g");
  await page.keyboard.press("q");
  await expectTool(page, "Redact", "redact key q");
  await page.keyboard.press("r");
  await expectTool(page, "Shape:", "geo key r");

  /* rectangle via geo tool */
  await page.mouse.move(460, 300);
  await page.mouse.down();
  await page.mouse.move(800, 520, { steps: 8 });
  await page.mouse.up();
  await waitForSaved(page, 3);
  check("rectangle created + autosaved", true);

  await page.waitForSelector(".cs-selection", { timeout: 5000 });
  check("selection bar shown on selection", true);

  /* duplicate via selection bar */
  await page.click('.cs-selection button[title^="Duplicate"]');
  await waitForSaved(page, 4);
  check("duplicate via selection bar", true);

  /* group / ungroup via native shortcuts */
  await page.keyboard.press("Control+a");
  await page.keyboard.press("Control+g");
  await waitForSaved(page, 1);
  check("group selected shapes (Ctrl+G)", true);
  await page.keyboard.press("Control+Shift+g");
  await waitForSaved(page, 4);
  check("ungroup selected shapes (Ctrl+Shift+G)", true);

  /* undo / redo via selection toolbar buttons */
  await page.click('.cs-toolbar button[title^="Undo"]');
  await waitForSaved(page, 1);
  check("undo works", true);
  await page.click('.cs-toolbar button[title^="Redo"]');
  await waitForSaved(page, 4);
  check("redo works", true);

  /* align + flip + order + lock (count stays 4) */
  await page.click('.cs-selection button[title^="Align"]');
  await page.click(".align-popover button.popover-item:has-text('Align left')");
  await page.waitForSelector(".align-popover", { state: "detached", timeout: 5000 });
  check("align popover closes after action", true);

  await page.click('.cs-selection button[title^="Flip horizontally"]');
  await page.click('.cs-selection button[title^="Arrange"]');
  await page.click(".align-popover button.popover-item:has-text('Bring to front')");
  await page.waitForSelector(".align-popover", { state: "detached", timeout: 5000 });

  await page.click('.cs-selection button[title^="Toggle lock"]');
  await page.waitForTimeout(800);
  check("flip/order/lock applied without errors", true);

  /* style controls */
  await page.click('.cs-toolbar .swatch[title="green"]');
  await page.waitForFunction(
    () => document.querySelector('.cs-toolbar .swatch[title="green"]')?.classList.contains("active"),
    { timeout: 5000 },
  );
  check("color swatch selects green", true);

  await page.click('.cs-toolbar .seg-btn[title="Solid fill"]');
  await page.click('.cs-toolbar .seg-btn[title="Dashed"]');
  await page.click('.cs-toolbar .size-btn[title^="Size L"]');
  await page.waitForFunction(
    () => {
      const fill = document.querySelector('.cs-toolbar .seg-btn[title="Solid fill"]')?.classList.contains("active");
      const dash = document.querySelector('.cs-toolbar .seg-btn[title="Dashed"]')?.classList.contains("active");
      const size = document.querySelector('.cs-toolbar .size-btn[title^="Size L"]')?.classList.contains("active");
      return fill && dash && size;
    },
    { timeout: 5000 },
  );
  check("fill/dash/size style buttons reflect state", true);

  /* style popover: font, opacity, dark, snap, grid */
  await page.click('.cs-toolbar button[title="More styles"]');
  await page.waitForSelector(".style-popover", { timeout: 5000 });
  await page.click('.style-popover .seg-btn[title="Mono"]');
  await page.click('.style-popover .seg-btn[title="50%"]');
  await page.click('.style-popover .cs-toggle:has-text("Dark") input');
  await page.click('.style-popover .cs-toggle:has-text("Grid") input');
  await page.click('.style-popover .cs-toggle:has-text("Snap") input');
  await page.waitForFunction(
    () => {
      const pop = document.querySelector(".style-popover");
      if (!pop) return false;
      const mono = pop.querySelector('.seg-btn[title="Mono"]')?.classList.contains("active");
      const fifty = pop.querySelector('.seg-btn[title="50%"]')?.classList.contains("active");
      const gridToggle = [...pop.querySelectorAll(".cs-toggle")].find((t) => t.textContent.includes("Grid"));
      return mono && fifty && !!gridToggle?.querySelector("input")?.checked;
    },
    { timeout: 5000 },
  );
  check("style popover font/opacity/toggles work", true);
  await page.click(".style-popover .popover-close");
  await page.waitForSelector(".style-popover", { state: "detached", timeout: 5000 });

  /* persistence across reload */
  await page.waitForTimeout(1400);
  await page.reload({ waitUntil: "domcontentloaded" });
  await page.waitForSelector(".history-item", { timeout: 15000 });
  await page.click(".history-item");
  await page.waitForSelector(".cs-tldraw .tl-container", { timeout: 15000 });
  await page.waitForFunction(
    () => document.querySelector(".cs-statusbar")?.textContent.includes("4 markups"),
    { timeout: 15000 },
  );
  check("doc persisted across reload (4 markups)", true);

  const historyText = await page.evaluate(() => document.querySelector(".history-meta span")?.textContent ?? "");
  check("history rail shows markup count", historyText.includes("4 marks"), historyText);

  const failed = results.filter((r) => !r.ok);
  console.log(`\n${results.length - failed.length}/${results.length} checks passed`);
  await browser.close();
  process.exit(failed.length ? 1 : 0);
} catch (e) {
  console.error("SMOKE FAILURE:", e.message.split("\n")[0]);
  await page.screenshot({ path: "smoke-failure.png", fullPage: false });
  await browser.close();
  process.exit(1);
}
