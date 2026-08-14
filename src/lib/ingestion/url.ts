import { UrlIngestionError } from "./errors";

const MAX_URL_LENGTH = 8_192;

export function parseHttpUrl(input: string): URL {
  const value = input.trim();

  if (value.length === 0 || value.length > MAX_URL_LENGTH) {
    throw new UrlIngestionError("invalid-url", "The URL is empty or too long.", { url: input });
  }

  let parsed: URL;
  try {
    parsed = new URL(value);
  } catch (cause) {
    throw new UrlIngestionError("invalid-url", "The URL could not be parsed.", { cause, url: input });
  }

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new UrlIngestionError("invalid-url", "Only HTTP and HTTPS URLs can be saved.", { url: input });
  }

  if (parsed.username || parsed.password || !parsed.hostname) {
    throw new UrlIngestionError("invalid-url", "The URL contains unsupported credentials or a missing host.", {
      url: input,
    });
  }

  return parsed;
}

export function normalizeHttpUrl(value: string | null | undefined, baseUrl?: string): string | null {
  const candidate = value?.trim();
  if (!candidate || candidate.startsWith("#")) {
    return null;
  }

  let parsed: URL;
  try {
    parsed = baseUrl ? new URL(candidate, baseUrl) : new URL(candidate);
  } catch {
    return null;
  }

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return null;
  }

  if (parsed.username || parsed.password || !parsed.hostname) {
    return null;
  }

  parsed.hash = "";
  return parsed.toString();
}

export function normalizePublishedDate(value: string | null | undefined): string | null {
  const candidate = value?.trim();
  if (!candidate) {
    return null;
  }

  const timestamp = Date.parse(candidate);
  return Number.isNaN(timestamp) ? null : new Date(timestamp).toISOString();
}

export function normalizeText(value: string | null | undefined): string {
  return value?.replace(/\s+/g, " ").trim() ?? "";
}

export function uniqueStrings(values: Iterable<string>): string[] {
  return [...new Set([...values].filter(Boolean))];
}
