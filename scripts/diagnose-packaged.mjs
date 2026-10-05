#!/usr/bin/env node
/**
 * Attach to a PACKAGED (release-mode) CleanShot W build over the WebView2
 * Chrome DevTools Protocol and report why the editor canvas is or is not
 * painting.
 *
 * Release builds ship without devtools, so the M2 release gate could only
 * observe that the canvas was blank, never why. This harness re-enables CDP
 * for the packaged process, captures every console message and uncaught
 * exception, then measures the real geometry of the tldraw container chain.
 *
 * Usage:
 *   node scripts/diagnose-packaged.mjs --exe "path\to\CleanShot W.exe"
 *   node scripts/diagnose-packaged.mjs --exe <path> --port 9333 --settle 9000 --out report.json
 *
 * Requires Node 18+ (global fetch) and Node 22+ (global WebSocket).
 * See docs/ENGINEERING.md for why CI cannot cover this path.
 */

import { spawn } from "node:child_process";
import { writeFileSync } from "node:fs";
import { setTimeout as sleep } from "node:timers/promises";

function parseArgs(argv) {
  const out = { port: 9222, settle: 6000, boot: 45000, out: null, urlFilter: "tauri.localhost", openCapture: null, afterOpen: 7000 };
  for (let i = 2; i < argv.length; i += 1) {
    const key = argv[i];
    const next = argv[i + 1];
    if (key === "--exe") { out.exe = next; i += 1; }
    else if (key === "--port") { out.port = Number(next); i += 1; }
    else if (key === "--settle") { out.settle = Number(next); i += 1; }
    else if (key === "--boot") { out.boot = Number(next); i += 1; }
    else if (key === "--out") { out.out = next; i += 1; }
    else if (key === "--url-filter") { out.urlFilter = next; i += 1; }
    else if (key === "--open-capture") { out.openCapture = next; i += 1; }
    else if (key === "--after-open") { out.afterOpen = Number(next); i += 1; }
  }
  if (!out.exe) {
    console.error("error: --exe <path to packaged executable> is required");
    process.exit(2);
  }
  return out;
}

/* Chromium flags we ask WebView2 to honour. --remote-debugging-port is the
   whole point; the rest are inert defaults that make the session reproducible
   and are documented in docs/ENGINEERING.md. */
function browserArgs(port) {
  const existing = process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS || "";
  const flags = [
    `--remote-debugging-port=${port}`,
    "--remote-allow-origins=*",
  ];
  return [existing, ...flags].filter(Boolean).join(" ").trim();
}

async function fetchJson(url, timeoutMs = 2000) {
  const res = await fetch(url, { signal: AbortSignal.timeout(timeoutMs) });
  if (!res.ok) throw new Error(`${url} -> HTTP ${res.status}`);
  return res.json();
}

/* WebView2 exposes the app document as a page target. Poll until it appears;
   the endpoint is not listening until the runtime has booted. */
async function waitForTarget(port, urlFilter, bootMs) {
  const deadline = Date.now() + bootMs;
  let lastErr = "no attempt made";
  while (Date.now() < deadline) {
    try {
      const list = await fetchJson(`http://127.0.0.1:${port}/json/list`);
      const page = list.find((t) => t.type === "page" && String(t.url).includes(urlFilter));
      if (page?.webSocketDebuggerUrl) return page;
      lastErr = `targets: ${JSON.stringify(list.map((t) => ({ type: t.type, url: t.url })))}`;
    } catch (err) {
      lastErr = String(err?.message || err);
    }
    await sleep(400);
  }
  throw new Error(`no CDP page target matching "${urlFilter}" within ${bootMs}ms. Last: ${lastErr}`);
}

/* Minimal CDP client: enough to subscribe to diagnostics and evaluate a probe. */
class Cdp {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pending = new Map();
    this.events = [];
    ws.addEventListener("message", (ev) => {
      let msg;
      try { msg = JSON.parse(ev.data); } catch { return; }
      if (msg.id !== undefined && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error.message || JSON.stringify(msg.error)));
        else resolve(msg.result);
        return;
      }
      if (msg.method) this.events.push(msg);
    });
  }

  static async connect(wsUrl) {
    const ws = new WebSocket(wsUrl);
    await new Promise((resolve, reject) => {
      ws.addEventListener("open", resolve, { once: true });
      ws.addEventListener("error", () => reject(new Error(`websocket error: ${wsUrl}`)), { once: true });
    });
    return new Cdp(ws);
  }

  send(method, params = {}) {
    this.id += 1;
    const id = this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  /* Evaluate an expression in the page and return its value by value. */
  async eval(expression) {
    const res = await this.send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
      allowUnsafeEvalBlockedByCSP: false,
    });
    if (res.exceptionDetails) {
      throw new Error(res.exceptionDetails.exception?.description || JSON.stringify(res.exceptionDetails));
    }
    return res.result?.value;
  }

  close() {
    try { this.ws.close(); } catch { /* already closing */ }
  }
}

