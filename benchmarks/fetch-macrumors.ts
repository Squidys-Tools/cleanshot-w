#!/usr/bin/env bun
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseHTML } from "linkedom";
import { extractJsonLdMetadata } from "../src/lib/ingestion/json-ld";
import { normalizeText } from "../src/lib/ingestion/url";
import { startCorpusServer } from "./harness/server";
import { extractFixture } from "./harness/extract";
import { loadManifest } from "./harness/manifest";

// This script lives in benchmarks/, so the benchmarks root is its own directory.
const benchmarksRoot = resolve(import.meta.dir);
const corpusDir = join(benchmarksRoot, "corpus", "live", "macrumors");
const expectedDir = join(benchmarksRoot, "expected", "extraction");
const manifestPath = join(benchmarksRoot, "manifest.json");
const HOMEPAGE = "https://www.macrumors.com/";
const USER_AGENT = "mymind-library/0.1 (+local article capture)";
const COUNT = Number(process.argv[2] ?? 6);

const STOPWORDS = new Set([
  "about", "after", "again", "also", "along", "among", "another", "around", "been", "before",
  "between", "both", "could", "from", "have", "into", "more", "most", "much", "other", "over",
  "said", "says", "some", "such", "that", "their", "them", "then", "there", "these", "they",
  "this", "those", "three", "through", "time", "until", "using", "what", "when", "which",
  "while", "with", "would", "your", "will",
]);

function urlJoin(...parts: string[]): string {
  return parts.join("/").replace(/\\/g, "/");
}

interface ArticleLink {
  url: string;
  date: string;
  slug: string;
}

async function fetchText(url: string): Promise<string> {
  const response = await fetch(url, { headers: { "User-Agent": USER_AGENT }, redirect: "follow" });
  if (!response.ok) throw new Error(`GET ${url} -> ${response.status}`);
  return await response.text();
}

function latestArticleLinks(html: string): ArticleLink[] {
  const { document } = parseHTML(html);
  const seen = new Set<string>();
  const links: ArticleLink[] = [];
  for (const anchor of document.querySelectorAll("a[href]")) {
    const href = anchor.getAttribute("href") ?? "";
    const pathname = href.replace(/^https?:\/\/[^/]+/, "");
    const match = /^\/(\d{4}\/\d{2}\/\d{2})\/([a-z0-9-]+)\/?$/.exec(pathname);
    if (!match || seen.has(href)) continue;
    seen.add(href);
    links.push({
      url: `https://www.macrumors.com${pathname.replace(/\/$/, "")}/`,
      date: match[1],
      slug: match[2],
    });
  }
  links.sort((a, b) => `${b.date}/${b.slug}`.localeCompare(`${a.date}/${a.slug}`));
  return links;
}

/** Distinctive terms picked from the article body (validates body extraction). */
function pickSearchTerms(description: string, bodyText: string, limit = 4): string[] {
  const counts = new Map<string, number>();
  const text = `${description} ${bodyText}`.toLowerCase();
  const words = text.replace(/[^a-z0-9\s-]/g, " ").split(/\s+/);
  for (const word of words) {
    const cleaned = word.trim();
    if (cleaned.length < 4 || STOPWORDS.has(cleaned)) continue;
    counts.set(cleaned, (counts.get(cleaned) ?? 0) + 1);
  }
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit)
    .map(([term]) => term);
}

function bylineAuthor(document: Document): string {
  for (const element of document.querySelectorAll('[class*="byline"], [class*="author"], [rel="author"]')) {
    const anchors = [...element.querySelectorAll("a")].map((a) => normalizeText(a.textContent)).filter(Boolean);
    for (const anchor of anchors) {
      if (/^[A-Z][\w'.-]+(?:\s+[A-Z][\w'.-]+){0,2}$/.test(anchor)) return anchor;
    }
    const text = normalizeText(element.textContent);
    const match = /\bby\s+([A-Z][\w'.-]+(?:\s+[A-Z][\w'.-]+)?)\b/i.exec(text);
    if (match) return match[1];
  }
  return "";
}

function buildExpected(html: string, url: string, article: ArticleLink) {
  const { document } = parseHTML(html);
  const structured = extractJsonLdMetadata(document, url);
  const ogTitle = document.querySelector('meta[property="og:title"]')?.getAttribute("content");
  const ogDescription = document.querySelector('meta[property="og:description"]')?.getAttribute("content");
  const bodyText = normalizeText(document.querySelector("article")?.textContent ?? "");
  const title = normalizeText(structured.title || ogTitle || "");
  const author = normalizeText(structured.author || bylineAuthor(document));
  const description = normalizeText(structured.description || ogDescription || "");
  return {
    title,
    author,
    description,
    publishedDate: structured.publishedDate ?? null,
    sourceUrl: url,
    fetchedAt: new Date().toISOString(),
    id: `article-live-macrumors-${article.slug}`,
    file: `corpus/live/macrumors/${article.date.replaceAll("/", "-")}-${article.slug}.html`,
    searchTerms: pickSearchTerms(description, bodyText),
  };
}

