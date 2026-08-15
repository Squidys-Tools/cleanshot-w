import { cp, mkdir } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { pipeline } from "node:stream/promises";
import path from "node:path";

const dest = path.resolve("public/tessdata");
const workerSrc = path.resolve("node_modules/tesseract.js/dist/worker.min.js");
const coreSrc = path.resolve("node_modules/tesseract.js-core");
const engUrl =
  "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main/eng.traineddata";

await mkdir(dest, { recursive: true });

await cp(workerSrc, path.join(dest, "worker.min.js"));

const coreFiles = await import("node:fs/promises").then((m) =>
  m.readdir(coreSrc),
);
for (const file of coreFiles) {
  if (file.startsWith("tesseract-core") && file.endsWith(".js")) {
    await cp(path.join(coreSrc, file), path.join(dest, file));
  }
  if (file.startsWith("tesseract-core") && file.endsWith(".wasm")) {
    await cp(path.join(coreSrc, file), path.join(dest, file));
  }
}

const engPath = path.join(dest, "eng.traineddata");
const res = await fetch(engUrl);
if (!res.ok || !res.body) throw new Error(`fetch failed: ${res.status}`);
await pipeline(res.body, createWriteStream(engPath));

const size = (await import("node:fs/promises")).stat(engPath).then((s) => s.size);
console.log(
  `tessdata ready in ${dest}: worker, ${coreFiles.filter((f) => f.startsWith("tesseract-core")).length} core files, eng.traineddata (${(await size / 1024).toFixed(0)} KiB)`,
);
