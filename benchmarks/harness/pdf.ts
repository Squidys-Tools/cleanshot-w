import { readFileSync } from "node:fs";

export interface PdfExtraction {
  engine: "naive-streams";
  text: string;
  embeddedImages: number;
}

/** Decode a PDF literal string, handling \n \t \\ ( ) and octal escapes. */
function decodePdfString(raw: string): string {
  let out = "";
  for (let i = 0; i < raw.length; i++) {
    const ch = raw[i];
    if (ch !== "\\") {
      out += ch;
      continue;
    }
    const next = raw[i + 1];
    if (next === "n") { out += "\n"; i++; }
    else if (next === "r") { out += "\r"; i++; }
    else if (next === "t") { out += "\t"; i++; }
    else if (next === "b") { out += "\b"; i++; }
    else if (next === "f") { out += "\f"; i++; }
    else if (next === "(" || next === ")" || next === "\\") { out += next; i++; }
    else if (next >= "0" && next <= "7") {
      let octal = "";
      let j = i + 1;
      while (j < raw.length && octal.length < 3 && raw[j] >= "0" && raw[j] <= "7") {
        octal += raw[j];
        j++;
      }
      out += String.fromCharCode(parseInt(octal, 8));
      i = j - 1;
    } else {
      out += next;
      i++;
    }
  }
  return out;
}

/**
 * A minimal reference PDF text extractor that decodes content-stream literal
 * strings (`(text) Tj` and `[...] TJ`). Intended for the hand-generated ASCII
 * PDFs in the benchmark corpus; real-world PDFs need a full parser
 * (pdf-extract / pdfjs) in a later milestone.
 */
export function extractPdf(filePath: string): PdfExtraction {
  const latin1 = readFileSync(filePath).toString("latin1");

  const streams: string[] = [];
  const streamRe = /stream\r?\n/g;
  let match: RegExpExecArray | null;
  while ((match = streamRe.exec(latin1)) !== null) {
    const start = match.index + match[0].length;
    const end = latin1.indexOf("endstream", start);
    if (end === -1) break;
    streams.push(latin1.slice(start, end));
    streamRe.lastIndex = end;
  }

  const contentStreams = streams.filter((stream) => /\b(Tj|TJ|Tf|Tm)\b/.test(stream));
  const tokens: string[] = [];

  for (const stream of contentStreams) {
    const tjRe = /\(((?:[^()\\]|\\.)*)\)\s*Tj/g;
    let t: RegExpExecArray | null;
    while ((t = tjRe.exec(stream)) !== null) {
      tokens.push(decodePdfString(t[1]));
    }

    const arrRe = /\[((?:\((?:(?:[^()\\]|\\.)*)\)|[^[\]])*)\]\s*TJ/g;
    while ((t = arrRe.exec(stream)) !== null) {
      const litRe = /\(((?:[^()\\]|\\.)*)\)/g;
      let l: RegExpExecArray | null;
      while ((l = litRe.exec(t[1])) !== null) {
        tokens.push(decodePdfString(l[1]));
      }
    }
  }

  const embeddedImages = (latin1.match(/\/Subtype\s*\/Image\b/g) ?? []).length;
  return {
    engine: "naive-streams",
    text: tokens.join(" ").replace(/\s+/g, " ").trim(),
    embeddedImages,
  };
}

/**
 * Extracts the bytes of the first `/DCTDecode` (JPEG) image stream in a PDF.
 * The benchmark generator embeds raw JPEG files verbatim, so the stream bytes
 * are a complete JPEG. Returns null if there is no DCTDecode image.
 */
export function extractFirstEmbeddedJpeg(filePath: string): Buffer | null {
  const latin1 = readFileSync(filePath).toString("latin1");
  const marker = latin1.indexOf("/DCTDecode");
  if (marker === -1) return null;

  const tail = latin1.slice(marker);
  const lenMatch = /\/Length\s+(\d+)\b/.exec(tail);
  if (!lenMatch) return null;
  const length = parseInt(lenMatch[1], 10);

  const streamIdx = tail.indexOf("\nstream\n");
  if (streamIdx === -1) return null;
  const start = marker + streamIdx + "\nstream\n".length;
  if (start + length > latin1.length) return null;

  return Buffer.from(latin1.slice(start, start + length), "latin1");
}
