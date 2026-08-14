export type IngestionExtractor = "defuddle" | "fallback";

export type SafeEmbedKind = "iframe" | "video" | "audio";

export type SafeEmbedProvider =
  | "direct"
  | "loom"
  | "soundcloud"
  | "spotify"
  | "twitch"
  | "vimeo"
  | "youtube";

export interface SafeEmbedCandidate {
  kind: SafeEmbedKind;
  provider: SafeEmbedProvider;
  sourceUrl: string;
  embedUrl: string;
  title?: string;
}

export interface NormalizedArticle {
  sourceUrl: string;
  fetchedUrl: string;
  canonicalUrl: string;
  title: string;
  description: string;
  author: string;
  publishedDate: string | null;
  html: string;
  text: string;
  imageUrls: string[];
  safeEmbeds: SafeEmbedCandidate[];
  extractor: IngestionExtractor;
}

export interface RawArticleExtraction {
  title?: string;
  description?: string;
  author?: string;
  publishedDate?: string | null;
  canonicalUrl?: string;
  contentHtml?: string;
  imageUrls?: string[];
}

export interface DefuddleAdapter {
  readonly name: string;
  extract(document: Document, url: string): RawArticleExtraction | Promise<RawArticleExtraction>;
}

export interface UrlIngestionOptions {
  fetch?: typeof globalThis.fetch;
  timeoutMs?: number;
  maxResponseBytes?: number;
  userAgent?: string;
  defuddleAdapter?: DefuddleAdapter;
}
