/**
 * Minimal ambient types for the optional tesseract.js OCR engine.
 * tesseract.js is intentionally not installed by default (the harness uses
 * the zero-install Windows.Media.Ocr engine when available); these types keep
 * the harness typecheckable without forcing the heavy dependency.
 */
declare module "tesseract.js" {
  export interface TesseractResult {
    data: {
      text: string;
    };
  }

  export function recognize(
    image: string | Uint8Array | Blob,
    lang?: string,
    options?: Record<string, unknown>,
  ): Promise<TesseractResult>;
}