/* Runs inside the packaged app. Reports the geometry of the tldraw container
   chain, because a zero-sized ancestor is indistinguishable from a broken
   canvas when you can only look at pixels. */
const PROBE = `(() => {
  const round = (n) => (Number.isFinite(n) ? Math.round(n * 100) / 100 : String(n));
  const describe = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return {
      rect: { x: round(r.x), y: round(r.y), w: round(r.width), h: round(r.height) },
      offset: { w: el.offsetWidth, h: el.offsetHeight },
      client: { w: el.clientWidth, h: el.clientHeight },
      scroll: { w: el.scrollWidth, h: el.scrollHeight },
      /* Backing-store size vs painted size. A mismatch here is the classic
         signature of a scaling or DPR fault that leaves the canvas unpainted. */
      backing: el.tagName === "CANVAS"
        ? { w: el.width, h: el.height, attrW: el.getAttribute("width"), attrH: el.getAttribute("height") }
        : null,
      style: {
        display: cs.display, position: cs.position, visibility: cs.visibility,
        opacity: cs.opacity, transform: cs.transform, overflow: cs.overflow,
        zIndex: cs.zIndex, contain: cs.contain, contentVisibility: cs.contentVisibility,
        willChange: cs.willChange, filter: cs.filter, mixBlendMode: cs.mixBlendMode,
      },
      inDocument: el.isConnected,
    };
  };

  /* Walk ancestors of the tldraw container and flag any that collapse to zero. */
  const chain = [];
  let node = document.querySelector(".cs-tldraw");
  while (node) {
    const r = node.getBoundingClientRect();
    chain.push({
      tag: node.tagName.toLowerCase(),
      id: node.id || null,
      cls: (node.className && String(node.className).slice(0, 90)) || null,
      w: round(r.width), h: round(r.height),
      zeroSized: r.width === 0 || r.height === 0,
    });
    node = node.parentElement;
  }

  let webgl = "unavailable";
  try {
    const c = document.createElement("canvas");
    webgl = c.getContext("webgl2") ? "webgl2" : c.getContext("webgl") ? "webgl" : "none";
  } catch (err) { webgl = "threw: " + String(err && err.message); }

  const tldrawCanvases = Array.from(document.querySelectorAll(".tl-canvas, .tl-html-layer, canvas"))
    .slice(0, 8)
    .map((el, i) => ({ index: i, cls: String(el.className || "").slice(0, 70), ...describe(el) }));

  return {
    url: location.href,
    viewport: {
      innerWidth: window.innerWidth, innerHeight: window.innerHeight,
      outerWidth: window.outerWidth, outerHeight: window.outerHeight,
      devicePixelRatio: window.devicePixelRatio,
      screen: { w: screen.width, h: screen.height, availW: screen.availWidth, availH: screen.availHeight },
    },
    document: {
      hidden: document.hidden, visibilityState: document.visibilityState, hasFocus: document.hasFocus(),
    },
    counts: {
      csTldraw: document.querySelectorAll(".cs-tldraw").length,
      tlContainer: document.querySelectorAll(".tl-container").length,
      tlViewport: document.querySelectorAll(".tl-viewport").length,
      tlCanvas: document.querySelectorAll(".tl-canvas").length,
      csUi: document.querySelectorAll(".cs-ui").length,
      canvases: document.querySelectorAll("canvas").length,
      images: document.querySelectorAll("img").length,
    },
    chain,
    csTldraw: describe(document.querySelector(".cs-tldraw")),
    tlContainer: describe(document.querySelector(".tl-container")),
    tlViewport: describe(document.querySelector(".tl-viewport")),
    csUi: describe(document.querySelector(".cs-ui")),
    tldrawCanvases,
    webgl,
    statusBarText: (document.querySelector(".cs-statusbar, .cs-status") || {}).textContent
      ? String((document.querySelector(".cs-statusbar, .cs-status")).textContent).trim().slice(0, 200)
      : null,
  };
})()`;

