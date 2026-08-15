import { resolve } from "node:path";

export interface OcrEngine {
  name: string;
  available(): Promise<boolean>;
  run(imagePath: string): Promise<string>;
}

/**
 * Native Windows OCR (Windows.Media.Ocr) invoked through Windows PowerShell
 * 5.1, which is the only host that can project WinRT types. Zero-install on
 * any Windows machine with the OCR language pack.
 */
export class WindowsOcrEngine implements OcrEngine {
  name = "windows-ocr";
  private checked = false;
  private ready = false;

  constructor(private readonly scriptPath: string) {}

  async available(): Promise<boolean> {
    if (this.checked) return this.ready;
    this.checked = true;
    if (process.platform !== "win32") return false;
    try {
      const result = Bun.spawnSync([
        "powershell.exe",
        "-NoProfile",
        "-Command",
        "& { $null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType=WindowsRuntime]; exit 0 }",
      ]);
      this.ready = result.exitCode === 0;
    } catch {
      this.ready = false;
    }
    return this.ready;
  }

  async run(imagePath: string): Promise<string> {
    const result = Bun.spawnSync(
      ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", this.scriptPath, "-ImagePath", imagePath],
      { stdout: "pipe", stderr: "pipe" },
    );
    if (result.exitCode !== 0) {
      throw new Error(`Windows OCR failed (exit ${result.exitCode}): ${result.stderr.toString("utf8")}`);
    }
    return result.stdout.toString("utf8").trim();
  }
}

/** OCR engine backed by tesseract.js (devDependency). */
export class TesseractJsEngine implements OcrEngine {
  name = "tesseract.js";
  private modules: string | null = null;

  async available(): Promise<boolean> {
    try {
      await import("tesseract.js");
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Runs recognition with several page-segmentation modes and returns the
   * highest-confidence text. Tries modes that suit full pages, blocks, and
   * single lines so stylized images (memes, rotated scans) have a chance.
   */
  async run(imagePath: string): Promise<string> {
    // The tesseract worker resolves `regenerator-runtime/runtime` through
    // NODE_PATH; without it recognition silently returns empty text. Point it
    // at the project's node_modules before the first import.
    if (this.modules === null) {
      const projectModules = resolve(import.meta.dir, "../..", "node_modules");
      const existing = process.env.NODE_PATH ?? "";
      this.modules = existing.split(/[;:]/).includes(projectModules)
        ? existing
        : existing ? `${existing}${process.platform === "win32" ? ";" : ":"}${projectModules}` : projectModules;
      process.env.NODE_PATH = this.modules;
    }

    const Tesseract = await import("tesseract.js");
    let best = "";
    let bestConfidence = -1;
    for (const psm of [3, 6, 7]) {
      try {
        // tesseract.js 7 ships `data: Page` in its types, but tsc 7 resolves it
        // as `{ text: string }`; the runtime object has confidence too.
        const result = (await Tesseract.recognize(imagePath, "eng", { psm })) as {
          data: { text: string; confidence: number };
        };
        if (result.data.confidence > bestConfidence) {
          bestConfidence = result.data.confidence;
          best = result.data.text ?? "";
        }
      } catch {
        // try the next page-segmentation mode
      }
    }
    return best;
  }
}

export async function pickOcrEngine(engines: OcrEngine[]): Promise<OcrEngine | null> {
  for (const engine of engines) {
    try {
      if (await engine.available()) return engine;
    } catch {
      // keep probing
    }
  }
  return null;
}

export async function availableOcrEngines(engines: OcrEngine[]): Promise<OcrEngine[]> {
  const ready: OcrEngine[] = [];
  for (const engine of engines) {
    try {
      if (await engine.available()) ready.push(engine);
    } catch {
      // engine not usable; skip it
    }
  }
  return ready;
}
