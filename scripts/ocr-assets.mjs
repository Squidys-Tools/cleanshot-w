import { mkdir, stat } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { pipeline } from "node:stream/promises";
import path from "node:path";

const dest = path.resolve("ocr/dist/tessdata");
const engUrl =
  "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main/eng.traineddata";

await mkdir(dest, { recursive: true });

const engPath = path.join(dest, "eng.traineddata");
const res = await fetch(engUrl);
if (!res.ok || !res.body) throw new Error(`fetch failed: ${res.status}`);
await pipeline(res.body, createWriteStream(engPath));

const size = (await stat(engPath)).size;
console.log(`Tesseract English data ready in ${dest} (${(size / 1024).toFixed(0)} KiB)`);
