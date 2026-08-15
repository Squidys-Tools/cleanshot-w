#!/usr/bin/env bun
import { mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { loadManifest, corpusPath, benchmarksRoot } from "./manifest";
import { startCorpusServer } from "./server";
import { extractFixture } from "./extract";
import { extractPdf, extractFirstEmbeddedJpeg } from "./pdf";
import { pickOcrEngine, WindowsOcrEngine, TesseractJsEngine, type OcrEngine } from "./ocr";
import { average, bannedTermHits, stringScore, termMatches, tokenStats, tokenize } from "./score";
import type { ManifestItem, ExpectedSearch } from "./manifest";
import type { ExtractionOutcome } from "./extract";

type Status = "pass" | "partial" | "fail" | "skip";

interface ItemResult {
  id: string;
  type: string;
  engine: string;
  status: Status;
  score: number;
  metrics: Record<string, unknown>;
  messages: string[];
}

const EXTRACTION_TYPES = new Set(["article", "recipe", "video"]);

function statusFor(score: number): Status {
  if (score === 1) return "pass";
  if (score >= 0.6) return "partial";
  return "fail";
}

function skipResult(item: ManifestItem, reason: string): ItemResult {
  return {
    id: item.id,
    type: item.type,
    engine: "none",
    status: "skip",
    score: 0,
    metrics: { reason },
    messages: [reason],
  };
}

function scoreExtraction(item: ManifestItem, outcome: ExtractionOutcome): ItemResult {
  const exp = item.expected ?? {};
  const messages: string[] = [];

  if (!outcome.ok) {
    return {
      id: item.id,
      type: item.type,
      engine: outcome.engine,
      status: "fail",
      score: 0,
      metrics: { errorCode: outcome.errorCode, errorDetail: outcome.errorDetail },
      messages: [`ingestion failed (${outcome.errorCode}): ${outcome.errorDetail}`],
    };
  }

  const parts: number[] = [];
  const searchable = `${outcome.title ?? ""} ${outcome.description ?? ""} ${outcome.text ?? ""}`;
  if (exp.title) {
    const value = stringScore(exp.title, outcome.title ?? "");
    parts.push(value);
    if (value < 1) messages.push(`title mismatch: expected "${exp.title}", got "${outcome.title}"`);
  }
  if (exp.author) {
    const value = stringScore(exp.author, outcome.author ?? "");
    parts.push(value);
    if (value < 1) messages.push(`author mismatch: expected "${exp.author}", got "${outcome.author}"`);
  }
  if (exp.search_terms?.length) {
    const matches = termMatches(searchable, exp.search_terms);
    parts.push(matches.filter((m) => m.found).length / matches.length);
    for (const m of matches.filter((m) => !m.found)) messages.push(`search term not found: "${m.term}"`);
  }
  const banned = bannedTermHits(searchable, exp.must_not_match ?? []);
  for (const term of banned) messages.push(`banned term found in extracted text: "${term}"`);
  if (banned.length) parts.push(0);

  if (exp.image_urls_min != null) {
    const value = Math.min(1, outcome.imageUrls.length / exp.image_urls_min);
    parts.push(value);
    if (value < 1) messages.push(`expected >= ${exp.image_urls_min} images, got ${outcome.imageUrls.length}`);
  }
  if (exp.safe_embeds?.length) {
    for (const req of exp.safe_embeds) {
      const count = outcome.safeEmbeds.filter((e) => e.provider === req.provider).length;
      const min = req.min ?? 1;
      const value = Math.min(1, count / min);
      parts.push(value);
      if (value < 1) messages.push(`expected >= ${min} "${req.provider}" embed(s), got ${count}`);
    }
  }
  if (exp.safe_embeds_max != null && outcome.safeEmbeds.length > exp.safe_embeds_max) {
    parts.push(0);
    messages.push(
      `expected <= ${exp.safe_embeds_max} embed(s), got ${outcome.safeEmbeds.length} (${outcome.safeEmbeds.map((e) => e.provider).join(", ")})`,
    );
  }

  const score = average(parts);
  return {
    id: item.id,
    type: item.type,
    engine: outcome.engine,
    status: statusFor(score),
    score,
    metrics: {
      title: outcome.title,
      author: outcome.author,
      imageUrls: outcome.imageUrls.length,
      safeEmbeds: outcome.safeEmbeds.length,
      textLength: (outcome.text ?? "").length,
      extractor: outcome.extractor,
    },
    messages,
  };
}

function scorePlainText(item: ManifestItem, text: string, engine: string): ItemResult {
  const exp = item.expected ?? {};
  const messages: string[] = [];
  const parts: number[] = [];
  if (exp.search_terms?.length) {
    const matches = termMatches(text, exp.search_terms);
    parts.push(matches.filter((m) => m.found).length / matches.length);
    for (const m of matches.filter((m) => !m.found)) messages.push(`search term not found: "${m.term}"`);
  }
  const banned = bannedTermHits(text, exp.must_not_match ?? []);
  for (const term of banned) messages.push(`banned term found: "${term}"`);
  if (banned.length) parts.push(0);

  const score = average(parts);
  return {
    id: item.id,
    type: item.type,
    engine,
    status: statusFor(score),
    score,
    metrics: { textLength: text.length },
    messages,
  };
}

function scorePdf(
  item: ManifestItem,
  naive: { text: string; embeddedImages: number },
  ocrText: string | null,
  ocrEngine: string | null,
): ItemResult {
  const exp = item.expected ?? {};
  const messages: string[] = [];
  const parts: number[] = [];
  const text = naive.text || ocrText || "";

  if (exp.search_terms?.length) {
    const matches = termMatches(text, exp.search_terms);
    parts.push(matches.filter((m) => m.found).length / matches.length);
    for (const m of matches.filter((m) => !m.found)) messages.push(`search term not found: "${m.term}"`);
  }
  const banned = bannedTermHits(text, exp.must_not_match ?? []);
  for (const term of banned) messages.push(`banned term found: "${term}"`);
  if (banned.length) parts.push(0);

  if (exp.embedded_images_min != null) {
    const value = Math.min(1, naive.embeddedImages / exp.embedded_images_min);
    parts.push(value);
    if (value < 1) messages.push(`expected >= ${exp.embedded_images_min} embedded image(s), got ${naive.embeddedImages}`);
  }

  const score = average(parts);
  return {
    id: item.id,
    type: item.type,
    engine: naive.text ? "naive-streams" : ocrEngine ?? "none",
    status: statusFor(score),
    score,
    metrics: {
      naiveTextLength: naive.text.length,
      ocrText: ocrText != null,
      embeddedImages: naive.embeddedImages,
    },
    messages,
  };
}

function scoreOcr(item: ManifestItem, expectedText: string, actualText: string, engine: string): ItemResult {
  const stats = tokenStats(expectedText, actualText);
  const score = (stats.recall + stats.precision) / 2;
  const messages: string[] = [];
  if (stats.recall < 1) {
    const missing = [...new Set(tokenize(expectedText))].filter((t) => !tokenize(actualText).includes(t));
    messages.push(`OCR recall ${stats.recall.toFixed(2)} (${stats.expectedCount} expected tokens)`);
    messages.push(`missing tokens (sample): ${missing.slice(0, 8).join(", ")}`);
  }
  if (stats.precision < 1) messages.push(`OCR precision ${stats.precision.toFixed(2)}`);
  return {
    id: item.id,
    type: item.type,
    engine,
    status: statusFor(score),
    score,
    metrics: stats,
    messages,
  };
}

async function tryOcr(engine: OcrEngine | null, filePath: string): Promise<{ text: string; engine: string } | null> {
  if (!engine) return null;
  try {
    const text = await engine.run(filePath);
    return { text, engine: engine.name };
  } catch (cause) {
    console.warn(`  [ocr] ${engine.name} failed for ${filePath}: ${cause instanceof Error ? cause.message : String(cause)}`);
    return null;
  }
}

async function runItem(
  item: ManifestItem,
  baseUrl: string,
  ocrEngine: OcrEngine | null,
): Promise<ItemResult> {
  const exp: ExpectedSearch = item.expected ?? {};
  const filePath = corpusPath(item.path);

  if (EXTRACTION_TYPES.has(item.type)) {
    const outcome = await extractFixture(baseUrl, item.path);
    return scoreExtraction(item, outcome);
  }

  if (item.type === "note" || item.type === "quote") {
    const text = readFileSync(filePath, "utf8");
    return scorePlainText(item, text, "raw-file");
  }

  if (item.type === "pdf") {
    const naive = extractPdf(filePath);
    let ocrText: string | null = null;
    if (!naive.text && exp.ocr_text_file) {
      const jpeg = extractFirstEmbeddedJpeg(filePath);
      if (jpeg && ocrEngine) {
        const tmpFile = join(tmpdir(), `mymind-ocr-${item.id}.jpg`);
        writeFileSync(tmpFile, jpeg);
        try {
          const result = await tryOcr(ocrEngine, tmpFile);
          ocrText = result?.text ?? null;
        } finally {
          rmSync(tmpFile, { force: true });
        }
      }
    }
    if (!naive.text && !ocrText && exp.ocr_text_file) {
      return skipResult(item, "scanned PDF: no OCR engine available (Windows.Media.Ocr or tesseract.js)");
    }
    return scorePdf(item, naive, ocrText, ocrEngine?.name ?? null);
  }

  if (item.type === "image" || item.type === "screenshot") {
    if (exp.ocr_text_file) {
      const result = await tryOcr(ocrEngine, filePath);
      if (!result) return skipResult(item, "no OCR engine available (Windows.Media.Ocr or tesseract.js)");
      const expectedText = readFileSync(corpusPath(exp.ocr_text_file), "utf8");
      return scoreOcr(item, expectedText, result.text, result.engine);
    }
    return skipResult(item, "vision/similarity scoring not implemented (embeddings benchmark)");
  }

  return skipResult(item, `unknown item type "${item.type}"`);
}

interface Summary {
  total: number;
  pass: number;
  partial: number;
  fail: number;
  skip: number;
  avgScore: number;
}

function summarize(results: ItemResult[], types: Set<string>): Summary {
  const scorable = results.filter((r) => r.status !== "skip");
  return {
    total: results.length,
    pass: results.filter((r) => r.status === "pass").length,
    partial: results.filter((r) => r.status === "partial").length,
    fail: results.filter((r) => r.status === "fail").length,
    skip: results.filter((r) => r.status === "skip").length,
    avgScore: scorable.length ? scorable.reduce((sum, r) => sum + r.score, 0) / scorable.length : 0,
  };
}

function writeSummaryMarkdown(report: {
  generatedAt: string;
  overall: Summary;
  byType: Record<string, Summary>;
  items: ItemResult[];
}): string {
  const lines: string[] = [];
  lines.push("# Benchmark results");
  lines.push(`Generated: ${report.generatedAt}`);
  lines.push("");
  lines.push(`## Overall: score ${report.overall.avgScore.toFixed(3)} — pass ${report.overall.pass}, partial ${report.overall.partial}, fail ${report.overall.fail}, skip ${report.overall.skip}`);
  lines.push("");
  lines.push("| Type | Score | Pass | Partial | Fail | Skip | Total |");
  lines.push("| --- | --- | --- | --- | --- | --- | --- |");
  for (const [type, s] of Object.entries(report.byType)) {
    lines.push(`| ${type} | ${s.avgScore.toFixed(3)} | ${s.pass} | ${s.partial} | ${s.fail} | ${s.skip} | ${s.total} |`);
  }
  lines.push("");
  lines.push("## Items");
  for (const r of report.items) {
    lines.push(`- [${r.status.toUpperCase()}] ${r.id} (score ${r.score.toFixed(3)}, ${r.engine})`);
  }
  lines.push("");
  const withMessages = report.items.filter((r) => r.messages.length > 0);
  if (withMessages.length) {
    lines.push("## Details");
    for (const r of withMessages) {
      lines.push(`### ${r.id} — ${r.status} (${r.score.toFixed(3)})`);
      for (const m of r.messages) lines.push(`- ${m}`);
      lines.push("");
    }
  }
  return lines.join("\n");
}

async function main(): Promise<void> {
  const manifest = loadManifest();
  const server = await startCorpusServer(corpusPath("."));

  console.log(`Loaded ${manifest.items.length} fixtures; corpus server at ${server.baseUrl}`);
  const ocrEngine = await pickOcrEngine([
    new WindowsOcrEngine(join(import.meta.dir, "win-ocr.ps1")),
    new TesseractJsEngine(),
  ]);
  console.log(`OCR engine: ${ocrEngine ? ocrEngine.name : "none (screenshots/scan OCR skipped)"}`);
  console.log("");

  const items: ItemResult[] = [];
  for (const item of manifest.items) {
    process.stdout.write(`  ${item.id} ... `);
    const result = await runItem(item, server.baseUrl, ocrEngine);
    items.push(result);
    console.log(`[${result.status}] score ${result.score.toFixed(3)} ${result.messages.length ? `(${result.messages[0]})` : ""}`);
  }
  server.stop();

  const byType: Record<string, Summary> = {};
  for (const item of manifest.items) {
    if (!byType[item.type]) byType[item.type] = { total: 0, pass: 0, partial: 0, fail: 0, skip: 0, avgScore: 0 };
  }
  for (const r of items) {
    const s = byType[r.type];
    s.total++;
    s[r.status]++;
    if (r.status !== "skip") {
      s.avgScore = (s.avgScore * (s.total - 1 - s.skip) + r.score) / Math.max(1, s.total - s.skip);
    }
  }

  const overall = summarize(items, new Set(manifest.items.map((i) => i.type)));
  const report = {
    generatedAt: new Date().toISOString(),
    engine: { ocr: ocrEngine?.name ?? null },
    overall,
    byType,
    items,
  };

  const resultsDir = join(benchmarksRoot, "results");
  mkdirSync(resultsDir, { recursive: true });
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  writeFileSync(join(resultsDir, "results-latest.json"), JSON.stringify(report, null, 2));
  writeFileSync(join(resultsDir, `results-${stamp}.json`), JSON.stringify(report, null, 2));
  writeFileSync(join(resultsDir, "summary.md"), writeSummaryMarkdown(report));

  console.log("");
  console.log(`Overall score: ${overall.avgScore.toFixed(3)} (pass ${overall.pass}, partial ${overall.partial}, fail ${overall.fail}, skip ${overall.skip})`);
  console.log(`Results written to benchmarks/results/`);
}

await main();
