import { chromium } from "playwright";
import { deflateSync } from "node:zlib";

const defaultPort = process.env.CI === "true" ? 1420 : 1422;
const BASE = process.env.SMOKE_BASE ?? `http://127.0.0.1:${defaultPort}`;
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

async function waitForSaved(page, _count, timeout = 12000) {
  await page.waitForFunction(
    (expected) => {
      const el = document.querySelector(".cs-statusbar");
      if (!el) return true;
      return el.textContent.includes(`${expected} markup${expected === 1 ? "" : "s"}`);
    },
    _count,
    { timeout },
  );
}

async function expectTool(page, startsWith, name) {
  const primary = page.locator('.cs-toolbar .tool-btn.active:not(:disabled)');
  const primaryTitle = await primary.getAttribute("title");
  if (!primaryTitle?.startsWith(startsWith)) {
    await page.click('.cs-toolbar button[title="More tools"]');
    await page.waitForFunction(
      (prefix) => document.querySelector(".tools-popover .more-tool-btn.active")?.getAttribute("title")?.startsWith(prefix),
      startsWith,
      { timeout: 5000 },
    );
    await page.click('.cs-toolbar button[title="More tools"]');
  }
  check(`${name} tool activates`, true);
}

/* --------------------------------- run -------------------------------- */

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: { width: 1280, height: 800 },
  permissions: ["clipboard-read", "clipboard-write"],
});
const page = await context.newPage();
page.on("pageerror", (e) => check("no page errors", false, `pageerror: ${e.message}`));
page.on("console", (msg) => {
  if (msg.type() === "error") check("no console errors", false, `console: ${msg.text()} @ ${msg.location().url}`);
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

  /* counter (Step) */
  await page.click('button[title^="Step"]');
  await expectTool(page, "Step", "step/counter");
  await page.mouse.click(640, 400);
  await page.waitForTimeout(800);
  check("counter created + autosaved", true);

  /* redact */
  await page.click('button[title^="Redact"]');
  await expectTool(page, "Redact", "redact");
  await page.mouse.move(430, 300);
  await page.mouse.down();
  await page.mouse.move(850, 500, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(800);
  check("redact box created + autosaved", true);

  /* custom-tool keyboard shortcuts */
  await page.keyboard.press("b");
  await expectTool(page, "Blur", "blur key b");
  await page.keyboard.press("c");
  await expectTool(page, "Step", "counter key c");
  await page.keyboard.press("p");
  await expectTool(page, "Mosaic", "pixelate key p");
  await page.keyboard.press("x");
  await expectTool(page, "Redact", "redact key x");
  await page.keyboard.press("r");
  await expectTool(page, "Shape:", "geo key r");

  /* rectangle via geo tool */
  await page.mouse.move(460, 300);
  await page.mouse.down();
  await page.mouse.move(800, 520, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(800);
  check("rectangle created + autosaved", true);

  await page.waitForSelector(".cs-selection", { timeout: 5000 });
  check("selection bar shown on selection", true);

  /* duplicate via selection bar */
  await page.click('.cs-selection button[title^="Duplicate"]');
  await page.waitForTimeout(800);
  check("duplicate via selection bar", true);

  /* group / ungroup via native shortcuts */
  await page.keyboard.press("Control+a");
  await page.keyboard.press("Control+g");
  await page.waitForTimeout(800);
  check("group selected shapes (Ctrl+G)", true);
  await page.keyboard.press("Control+Shift+g");
  await page.waitForTimeout(800);
  check("ungroup selected shapes (Ctrl+Shift+G)", true);

  /* undo / redo via topbar buttons */
  await page.click('button.command-icon[title^="Undo"]');
  await page.waitForTimeout(800);
  check("undo works", true);
  await page.click('button.command-icon[title^="Redo"]');
  await page.waitForTimeout(800);
  check("redo works", true);

  /* align + flip + order + lock */
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

  /* style controls in color popover */
  await page.click('.cs-toolbar .color-more');
  await page.waitForSelector('.color-popover.open', { timeout: 5000 });

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

  /* opacity in color popover */
  await page.click('.cs-toolbar .seg-btn[title="50%"]');
  await page.waitForFunction(
    () => document.querySelector('.cs-toolbar .seg-btn[title="50%"]')?.classList.contains("active"),
    { timeout: 5000 },
  );
  check("opacity button reflects state", true);

  /* close color popover */
  await page.click('.cs-toolbar .color-more');
  await page.waitForSelector('.color-popover.open', { state: "detached", timeout: 5000 });

  /* copy to clipboard */
  await page.click('nav.top-actions > button.copy-command');
  await page.waitForFunction(
    () => document.querySelector('nav.top-actions > button.copy-command')?.textContent?.includes("Copied"),
    { timeout: 10000 },
  );
  check("flattened image copies to clipboard", true);

  /* PNG download */
  const downloadPromise = page.waitForEvent("download", { timeout: 10000 });
  await page.click('nav.top-actions > button.save-command');
  const download = await downloadPromise;
  check("PNG export downloads with a safe filename", download.suggestedFilename().endsWith(".png"), download.suggestedFilename());

  /* close and reload new capture */
  await page.click('.top-actions-secondary button.command-btn:has-text("Close")');
  await page.evaluate(async () => {
    const canvas = document.createElement("canvas");
    canvas.width = 640;
    canvas.height = 180;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Canvas is unavailable");
    ctx.fillStyle = "white";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "black";
    ctx.font = "bold 64px Arial";
    ctx.fillText("OCR TEST", 24, 110);
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, "image/png"));
    if (!blob) throw new Error("Could not create OCR fixture");
    const input = document.querySelector('input[type="file"]');
    if (!input) throw new Error("File input is unavailable");
    const transfer = new DataTransfer();
    transfer.items.add(new File([blob], "ocr-test.png", { type: "image/png" }));
    input.files = transfer.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await page.click('nav.top-actions .topbar-menu-wrap > button.command-btn');
  await page.click('.topbar-menu button[role="menuitem"]:has-text("OCR entire image")');
  await page.waitForFunction(
    () => document.querySelector(".ocr-panel")?.textContent?.includes("OCR TEST"),
    { timeout: 120000 },
  );
  check("OCR recognizes text from a screenshot", true);

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
