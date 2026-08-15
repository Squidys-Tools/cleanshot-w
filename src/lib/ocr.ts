import Tesseract from "tesseract.js";

type Progress = { status: string; progress: number };

let workerPromise: Promise<Tesseract.Worker> | null = null;

async function getWorker(onProgress?: (p: Progress) => void): Promise<Tesseract.Worker> {
  if (!workerPromise) {
    workerPromise = Tesseract.createWorker("eng", 1, {
      langPath: "/tessdata",
      workerPath: "/tessdata/worker.min.js",
      corePath: "/tessdata",
      gzip: false,
      logger: (m) => {
        if (m.status === "recognizing text" || m.status === "loading language traineddata" || m.status === "initializing tesseract") {
          onProgress?.({ status: m.status, progress: m.progress });
        }
      },
    }).catch((err) => {
      workerPromise = null;
      throw err;
    });
  }
  return workerPromise;
}

export type OcrResult = {
  text: string;
  confidence: number;
  words: { text: string; x0: number; y0: number; x1: number; y1: number; confidence: number }[];
};

export async function recognizeText(blob: Blob, onProgress?: (p: Progress) => void): Promise<OcrResult> {
  const worker = await getWorker(onProgress);
  const { data } = await worker.recognize(blob);
  const words =
    data.blocks?.flatMap((b) =>
      b.paragraphs.flatMap((p) =>
        p.lines.flatMap((l) => l.words.map((w) => ({ text: w.text, ...w.bbox, confidence: w.confidence }))),
      ),
    ) ?? [];
  return { text: data.text ?? "", confidence: data.confidence ?? 0, words };
}

export async function terminateOcr(): Promise<void> {
  if (workerPromise) {
    const w = await workerPromise;
    await w.terminate();
    workerPromise = null;
  }
}