async function main(): Promise<void> {
  console.log(`Fetching MacRumors homepage...`);
  const homepage = await fetchText(HOMEPAGE);
  const candidates = latestArticleLinks(homepage);
  console.log(`Found ${candidates.length} article links on the homepage.`);

  const existing = new Set(
    loadManifest()
      .items
      .filter((item) => item.id.startsWith("article-live-macrumors-"))
      .map((item) => item.path.split("/").pop()),
  );
  const chosen = candidates.filter((candidate) => !existing.has(`${candidate.date.replaceAll("/", "-")}-${candidate.slug}.html`)).slice(0, COUNT);
  console.log(`Fetching ${chosen.length} latest article(s).`);

  mkdirSync(corpusDir, { recursive: true });
  mkdirSync(expectedDir, { recursive: true });

  const newItems: Record<string, unknown>[] = [];
  const verified: { id: string; ok: boolean; score: number }[] = [];

  for (const article of chosen) {
    const id = `article-live-macrumors-${article.slug}`;
    const file = `corpus/live/macrumors/${article.date.replaceAll("/", "-")}-${article.slug}.html`;
    console.log(`  ${id} <- ${article.url}`);
    const html = await fetchText(article.url);
    const expected = buildExpected(html, article.url, article);
    if (!expected.title || !expected.author) {
      console.warn(`    WARN: no title/author in JSON-LD for ${article.url}; skipping`);
      continue;
    }
    writeFileSync(join(corpusDir, file.split("/").pop()!), html);
    writeFileSync(
      join(expectedDir, `${id}.json`),
      JSON.stringify(expected, null, 2) + "\n",
    );
    newItems.push({
      id,
      path: file,
      type: "article",
      language: "en",
      expected: {
        title: expected.title,
        author: expected.author,
        search_terms: expected.searchTerms,
      },
      notes: `Live MacRumors article (fetched ${expected.fetchedAt.slice(0, 10)}); ${expected.sourceUrl}`,
    });
  }

  if (newItems.length === 0) {
    console.log("Nothing new to fetch.");
    return;
  }

  const manifest = loadManifest();
  const kept = manifest.items;
  writeFileSync(manifestPath, JSON.stringify({ ...manifest, items: [...kept, ...newItems] }, null, 2) + "\n");
  console.log(`Manifest updated: ${kept.length} existing + ${newItems.length} macrumors item(s).`);

  const server = await startCorpusServer(join(benchmarksRoot, "corpus"));
  console.log(`\nVerifying via production ingestion pipeline (${server.baseUrl}):`);
  for (const item of newItems) {
    const outcome = await extractFixture(server.baseUrl, item.path as string);
    if (!outcome.ok) {
      console.log(`  ${item.id}: INGEST FAILED (${outcome.errorCode})`);
      verified.push({ id: item.id as string, ok: false, score: 0 });
      continue;
    }
    const exp = (item.expected as { title: string; author: string; search_terms: string[] });
    const titleOk = normalizeText(outcome.title ?? "") === normalizeText(exp.title);
    const authorOk = normalizeText(outcome.author ?? "") === normalizeText(exp.author);
    const searchable = `${outcome.title ?? ""} ${outcome.description ?? ""} ${outcome.text ?? ""}`.toLowerCase();
    const terms = exp.search_terms.filter((term) => searchable.includes(term.toLowerCase()));
    const score = terms.length / exp.search_terms.length;
    console.log(
      `  ${item.id}: title=${titleOk ? "OK" : `MISS (${JSON.stringify(outcome.title)})`} author=${authorOk ? "OK" : `MISS (${JSON.stringify(outcome.author)})`} terms=${terms.length}/${exp.search_terms.length} textLen=${(outcome.text ?? "").length} extractor=${outcome.extractor}`,
    );
    verified.push({ id: item.id as string, ok: titleOk && authorOk && score === 1, score });
  }
  server.stop();

  const failed = verified.filter((v) => !v.ok);
  console.log(`\n${verified.length - failed.length}/${verified.length} fixtures verified.`);
  if (failed.length) process.exit(1);
}

await main();
