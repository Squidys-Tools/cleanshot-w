export { DefaultDefuddleAdapter } from "./defuddle-adapter";
export { UrlIngestionError, isUrlIngestionError } from "./errors";
export { extractFallback } from "./fallback";
export { collectImageUrls, collectSafeEmbeds, htmlToText, sanitizeHtml } from "./html-safety";
export { ingestUrl } from "./url-ingestion";
export { normalizeHttpUrl, normalizePublishedDate, normalizeText, parseHttpUrl, uniqueStrings } from "./url";
export type {
  DefuddleAdapter,
  IngestionExtractor,
  NormalizedArticle,
  RawArticleExtraction,
  SafeEmbedCandidate,
  SafeEmbedKind,
  SafeEmbedProvider,
  UrlIngestionOptions,
} from "./types";
