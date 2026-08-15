import { extractJsonLdMetadata } from "./json-ld";
import { normalizeHttpUrl, normalizePublishedDate, normalizeText, uniqueStrings } from "./url";
import type { RawArticleExtraction } from "./types";

const META_DESCRIPTION_NAMES = ["description", "og:description", "twitter:description"];
const META_AUTHOR_NAMES = ["author", "article:author", "byl", "byline"];
const META_DATE_NAMES = [
  "article:published_time",
  "datePublished",
  "datepublished",
  "publishdate",
  "publish_date",
  "date",
];

function metaContent(document: Document, names: string[]): string | null {
  for (const name of names) {
    const element = document.querySelector(`meta[name="${name}"], meta[property="${name}"]`);
    const content = element?.getAttribute("content");
    if (content?.trim()) {
      return content.trim();
    }
  }

  return null;
}

function firstMeaningfulText(document: Document, selectors: string[]): string {
  for (const selector of selectors) {
    const text = normalizeText(document.querySelector(selector)?.textContent);
    if (text) {
      return text;
    }
  }

  return "";
}

function extractImageUrls(document: Document, baseUrl: string): string[] {
  const values: string[] = [];

  for (const element of document.querySelectorAll("meta[property], meta[name]")) {
    const key = (element.getAttribute("property") ?? element.getAttribute("name") ?? "").toLowerCase();
    if (key.includes("image")) {
      const value = normalizeHttpUrl(element.getAttribute("content"), baseUrl);
      if (value) values.push(value);
    }
  }

  for (const element of document.querySelectorAll("img[src], source[src], video[poster]")) {
    const raw = element.getAttribute("src") ?? element.getAttribute("poster");
    const value = normalizeHttpUrl(raw, baseUrl);
    if (value) values.push(value);
  }

  return uniqueStrings(values);
}

function contentRoot(document: Document): Element | null {
  const candidates = [
    ...document.querySelectorAll("article"),
    ...document.querySelectorAll("main"),
    ...document.querySelectorAll('[role="main"]'),
  ];

  return candidates.find((candidate) => normalizeText(candidate.textContent).length > 80) ?? candidates[0] ?? document.body;
}

const BYLINE_SELECTOR = [
  '[rel="author"]',
  '[itemprop="author"]',
  '[class*="byline"]',
  '[class*="byLine"]',
  '[class*="author"]',
  '[class*="writer"]',
].join(", ");

const BYLINE_PREFIXES = /^(?:by|written by|recipe by|words by|story by|text by)\b/i;

const NAME_PATTERN = /^[A-ZÀ-ÖØ-Þ][\w'.&À-ÖØ-Þ-]*(?:\s+[A-ZÀ-ÖØ-Þ][\w'.&À-ÖØ-Þ-]*){1,2}$/u;
const NAME_WORD_PATTERN = /^[A-ZÀ-ÖØ-Þ][\w'.&À-ÖØ-Þ-]*$/u;

function bylineAuthor(document: Document): string {
  const candidates = [...document.querySelectorAll(BYLINE_SELECTOR)];

  for (const candidate of candidates) {
    const anchored = [...candidate.querySelectorAll("a")]
      .map((anchor) => normalizeText(anchor.textContent))
      .filter(Boolean);
    const anchoredName = anchored.find((value) => NAME_PATTERN.test(value));
    if (anchoredName) return anchoredName;

    const text = normalizeText(candidate.textContent);
    if (!text) continue;

    const prefixed = text.match(BYLINE_PREFIXES);
    if (!prefixed) continue;

    const rest = text.slice(prefixed[0].length).trim();
    const head = rest.split(/[,·|—–/]/u)[0].trim();
    const nameWords: string[] = [];
    for (const word of head.split(/\s+/)) {
      if (!NAME_WORD_PATTERN.test(word) || nameWords.length === 3) break;
      nameWords.push(word);
    }
    const name = nameWords.join(" ");
    if (NAME_PATTERN.test(name)) return name;
  }

  return "";
}

export function extractFallback(document: Document, url: string): RawArticleExtraction {
  const root = contentRoot(document);
  const canonical = normalizeHttpUrl(document.querySelector('link[rel="canonical"]')?.getAttribute("href"), url);
  const published =
    metaContent(document, META_DATE_NAMES) ??
    document.querySelector("time[datetime]")?.getAttribute("datetime") ??
    null;
  const structured = extractJsonLdMetadata(document, url);

  return {
    title:
      structured.title ??
      metaContent(document, ["og:title", "twitter:title"]) ??
      firstMeaningfulText(document, ["h1", "title"]) ??
      new URL(url).hostname,
    description: structured.description ?? metaContent(document, META_DESCRIPTION_NAMES) ?? "",
    author:
      structured.author ??
      metaContent(document, META_AUTHOR_NAMES) ??
      bylineAuthor(document),
    publishedDate: normalizePublishedDate(structured.publishedDate ?? published),
    canonicalUrl: canonical ?? url,
    contentHtml: root?.innerHTML ?? "",
    imageUrls: uniqueStrings([
      ...(structured.imageUrls ?? []),
      ...extractImageUrls(document, url),
    ]),
  };
}