function summariseEvents(events) {
  const console_ = [];
  const exceptions = [];
  const logEntries = [];
  for (const ev of events) {
    if (ev.method === "Runtime.consoleAPICalled") {
      const p = ev.params || {};
      console_.push({
        type: p.type,
        text: (p.args || []).map((a) => a.value ?? a.description ?? a.unserializableValue ?? a.type).join(" "),
      });
    } else if (ev.method === "Runtime.exceptionThrown") {
      const d = ev.params?.exceptionDetails || {};
      exceptions.push({
        text: d.text,
        message: d.exception?.description || d.exception?.value || null,
        url: d.url || null,
        line: d.lineNumber ?? null,
      });
    } else if (ev.method === "Log.entryAdded") {
      const e = ev.params?.entry || {};
      logEntries.push({ level: e.level, source: e.source, text: e.text, url: e.url || null });
    }
  }
  return { console: console_, exceptions, logEntries };
}

async function main() {
  const args = parseArgs(process.argv);
  const flags = browserArgs(args.port);
  const report = {
    startedAt: new Date().toISOString(),
    exe: args.exe,
    port: args.port,
    webview2Args: flags,
    target: null,
    probe: null,
    diagnostics: null,
    error: null,
  };

  console.log(`[diagnose] exe        : ${args.exe}`);
  console.log(`[diagnose] CDP port   : ${args.port}`);
  console.log(`[diagnose] webview2   : ${flags}`);

  /* Single-instance means a leftover process from a previous run silently
     absorbs this launch, and then CDP attaches to THAT window with whatever
     UI state it was left in. Every symptom then looks like a bug in the app.
     Refuse to run unless we are starting the only instance. */
  if (args.force) {
    try {
      const { execFileSync } = await import("node:child_process");
      execFileSync("taskkill", ["/F", "/IM", `${args.exe.split(/[\\/]/).pop()}`, "/T"], {
        stdio: "ignore",
      });
      console.log("[diagnose] killed a leftover instance (--force)");
      await sleep(1500);
    } catch {
      /* nothing was running, which is the normal case */
    }
  }

  const child = spawn(args.exe, [], {
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: flags },
    stdio: ["ignore", "pipe", "pipe"],
  });
  const appOutput = [];
  child.stdout?.on("data", (d) => appOutput.push(String(d)));
  child.stderr?.on("data", (d) => appOutput.push(String(d)));
  report.pid = child.pid;

  let cdp = null;
  try {
    const target = await waitForTarget(args.port, args.urlFilter, args.boot);
    report.target = { id: target.id, type: target.type, title: target.title, url: target.url };
    console.log(`[diagnose] attached   : ${target.url}`);

    cdp = await Cdp.connect(target.webSocketDebuggerUrl);
    await cdp.send("Runtime.enable");
    await cdp.send("Log.enable");
    await cdp.send("Page.enable");

    console.log(`[diagnose] listening ${args.settle}ms for console activity...`);
    await sleep(args.settle);

    /* The editor only mounts once a capture is open, so drive it open first.
       Clicking the history entry is what a user does, and it exercises the same
       path the release gate did. */
    if (args.openCapture) {
      console.log(`[diagnose] opening capture: ${args.openCapture}`);
      try {
        /* Report what is actually on screen first. "no-history-entry" is only
           useful alongside the DOM that produced it. */
        const seen = await cdp.eval(
          "(() => {" +
          '  const items = Array.from(document.querySelectorAll(".history-item"));' +
          '  const empties = Array.from(document.querySelectorAll(".history-empty")).map((e) => e.textContent);' +
          "  return JSON.stringify({" +
          "    historyItems: items.length," +
          "    historyTitles: items.map((e) => (e.textContent || '').trim().slice(0, 60))," +
          "    historyEmpty: empties," +
          "    railPresent: !!document.querySelector('.history')," +
          "    flyoutPresent: !!document.querySelector('.history-flyout')," +
          "    bodyText: (document.body.innerText || '').replace(/\\s+/g, ' ').slice(0, 300)," +
          "  });" +
          "})()",
        );
        console.log(`[diagnose] dom: ${seen}`);
        report.domBeforeOpen = seen;

        /* History lives in a flyout that is closed by default, so open it
           first. The rail is only mounted once the flyout is visible. */
        const opened = await cdp.eval(
          "(() => {" +
          '  const want = ' + JSON.stringify(args.openCapture) + ";" +
          "  const byText = (re) => Array.from(document.querySelectorAll('button, [role=\"button\"], .topbar-btn, .cs-btn'))" +
          "    .find((el) => re.test((el.textContent || '').trim()));" +
          "  const histBtn = byText(/^History/);" +
          "  if (!histBtn) return 'no-history-button';" +
          "  histBtn.click();" +
          "  return 'opened-flyout:' + histBtn.textContent.trim();" +
          "})()",
        );
        console.log(`[diagnose] flyout: ${opened}`);
        await sleep(1500);

        const clicked = await cdp.eval(
          "(() => {" +
          '  const want = ' + JSON.stringify(args.openCapture) + ";" +
          '  const items = Array.from(document.querySelectorAll(".history-item"));' +
          "  if (!items.length) return 'no-history-items';" +
          "  const hit = items.find((el) => (el.textContent || '').includes(want)) || items[0];" +
          "  hit.click();" +
          "  return 'clicked:' + (hit.textContent || '').trim().slice(0, 40);" +
          "})()",
        );
        console.log(`[diagnose] open result: ${clicked}`);
        await sleep(args.afterOpen ?? 7000);
      } catch (err) {
        console.error(`[diagnose] could not drive capture open: ${err?.message || err}`);
      }
    }

    report.diagnostics = summariseEvents(cdp.events);
    try {
      report.probe = await cdp.eval(PROBE);
    } catch (err) {
      report.probeError = String(err?.message || err);
    }
  } catch (err) {
    report.error = String(err?.message || err);
    console.error(`[diagnose] FAILED: ${report.error}`);
  } finally {
    cdp?.close();
    report.appOutput = appOutput.join("").slice(0, 8000);
    try { child.kill(); } catch { /* already gone */ }
  }

  const json = JSON.stringify(report, null, 2);
  if (args.out) {
    writeFileSync(args.out, json);
    console.log(`[diagnose] wrote ${args.out}`);
  }

  /* Human summary. */
  const d = report.diagnostics;
  if (d) {
    console.log("\n=== console ==================================================");
    for (const c of d.console.slice(0, 40)) console.log(`  [${c.type}] ${c.text}`);
    console.log("=== uncaught exceptions ======================================");
    if (!d.exceptions.length) console.log("  (none)");
    for (const e of d.exceptions) console.log(`  ${e.text}: ${e.message || ""} (${e.url || "?"}:${e.line ?? "?"})`);
    console.log("=== log entries ==============================================");
    for (const l of d.logEntries.slice(0, 30)) console.log(`  [${l.level}/${l.source}] ${l.text}`);
  }
  const p = report.probe;
  if (p) {
    console.log("\n=== viewport ================================================");
    console.log(`  ${p.viewport.innerWidth}x${p.viewport.innerHeight} @dpr=${p.viewport.devicePixelRatio} hidden=${p.document.hidden} focus=${p.document.hasFocus} webgl=${p.webgl}`);
    console.log("=== ancestor chain of .cs-tldraw (outermost first) =============");
    for (const n of [...p.chain].reverse()) {
      console.log(`  ${n.zeroSized ? "ZERO-SIZED" : "         "} ${n.tag}${n.id ? "#" + n.id : ""} .${n.cls || ""} -> ${n.w}x${n.h}`);
    }
    console.log("=== tldraw nodes =============================================");
    for (const [k, v] of Object.entries({ "cs-tldraw": p.csTldraw, "tl-container": p.tlContainer, "tl-viewport": p.tlViewport, "cs-ui": p.csUi })) {
      if (!v) { console.log(`  ${k}: ABSENT`); continue; }
      const bad = v.zeroSized ? " <<< ZERO-SIZED" : "";
      console.log(`  ${k}: rect ${v.rect.w}x${v.rect.h} offset ${v.offset.w}x${v.offset.h} pos=${v.style.position} vis=${v.style.visibility} op=${v.style.opacity}${bad}`);
    }
    for (const c of p.tldrawCanvases) {
      console.log(`  canvas[${c.index}] .${c.cls}: rect ${c.rect.w}x${c.rect.h} backing ${c.backing ? c.backing.w + "x" + c.backing.h : "n/a"} offset ${c.offset.w}x${c.offset.h}`);
    }
    if (p.statusBarText) console.log(`  statusBar: ${JSON.stringify(p.statusBarText)}`);
  }

  const failed = Boolean(report.error) || Boolean((d?.exceptions || []).length);
  console.log(`\n[diagnose] done. ${failed ? "PROBLEMS FOUND (see above)" : "no exceptions observed"}`);
  if (args.out) process.exit(failed ? 1 : 0);
}

main().catch((err) => {
  console.error("[diagnose] fatal:", err);
  process.exit(1);
});
