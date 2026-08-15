import { normalizeHttpUrl, normalizeText, uniqueStrings } from "./url";

const ARTICLE_TYPES = new Set([
  "AnalysisNewsArticle",
  "Article",
  "AskPublicNewsArticle",
  "BackgroundNewsArticle",
  "BlogPosting",
  "LiveBlogPosting",
  "NewsArticle",
  "OpinionNewsArticle",
  "ReportageNewsArticle",
  "Review",
  "ReviewNewsArticle",
  "ScholarlyArticle",
  "SocialMediaPosting",
  "TechArticle",
]);

export interface JsonLdArticleMetadata {
  title?: string;
  description?: string;
  author?: string;
  publishedDate?: string;
  imageUrls?: string[];
}

function typeNames(type: unknown): string[] {
  const values = Array.isArray(type) ? type : [type];
  return values
    .filter((value): value is string => typeof value === "string")
    .map((value) => value.split("/").filter(Boolean).pop() ?? value);
}

function isArticleType(type: unknown): boolean {
  return typeNames(type).some((name) => ARTICLE_TYPES.has(name));
}

function textOf(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function authorName(value: unknown): string {
  if (typeof value === "string") return value;
  if (Array.isArray(value)) {
    return uniqueStrings(value.map(authorName)).join(", ");
  }
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    const name = textOf(record.name).trim();
    if (name) return name;
    const givenName = textOf(record.givenName).trim();
    const familyName = textOf(record.familyName).trim();
    if (givenName || familyName) return [givenName, familyName].filter(Boolean).join(" ");
  }
  return "";
}

function imageUrlsOf(value: unknown, pageUrl: string): string[] {
  const collect = (item: unknown, out: string[]): void => {
    if (typeof item === "string") {
      const normalized = normalizeHttpUrl(item, pageUrl);
      if (normalized) out.push(normalized);
      return;
    }
    if (Array.isArray(item)) {
      for (const entry of item) collect(entry, out);
      return;
    }
    if (item && typeof item === "object") {
      const record = item as Record<string, unknown>;
      collect(record.url, out);
    }
  };
  const out: string[] = [];
  collect(value, out);
  return uniqueStrings(out);
}

function metadataFromNode(node: Record<string, unknown>, pageUrl: string): JsonLdArticleMetadata | null {
  if (!isArticleType(node["@type"])) return null;

  const author = Array.isArray(node.author)
    ? uniqueStrings(node.author.map(authorName)).join(", ")
    : authorName(node.author);

  return {
    title: normalizeText(textOf(node.headline) || textOf(node.name)),
    description: normalizeText(textOf(node.description)),
    author: normalizeText(author),
    publishedDate: textOf(node.datePublished) || textOf(node.dateModified) || undefined,
    imageUrls: imageUrlsOf(node.image, pageUrl),
  };
}

function mergeMetadata(
  target: JsonLdArticleMetadata,
  incoming: JsonLdArticleMetadata,
): JsonLdArticleMetadata {
  return {
    title: normalizeText(target.title) || incoming.title,
    description: normalizeText(target.description) || incoming.description,
    author: normalizeText(target.author) || incoming.author,
    publishedDate: target.publishedDate || incoming.publishedDate,
    imageUrls: uniqueStrings([...(target.imageUrls ?? []), ...(incoming.imageUrls ?? [])]),
  };
}

function walk(value: unknown, visit: (node: Record<string, unknown>) => void): void {
  if (Array.isArray(value)) {
    for (const entry of value) walk(entry, visit);
    return;
  }
  if (!value || typeof value !== "object") return;
  const record = value as Record<string, unknown>;
  if (typeof record["@type"] === "string" || Array.isArray(record["@type"])) {
    visit(record);
  }
  for (const key of ["@graph", "@type", "mainEntityOfPage"]) {
    walk(record[key], visit);
  }
}

/**
 * Pulls article metadata out of schema.org JSON-LD blocks. Only known article
 * types are read, and only metadata fields (never user comment bodies, which
 * many news sites embed in the same block).
 */
export function extractJsonLdMetadata(document: Document, pageUrl: string): JsonLdArticleMetadata {
  let metadata: JsonLdArticleMetadata = {};

  for (const script of document.querySelectorAll('script[type="application/ld+json"]')) {
    let parsed: unknown;
    try {
      parsed = JSON.parse(script.textContent ?? "");
    } catch {
      continue;
    }

    let candidate: JsonLdArticleMetadata | null = null;
    if (Array.isArray(parsed)) {
      for (const entry of parsed) {
        walk(entry, (node) => {
          const extracted = metadataFromNode(node, pageUrl);
          if (extracted) {
            candidate = candidate ? mergeMetadata(candidate, extracted) : extracted;
          }
        });
      }
    } else {
      walk(parsed, (node) => {
        const extracted = metadataFromNode(node, pageUrl);
        if (extracted) {
          candidate = candidate ? mergeMetadata(candidate, extracted) : extracted;
        }
      });
    }

    if (candidate) {
      metadata = mergeMetadata(metadata, candidate);
    }
  }

  return metadata;
}
