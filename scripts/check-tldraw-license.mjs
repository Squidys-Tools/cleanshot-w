#!/usr/bin/env node
/**
 * Fail loudly when a production build has no tldraw license key.
 *
 * tldraw does not throw when it is unlicensed. Five seconds after the editor
 * mounts, `LicenseProvider` swaps the whole editor for an invisible
 * `<div data-testid="tl-license-expired" style="display:none">`. The canvas, the
 * annotation dock, and the status bar all disappear, the rest of the app keeps
 * working, and nothing is written to the console except a styled banner. That
 * is release-gate Finding 1, and it cost a full manual gate pass to diagnose.
 *
 * It only happens in a production build. `LicenseManager.getIsDevelopment()`
 * treats loopback and `*.localhost` hosts as development unless
 * `process.env.NODE_ENV === "production"`, so `bun run dev` and the
 * headless-Chromium smoke suite render the editor happily while the packaged
 * Windows build goes blank. A check that only runs in CI therefore misses it,
 * which is why this script exists.
 *
 * usage: node scripts/check-tldraw-license.mjs [--warn-only]
 * exit 0 when a key is present or --warn-only was passed, 1 otherwise.
 */

const VARS = [
  "TLDRAW_LICENSE_KEY",
  "VITE_TLDRAW_LICENSE_KEY",
  "NEXT_PUBLIC_TLDRAW_LICENSE_KEY",
  "REACT_APP_TLDRAW_LICENSE_KEY",
  "GATSBY_TLDRAW_LICENSE_KEY",
  "PUBLIC_TLDRAW_LICENSE_KEY",
];

const warnOnly = process.argv.includes("--warn-only");

/* Vite inlines VITE_-prefixed variables at build time, so a plain TLDRAW_LICENSE_KEY
   is only useful when the bundler is configured to forward it. */
const present = VARS.filter((name) => typeof process.env[name] === "string" && process.env[name].trim().length > 0);

if (present.length > 0) {
  console.log(`[license] tldraw license key present (${present.join(", ")}).`);
  process.exit(0);
}

const message = [
  "",
  "  No tldraw license key is set for this build.",
  "",
  "  A production build without one renders a blank editor: about five seconds",
  "  after a capture opens, tldraw hides the canvas, the dock, and the status bar",
  "  and shows nothing. Development builds are unaffected, so browser checks pass.",
  "",
  "  Set one of these in the build environment:",
  ...VARS.map((v) => `    ${v}=...`),
  "",
  "  See docs/ROADMAP.md for the recorded gate run and docs/ENGINEERING.md for",
  "  the licensing requirement.",
  "",
].join("\n");

if (warnOnly) {
  /* One line, so a routine PR run does not look like a failure. The long
     explanation belongs in the blocking path, where someone has to act. */
  console.warn("[license] no tldraw license key configured; see docs/ENGINEERING.md");
  process.exit(0);
}

console.error(message);
process.exit(1);
