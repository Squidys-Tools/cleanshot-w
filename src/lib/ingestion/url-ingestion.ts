import { parseHTML } from "linkedom";
import { DefaultDefuddleAdapter } from "./defuddle-adapter";
import { UrlIngestionError } from "./errors";
import { extractFallback } from "./fallback";
import { collectImageUrls, collectSafeEmbeds, htmlToText, sanitizeHtml } from "./html-safety";
import { normalizeHttpUrl, normalizePublishedDate, normalizeText, parseHttpUrl, uniqueStrings } from "./url";
import type { NormalizedArticle, RawArticleExtraction, UrlIngestionOptions } from "./types";

const DEFAULT_TIMEOUT_MS = 15_000;
const DEFAULT_MAX_RESPONSE_BYTES = 8 * 1024 * 1024;
const DEFAULT_USER_AGENT = "mymind-library/0.1 (+local article capture)";

async function readResponseText(response: Response, maxBytes: number, url: string): Promise<string> {
  if (!response.body) {
    const text = await response.text();
    if (new TextEncoder().encode(text).byteLength > maxBytes) {
      throw new UrlIngestionError("content-too-large", "The downloaded page is larger than the ingestion limit.", {
        url,
      });
    }
    return text;
  }

  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (!value) continue;
      total += value.byteLength;
      if (total > maxBytes) {
        await reader.cancel();
        throw new UrlIngestionError("content-too-large", "The downloaded page is larger than the ingestion limit.", {
          url,
        });
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }

  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder().decode(bytes);
}

function mergeExtraction(primary: RawArticleExtraction, fallback: RawArticleExtraction): RawArticleExtraction {
  return {
    title: normalizeText(primary.title) || fallback.title,
    description: normalizeText(primary.description) || fallback.description,
    author: normalizeText(primary.author) || fallback.author,
    publishedDate: primary.publishedDate ?? fallback.publishedDate ?? null,
    canonicalUrl: primary.canonicalUrl ?? fallback.canonicalUrl,
    contentHtml: normalizeText(primary.contentHtml) ? primary.contentHtml : fallback.contentHtml,
    imageUrls: uniqueStrings([...(fallback.imageUrls ?? []), ...(primary.imageUrls ?? [])]),
  };
}

function parseDocument(html: string): Document {
  try {
    return parseHTML(html).document;
  } catch (cause) {
    throw new UrlIngestionError("parse-failed", "The downloaded page could not be parsed as HTML.", { cause });
  }
}

export async function ingestUrl(input: string, options: UrlIngestionOptions = {}): Promise<NormalizedArticle> {
  const sourceUrl = parseHttpUrl(input).toString();
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const maxResponseBytes = options.maxResponseBytes ?? DEFAULT_MAX_RESPONSE_BYTES;
  const fetchImpl = options.fetch ?? globalThis.fetch;

  if (!fetchImpl) {
    throw new UrlIngestionError("network-error", "No fetch implementation is available.", { url: sourceUrl });
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);
  let response: Response;

  try {
    response = await fetchImpl(sourceUrl, {
      headers: {
        Accept: "text/html,application/xhtml+xml;q=0.9,text/plain;q=0.5",
        "User-Agent": options.userAgent ?? DEFAULT_USER_AGENT,
      },
      redirect: "follow",
      signal: controller.signal,
    });
  } catch (cause) {
    clearTimeout(timeout);
    if (controller.signal.aborted) {
      throw new UrlIngestionError("timeout", "The page took too long to download.", { cause, url: sourceUrl });
    }
    throw new UrlIngestionError("network-error", "The page could not be downloaded.", { cause, url: sourceUrl });
  }

  let fetchedUrl = sourceUrl;
  let rawHtml: string;
  try {
    if (!response.ok) {
      throw new UrlIngestionError("http-error", `The page returned HTTP ${response.status}.`, {
        status: response.status,
        url: sourceUrl,
      });
    }

    fetchedUrl = normalizeHttpUrl(response.url, sourceUrl) ?? sourceUrl;
    const contentType = response.headers.get("content-type")?.toLowerCase() ?? "";
    if (contentType && !contentType.includes("text/html") && !contentType.includes("application/xhtml+xml") && !contentType.startsWith("text/plain")) {
      throw new UrlIngestionError("unsupported-content-type", "The URL did not return an HTML document.", {
        url: fetchedUrl,
      });
    }

    rawHtml = await readResponseText(response, maxResponseBytes, fetchedUrl);
  } catch (error) {
    if (error instanceof UrlIngestionError) throw error;
    if (controller.signal.aborted) {
      throw new UrlIngestionError("timeout", "The page took too long to download.", {
        cause: error,
        url: fetchedUrl,
      });
    }
    throw new UrlIngestionError("network-error", "The page could not be read.", { cause: error, url: fetchedUrl });
  } finally {
    clearTimeout(timeout);
  }

  const sourceDocument = parseDocument(rawHtml);
  const extractionDocument = parseDocument(rawHtml);
  const fallback = extractFallback(sourceDocument, fetchedUrl);
  const adapter = options.defuddleAdapter ?? new DefaultDefuddleAdapter();
  let extraction = fallback;
  let extractor: "defuddle" | "fallback" = "fallback";

  try {
    const defuddleExtraction = await adapter.extract(extractionDocument, fetchedUrl);
    extraction = mergeExtraction(defuddleExtraction, fallback);
    if (normalizeText(defuddleExtraction.contentHtml)) extractor = "defuddle";
  } catch {
    extraction = fallback;
  }

  const html = sanitizeHtml(extraction.contentHtml ?? "", fetchedUrl);
  if (!normalizeText(html)) {
    throw new UrlIngestionError("extraction-failed", "No readable article content was found on the page.", {
      url: fetchedUrl,
    });
  }

  const canonicalUrl = normalizeHttpUrl(extraction.canonicalUrl, fetchedUrl) ?? fetchedUrl;
  const imageUrls = uniqueStrings([
    ...collectImageUrls(sourceDocument, fetchedUrl),
    ...(extraction.imageUrls ?? []).map((value) => normalizeHttpUrl(value, fetchedUrl)).filter((value): value is string => value !== null),
  ]);

  return {
    sourceUrl,
    fetchedUrl,
    canonicalUrl,
    title: normalizeText(extraction.title) || new URL(fetchedUrl).hostname,
    description: normalizeText(extraction.description),
    author: normalizeText(extraction.author),
    publishedDate: normalizePublishedDate(extraction.publishedDate),
    html,
    text: htmlToText(html),
    imageUrls,
    safeEmbeds: collectSafeEmbeds(sourceDocument, fetchedUrl),
    extractor,
  };
}
