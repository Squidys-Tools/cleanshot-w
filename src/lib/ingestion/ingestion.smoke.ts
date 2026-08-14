import { parseHTML } from "linkedom";
import { DefaultDefuddleAdapter } from "./defuddle-adapter";
import { extractFallback } from "./fallback";
import { collectSafeEmbeds, sanitizeHtml } from "./html-safety";
import { classifyFile } from "./file-classification";
import { assetRelativePath, sanitizeAssetSegment, thumbnailRelativePath } from "./asset-paths";
import { ingestUrl } from "./url-ingestion";
import { normalizeHttpUrl } from "./url";
import { sanitizeEmbedUrl } from "./safe-embeds";

function assert(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(`Smoke assertion failed: ${message}`);
}

const pageUrl = "https://example.com/articles/first";
const html = `
  <html><head>
    <title>Fallback title</title>
    <meta name="description" content="A useful description">
    <meta property="og:image" content="/images/cover.jpg">
    <link rel="canonical" href="/articles/canonical">
  </head><body>
    <article>
      <h1>Fallback title</h1>
      <p>This article has enough meaningful text to exercise both the normal Defuddle path and the fallback path.</p>
      <img src="photos/inline.jpg">
      <iframe src="https://www.youtube.com/watch?v=abc12345678" title="Video"></iframe>
      <iframe src="https://www.youtube.com/not-a-video"></iframe>
      <video src="https://www.youtube.com/watch?v=abc12345678"></video>
      <video src="/media/clip.mp4"></video>
    </article>
  </body></html>`;

const { document } = parseHTML(html);
const defuddle = new DefaultDefuddleAdapter().extract(document, pageUrl);
assert(defuddle.contentHtml?.includes("meaningful text"), "Defuddle extracts article content");
assert(defuddle.canonicalUrl === "https://example.com/articles/canonical", "Defuddle normalizes canonical URL");

const fallback = extractFallback(document, pageUrl);
assert(fallback.canonicalUrl === "https://example.com/articles/canonical", "fallback normalizes canonical URL");
assert(fallback.imageUrls?.includes("https://example.com/articles/photos/inline.jpg"), "fallback resolves relative image URL");

const safeEmbeds = collectSafeEmbeds(document, pageUrl);
assert(safeEmbeds.some((embed) => embed.provider === "youtube"), "valid YouTube iframe is allowlisted");
assert(!safeEmbeds.some((embed) => embed.sourceUrl.includes("not-a-video")), "malformed YouTube URL is rejected");
assert(!safeEmbeds.some((embed) => embed.kind === "video" && embed.sourceUrl.includes("youtube.com/watch")), "provider URL is not accepted as direct video media");

const sanitized = sanitizeHtml(`<iframe src="javascript:alert(1)"></iframe><video src="https://example.com/not-a-video"></video>`, pageUrl);
assert(!sanitized.includes("javascript:"), "unsafe iframe URL is removed");
assert(!sanitized.includes("not-a-video"), "unsupported direct video URL is removed");

const fetched = await ingestUrl(pageUrl, {
  fetch: async () => new Response(html, { headers: { "content-type": "text/html; charset=utf-8" } }),
});
assert(fetched.extractor === "defuddle", "ingestUrl reports Defuddle extraction");
assert(fetched.imageUrls.includes("https://example.com/images/cover.jpg"), "ingestion keeps relative Open Graph image");
assert(fetched.imageUrls.includes("https://example.com/articles/photos/inline.jpg"), "ingestion keeps relative inline image");

const fallbackFetched = await ingestUrl(pageUrl, {
  fetch: async () => new Response(html, { headers: { "content-type": "text/html" } }),
  defuddleAdapter: { name: "failing-test-adapter", extract: () => { throw new Error("expected test failure"); } },
});
assert(fallbackFetched.extractor === "fallback", "ingestion falls back when Defuddle fails");

assert(normalizeHttpUrl("../cover.jpg", pageUrl) === "https://example.com/cover.jpg", "relative URL normalization works");
assert(sanitizeEmbedUrl("https://youtu.be/abc12345678")?.provider === "youtube", "standalone YouTube policy allows valid URL");
assert(sanitizeEmbedUrl("https://videos.example/clip.mp4", { allowedDirectMediaOrigins: ["https://videos.example"] })?.type === "video", "standalone direct-media policy allows exact origin");
assert(sanitizeEmbedUrl("https://videos.example/clip.mp4", { allowedDirectMediaOrigins: ["https://other.example"] }) === null, "standalone direct-media policy rejects unknown origin");
assert(sanitizeEmbedUrl("javascript:alert(1)") === null, "standalone embed policy rejects javascript URL");

assert(classifyFile({ name: "photo.pdf", type: "image/png" }) === "image", "recognized MIME type beats misleading extension");
assert(classifyFile({ name: "clip.bin", type: "video/mp4; codecs=h264" }) === "video", "video MIME type is recognized");
assert(classifyFile({ name: "document.PDF", type: "" }) === "pdf", "PDF extension fallback works");
assert(thumbnailRelativePath("item-1", "image") === "items/item-1/thumbnail.webp", "image thumbnail policy is stable");
assert(thumbnailRelativePath("item-1", "other") === null, "other files do not get thumbnails");
assert(assetRelativePath({ itemId: "../item", variant: "original", extension: ".jpg" }) === "items/-item/original.jpg", "asset paths remain relative");
assert(sanitizeAssetSegment("", "../unsafe fallback") === "-unsafe-fallback", "fallback asset segment is sanitized");

console.log("Ingestion smoke checks passed.");
