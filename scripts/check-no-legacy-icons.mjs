// Fails if any of the five retired icon packs (lucide, phosphor, tabler,
// heroicons, remix) reappears in the repo — reference or asset. HugeIcons is
// the only icon set (see mockups/snipsy-ui-redesign/icon-mapping.md).
//
// Skipped paths:
//   mockups/, docs/   design-thread territory; the mapping doc names the old
//                     packs on purpose
//   src/, src-tauri/, public/, tests/, bun.lock, package-lock.json,
//   package.json      the legacy Tauri app and its manifests; deleted
//                     wholesale by the cleanup PR that drops the dead app
//
// Usage: node scripts/check-no-legacy-icons.mjs

import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const FORBIDDEN = /lucide|phosphor|tabler|heroicons|remix/i;
const SKIP = [
  /^mockups\//,
  /^docs\//,
  /^src\//,
  /^src-tauri\//,
  /^public\//,
  /^tests\//,
  /^bun\.lock/,
  /^package-lock\.json$/,
  /^package\.json$/,
];

const tracked = spawnSync("git", ["ls-files"], { encoding: "utf8" });
if (tracked.error) throw tracked.error;
if (tracked.status !== 0) {
  console.error(tracked.stderr);
  process.exit(1);
}

const offenders = tracked.stdout
  .split("\n")
  .filter(Boolean)
  .filter((path) => !SKIP.some((skip) => skip.test(path)))
  .filter((path) => FORBIDDEN.test(path) || FORBIDDEN.test(readFileSync(path, "utf8")));

if (offenders.length > 0) {
  console.error(
    "Retired icon pack found — HugeIcons is the only icon set:\n" +
      offenders.map((path) => `  ${path}`).join("\n"),
  );
  process.exit(1);
}
console.log("No legacy icon packs found.");
