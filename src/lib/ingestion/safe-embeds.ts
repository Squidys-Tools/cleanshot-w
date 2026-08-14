/**
 * URL-only embed validation. This module never fetches a URL and never returns
 * an arbitrary user-supplied iframe source.
 */

export type SafeEmbedDescriptor =
  | {
      type: "iframe";
      provider: "youtube" | "vimeo";
      src: string;
      allow: "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture";
      sandbox: "allow-scripts allow-same-origin allow-presentation";
    }
  | {
      type: "video";
      provider: "direct";
      src: string;
      controls: true;
      preload: "metadata";
    };

export interface SafeEmbedOptions {
  /** Exact HTTPS origins permitted for direct media files. */
  allowedDirectMediaOrigins?: readonly string[];
}

const YOUTUBE_HOSTS = new Set([
  "youtube.com",
  "www.youtube.com",
  "m.youtube.com",
  "youtu.be",
  "youtube-nocookie.com",
  "www.youtube-nocookie.com",
]);

const VIMEO_HOSTS = new Set(["vimeo.com", "www.vimeo.com", "player.vimeo.com"]);
const DIRECT_VIDEO_EXTENSIONS = new Set(["m4v", "mov", "mp4", "mkv", "ogv", "webm"]);

const IFRAME_ALLOW =
  "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" as const;
const IFRAME_SANDBOX = "allow-scripts allow-same-origin allow-presentation" as const;

function parseHttpsUrl(value: string): URL | null {
  try {
    const url = new URL(value.trim());
    if (
      url.protocol !== "https:" ||
      url.username.length > 0 ||
      url.password.length > 0 ||
      url.port.length > 0
    ) {
      return null;
    }
    return url;
  } catch {
    return null;
  }
}

function cleanHost(hostname: string): string {
  return hostname.toLowerCase().replace(/\.$/u, "");
}

function validYouTubeId(value: string | null | undefined): value is string {
  return value !== null && value !== undefined && /^[a-zA-Z0-9_-]{6,32}$/u.test(value);
}

function youtubeVideoId(url: URL): string | null {
  const host = cleanHost(url.hostname);
  const segments = url.pathname.split("/").filter(Boolean);

  if (host === "youtu.be") {
    return validYouTubeId(segments[0]) ? segments[0] : null;
  }

  if (url.pathname === "/watch") {
    const id = url.searchParams.get("v");
    return validYouTubeId(id) ? id : null;
  }

  if (["shorts", "embed", "live", "v"].includes(segments[0] ?? "")) {
    return validYouTubeId(segments[1]) ? segments[1] : null;
  }

  return null;
}

function vimeoVideoId(url: URL): string | null {
  const segments = url.pathname.split("/").filter(Boolean);
  if (segments.length === 1 && /^\d{1,20}$/u.test(segments[0] ?? "")) {
    return segments[0] ?? null;
  }

  const knownPrefixes = new Set(["video", "channels", "groups", "ondemand"]);
  const index = segments.findIndex((segment) => knownPrefixes.has(segment));
  const candidate = index > -1 ? segments[segments.length - 1] : null;
  return candidate && /^\d{1,20}$/u.test(candidate) ? candidate : null;
}

function directMediaExtension(url: URL): string | null {
  const lastSegment = url.pathname.split("/").pop() ?? "";
  const dot = lastSegment.lastIndexOf(".");
  const extension = dot > -1 ? lastSegment.slice(dot + 1).toLowerCase() : "";
  return DIRECT_VIDEO_EXTENSIONS.has(extension) ? extension : null;
}

function allowedOrigin(url: URL, origins: readonly string[]): boolean {
  return origins.some((candidate) => {
    const allowed = parseHttpsUrl(candidate);
    return allowed !== null && allowed.origin === url.origin;
  });
}

function iframeDescriptor(provider: "youtube" | "vimeo", src: string): SafeEmbedDescriptor {
  return {
    type: "iframe",
    provider,
    src,
    allow: IFRAME_ALLOW,
    sandbox: IFRAME_SANDBOX,
  };
}

/**
 * Returns a canonical descriptor for an allowed embed, or null for unsafe,
 * malformed, or unrecognized URLs. Direct media is opt-in by exact origin.
 */
export function sanitizeEmbedUrl(
  value: string,
  options: SafeEmbedOptions = {},
): SafeEmbedDescriptor | null {
  const url = parseHttpsUrl(value);
  if (!url) return null;

  const host = cleanHost(url.hostname);
  if (YOUTUBE_HOSTS.has(host)) {
    const id = youtubeVideoId(url);
    return id ? iframeDescriptor("youtube", `https://www.youtube-nocookie.com/embed/${id}`) : null;
  }

  if (VIMEO_HOSTS.has(host)) {
    const id = vimeoVideoId(url);
    return id ? iframeDescriptor("vimeo", `https://player.vimeo.com/video/${id}`) : null;
  }

  const extension = directMediaExtension(url);
  const origins = options.allowedDirectMediaOrigins ?? [];
  if (!extension || !allowedOrigin(url, origins)) return null;

  return {
    type: "video",
    provider: "direct",
    src: `${url.origin}${url.pathname}${url.search}`,
    controls: true,
    preload: "metadata",
  };
}
