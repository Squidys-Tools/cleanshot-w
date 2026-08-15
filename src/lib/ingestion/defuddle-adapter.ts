import Defuddle from "defuddle";
import type { DefuddleResponse } from "defuddle";
import { hasReadableText } from "./html-safety";
import { extractJsonLdMetadata } from "./json-ld";
import { normalizeHttpUrl, normalizePublishedDate, normalizeText, uniqueStrings } from "./url";
import type { DefuddleAdapter, RawArticleExtraction } from "./types";

function imageUrlsFromResult(result: DefuddleResponse, url: string): string[] {
  const image = normalizeHttpUrl(result.image, url);
  return image ? [image] : [];
}

export class DefaultDefuddleAdapter implements DefuddleAdapter {
  readonly name = "defuddle";

  extract(document: Document, url: string): RawArticleExtraction {
    // Defuddle's metadata extractor expects canonical links to be absolute.
    // Normalizing this small piece before handing over the document keeps the
    // adapter compatible with pages that use the common relative form.
    const canonical = document.querySelector('link[rel="canonical"]');
    const canonicalUrl = normalizeHttpUrl(canonical?.getAttribute("href"), url);
    if (canonicalUrl) canonical?.setAttribute("href", canonicalUrl);

    const result = new Defuddle(document, {
      url,
      removeImages: false,
      standardize: true,
      useAsync: false,
    }).parse();

    const structured = extractJsonLdMetadata(document, url);

    return {
      title: normalizeText(result.title) || structured.title,
      description: normalizeText(result.description) || structured.description,
      author: normalizeText(result.author) || structured.author,
      publishedDate: normalizePublishedDate(result.published ?? structured.publishedDate ?? null),
      canonicalUrl: canonicalUrl ?? undefined,
      contentHtml: hasReadableText(result.content) ? result.content : "",
      imageUrls: uniqueStrings([
        ...imageUrlsFromResult(result, url),
        ...(structured.imageUrls ?? []),
      ]),
    };
  }
}
