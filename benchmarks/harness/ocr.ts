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

/** Optional OCR engine backed by tesseract.js (add as devDependency to enable). */
export class TesseractJsEngine implements OcrEngine {
  name = "tesseract.js";

  async available(): Promise<boolean> {
    try {
      await import("tesseract.js");
      return true;
    } catch {
      return false;
    }
  }

  async run(imagePath: string): Promise<string> {
    const Tesseract = await import("tesseract.js");
    const result = await Tesseract.recognize(imagePath, "eng");
    return result.data.text;
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
