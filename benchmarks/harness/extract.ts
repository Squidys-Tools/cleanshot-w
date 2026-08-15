import { ingestUrl } from "../../src/lib/ingestion";
import { isUrlIngestionError } from "../../src/lib/ingestion/errors";

export interface ExtractionOutcome {
  engine: "production:ingestUrl";
  ok: boolean;
  errorCode?: string;
  errorDetail?: string;
  title?: string;
  author?: string;
  description?: string;
  publishedDate?: string | null;
  text?: string;
  imageUrls: string[];
  safeEmbeds: { kind: string; provider: string; sourceUrl: string }[];
  extractor?: string;
}

/** Runs the production `ingestUrl` pipeline against a local fixture URL. */
export async function extractFixture(baseUrl: string, relPath: string): Promise<ExtractionOutcome> {
  const url = `${baseUrl}/${relPath}`;
  try {
    const article = await ingestUrl(url);
    return {
      engine: "production:ingestUrl",
      ok: true,
      title: article.title,
      author: article.author,
      description: article.description,
      publishedDate: article.publishedDate,
      text: article.text,
      imageUrls: article.imageUrls,
      safeEmbeds: article.safeEmbeds.map((embed) => ({
        kind: embed.kind,
        provider: embed.provider,
        sourceUrl: embed.sourceUrl,
      })),
      extractor: article.extractor,
    };
  } catch (cause) {
    const code = isUrlIngestionError(cause) ? cause.code : "unknown";
    const detail = cause instanceof Error ? cause.message : String(cause);
    return {
      engine: "production:ingestUrl",
      ok: false,
      errorCode: code,
      errorDetail: detail,
      imageUrls: [],
      safeEmbeds: [],
    };
  }
}
