import { existsSync, statSync } from "node:fs";
import { resolve, sep } from "node:path";

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".md": "text/plain; charset=utf-8",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".pdf": "application/pdf",
  ".txt": "text/plain; charset=utf-8",
};

export interface CorpusServer {
  baseUrl: string;
  stop(): void;
}

/**
 * Serves the benchmark corpus over localhost so the real ingestion pipeline
 * (http/https URLs only) can fetch fixtures exactly as it would fetch a live page.
 */
export async function startCorpusServer(root: string): Promise<CorpusServer> {
  const rootResolved = resolve(root);
  const server = Bun.serve({
    port: 0,
    fetch(request) {
      const url = new URL(request.url);
      const rel = decodeURIComponent(url.pathname)
        .replace(/^[/\\]+/, "")
        .replace(/\//g, sep);
      const filePath = resolve(rootResolved, rel);
      if (!filePath.startsWith(rootResolved + sep) && filePath !== rootResolved) {
        return new Response("forbidden", { status: 403 });
      }
      if (!existsSync(filePath) || statSync(filePath).isDirectory()) {
        return new Response("not found", { status: 404 });
      }
      const extension = filePath.slice(filePath.lastIndexOf(".")).toLowerCase();
      return new Response(Bun.file(filePath), {
        headers: { "Content-Type": MIME[extension] ?? "application/octet-stream" },
      });
    },
  });
  return {
    baseUrl: `http://127.0.0.1:${server.port}`,
    stop: () => server.stop(true),
  };
}
