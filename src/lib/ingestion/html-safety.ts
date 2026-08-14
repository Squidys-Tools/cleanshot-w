import { parseHTML } from "linkedom";
import { normalizeHttpUrl, normalizeText, uniqueStrings } from "./url";
import type { SafeEmbedCandidate, SafeEmbedKind, SafeEmbedProvider } from "./types";

const SAFE_IFRAME_HOSTS = new Set([
  "loom.com",
  "open.spotify.com",
  "player.twitch.tv",
  "player.vimeo.com",
  "soundcloud.com",
  "w.soundcloud.com",
  "vimeo.com",
  "www.loom.com",
  "www.youtube-nocookie.com",
  "www.youtube.com",
  "youtu.be",
  "youtube-nocookie.com",
  "youtube.com",
]);

const MEDIA_EXTENSION = /\.(?:m3u8|mp3|mp4|ogg|oga|ogv|webm)(?:$|[?#])/i;

function urlsFromSrcSet(value: string | null, pageUrl: string): string[] {
  if (!value) return [];
  return value
    .split(",")
    .map((entry) => entry.trim().split(/\s+/)[0])
    .map((entry) => normalizeHttpUrl(entry, pageUrl))
    .filter((entry): entry is string => entry !== null);
}

function providerForHost(hostname: string): SafeEmbedProvider | null {
  const host = hostname.toLowerCase();
  if (host === "youtube.com" || host === "www.youtube.com" || host === "youtube-nocookie.com" || host === "www.youtube-nocookie.com" || host === "youtu.be") return "youtube";
  if (host === "vimeo.com" || host === "www.vimeo.com" || host === "player.vimeo.com") return "vimeo";
  if (host === "loom.com" || host === "www.loom.com") return "loom";
  if (host === "open.spotify.com") return "spotify";
  if (host === "soundcloud.com" || host === "w.soundcloud.com") return "soundcloud";
  if (host === "player.twitch.tv") return "twitch";
  return null;
}

function providerEmbedUrl(url: URL, provider: SafeEmbedProvider): string | null {
  const pathParts = url.pathname.split("/").filter(Boolean);

  if (provider === "youtube") {
    const host = url.hostname.toLowerCase();
    const id = host === "youtu.be"
      ? pathParts.length === 1 ? pathParts[0] : null
      : url.pathname === "/watch"
        ? url.searchParams.get("v")
        : ["shorts", "embed", "live", "v"].includes(pathParts[0] ?? "") && pathParts.length === 2
          ? pathParts[1]
          : null;
    return id && /^[a-zA-Z0-9_-]{6,32}$/u.test(id)
      ? `https://www.youtube-nocookie.com/embed/${encodeURIComponent(id)}`
      : null;
  }

  if (provider === "vimeo") {
    const id = pathParts[pathParts.length - 1];
    return id && /^\d{1,20}$/u.test(id) ? `https://player.vimeo.com/video/${encodeURIComponent(id)}` : null;
  }

  if (provider === "loom") {
    const id = pathParts[pathParts.length - 1];
    return pathParts.length === 2 && ["share", "embed"].includes(pathParts[0] ?? "") && id
      ? `https://www.loom.com/embed/${encodeURIComponent(id)}`
      : null;
  }

  if (provider === "spotify") {
    const [type, id] = pathParts;
    const validTypes = new Set(["album", "artist", "audiobook", "episode", "playlist", "show", "track"]);
    return type && id && pathParts.length === 2 && validTypes.has(type) && /^[a-zA-Z0-9]+$/u.test(id)
      ? `https://open.spotify.com/embed/${encodeURIComponent(type)}/${encodeURIComponent(id)}`
      : null;
  }

  if (provider === "soundcloud") {
    return url.hostname.toLowerCase() === "w.soundcloud.com" && url.pathname === "/player/"
      ? url.toString()
      : null;
  }

  if (provider === "twitch") {
    return url.hostname.toLowerCase() === "player.twitch.tv" &&
      (url.searchParams.has("channel") || url.searchParams.has("video"))
      ? url.toString()
      : null;
  }

  if (provider === "direct") {
    return url.toString();
  }

  return null;
}

function titleFor(element: Element): string | undefined {
  const title = normalizeText(element.getAttribute("title") ?? element.getAttribute("aria-label"));
  return title || undefined;
}

function candidateFromUrl(
  rawUrl: string | null,
  kind: SafeEmbedKind,
  title: string | undefined,
  pageUrl: string,
): SafeEmbedCandidate | null {
  const resolved = normalizeHttpUrl(rawUrl, pageUrl);
  if (!resolved) return null;

  let url: URL;
  try {
    url = new URL(resolved);
  } catch {
    return null;
  }

  const provider = providerForHost(url.hostname);
  if (kind === "iframe") {
    if (!provider || !SAFE_IFRAME_HOSTS.has(url.hostname.toLowerCase())) return null;
    const embedUrl = providerEmbedUrl(url, provider);
    return embedUrl ? { kind, provider, sourceUrl: resolved, embedUrl, title } : null;
  }

  if (!MEDIA_EXTENSION.test(url.toString()) || provider) return null;
  return { kind, provider: "direct", sourceUrl: resolved, embedUrl: resolved, title };
}

export function collectSafeEmbeds(document: Document, pageUrl: string): SafeEmbedCandidate[] {
  const candidates: SafeEmbedCandidate[] = [];

  for (const element of document.querySelectorAll("iframe[src], video[src], audio[src]")) {
    const tagName = element.tagName.toLowerCase();
    const kind: SafeEmbedKind = tagName === "iframe" ? "iframe" : tagName === "audio" ? "audio" : "video";
    const candidate = candidateFromUrl(element.getAttribute("src"), kind, titleFor(element), pageUrl);
    if (candidate) candidates.push(candidate);
  }

  for (const element of document.querySelectorAll("video[poster], video source[src], audio source[src]")) {
    const tagName = element.tagName.toLowerCase();
    const parentName = element.parentElement?.tagName.toLowerCase();
    const kind: SafeEmbedKind = tagName === "audio" || parentName === "audio" ? "audio" : "video";
    const candidate = candidateFromUrl(element.getAttribute("src"), kind, titleFor(element), pageUrl);
    if (candidate) candidates.push(candidate);
  }

  for (const element of document.querySelectorAll("meta[property], meta[name]")) {
    const key = (element.getAttribute("property") ?? element.getAttribute("name") ?? "").toLowerCase();
    if (key === "og:video" || key === "og:video:url" || key === "twitter:player") {
      const candidate = candidateFromUrl(element.getAttribute("content"), "iframe", undefined, pageUrl);
      if (candidate) candidates.push(candidate);
    }
  }

  const seen = new Set<string>();
  return candidates.filter((candidate) => {
    const key = `${candidate.kind}:${candidate.embedUrl}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function sanitizeElement(element: Element, pageUrl: string): void {
  const tagName = element.tagName.toLowerCase();
  const attributes = Array.from(element.attributes);

  for (const attribute of attributes) {
    const name = attribute.name.toLowerCase();
    if (name.startsWith("on") || name === "style" || name === "srcdoc" || name === "integrity") {
      element.removeAttribute(attribute.name);
    }
  }

  for (const attributeName of ["href", "src", "poster", "cite"]) {
    const raw = element.getAttribute(attributeName);
    if (!raw) continue;
    const normalized = normalizeHttpUrl(raw, pageUrl);
    if (normalized) element.setAttribute(attributeName, normalized);
    else element.removeAttribute(attributeName);
  }

  element.removeAttribute("srcset");

  if (tagName === "iframe") {
    const candidate = candidateFromUrl(element.getAttribute("src"), "iframe", titleFor(element), pageUrl);
    if (!candidate) {
      element.remove();
      return;
    }
    element.setAttribute("src", candidate.embedUrl);
    element.setAttribute("loading", "lazy");
    element.setAttribute("referrerpolicy", "no-referrer");
    element.removeAttribute("allow");
    element.removeAttribute("allowfullscreen");
  }

  const mediaParent = element.parentElement?.tagName.toLowerCase();
  if (tagName === "video" || tagName === "audio" ||
      (tagName === "source" && (mediaParent === "video" || mediaParent === "audio"))) {
    const rawSource = element.getAttribute("src");
    const kind: SafeEmbedKind = tagName === "audio" || mediaParent === "audio" ? "audio" : "video";
    if (rawSource && !candidateFromUrl(rawSource, kind, titleFor(element), pageUrl)) {
      if (tagName === "source") element.remove();
      else element.removeAttribute("src");
    }
  }
}

export function sanitizeHtml(html: string, pageUrl: string): string {
  const { document } = parseHTML(`<html><head></head><body>${html}</body></html>`);
  for (const selector of ["script", "noscript", "template", "style", "link", "meta", "object", "embed", "form"]) {
    for (const element of document.querySelectorAll(selector)) element.remove();
  }
  for (const element of document.querySelectorAll("*")) sanitizeElement(element, pageUrl);
  return document.body?.innerHTML ?? "";
}

export function htmlToText(html: string): string {
  const { document } = parseHTML(`<html><head></head><body>${html}</body></html>`);
  return normalizeText(document.body?.textContent);
}

export function collectImageUrls(document: Document, pageUrl: string): string[] {
  const values: string[] = [];

  for (const element of document.querySelectorAll("img[src], source[src], video[poster], img[srcset], source[srcset]")) {
    const raw = element.getAttribute("src") ?? element.getAttribute("poster");
    const normalized = normalizeHttpUrl(raw, pageUrl);
    if (normalized) values.push(normalized);
    values.push(...urlsFromSrcSet(element.getAttribute("srcset"), pageUrl));
  }

  for (const element of document.querySelectorAll("meta[property], meta[name]")) {
    const key = (element.getAttribute("property") ?? element.getAttribute("name") ?? "").toLowerCase();
    if (!key.includes("image")) continue;
    const normalized = normalizeHttpUrl(element.getAttribute("content"), pageUrl);
    if (normalized) values.push(normalized);
  }

  return uniqueStrings(values);
}
