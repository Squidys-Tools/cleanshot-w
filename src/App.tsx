import { useCallback, useEffect, useRef, useState } from "react";
import type { CaptureRecord, TldrawState } from "./types";
import { uid } from "./types";
import { host } from "./lib/bridge";
import { loadImage, makeThumb } from "./lib/storage";
import { downloadBlob, flattenToBlob, sanitizeFileName } from "./lib/export";
import type { Exporter } from "./lib/tldrawDoc";
import Editor from "./components/Editor";
import Dropzone from "./components/Dropzone";
import HistoryRail from "./components/HistoryRail";
import QuickAccess, { type OcrStatus } from "./components/QuickAccess";
import "./App.css";

type OcrState = { status: OcrStatus; text?: string; progress?: number; message?: string };

function defaultTitle(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `Capture ${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function App() {
  const [rec, setRec] = useState<CaptureRecord | null>(null);
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [history, setHistory] = useState<CaptureRecord[]>([]);
  const [showHistory, setShowHistory] = useState(true);
  const [ocr, setOcr] = useState<OcrState>({ status: "idle" });
  const [copied, setCopied] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const urlRef = useRef<string | null>(null);
  const recRef = useRef<CaptureRecord | null>(null);
  const exporterRef = useRef<Exporter | null>(null);
  const saveTimers = useRef(new Map<string, { version: number; timer: number }>());
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    recRef.current = rec;
  }, [rec]);

  const refreshHistory = useCallback(async () => {
    setHistory(await host.listCaptures());
  }, []);

  useEffect(() => {
    refreshHistory().catch(() => setNotice("Could not open the local library."));
  }, [refreshHistory]);

  const openCapture = useCallback((r: CaptureRecord) => {
    if (urlRef.current) URL.revokeObjectURL(urlRef.current);
    const url = URL.createObjectURL(r.imageBlob);
    urlRef.current = url;
    setImageUrl(url);
    setRec(r);
    setOcr({ status: "idle" });
    setCopied(false);
    setNotice(null);
  }, []);

  const newCapture = useCallback(
    async (blob: Blob, name?: string) => {
      setBusy(true);
      setNotice(null);
      try {
        const dims = await loadImage(blob);
        const thumb = await makeThumb(blob);
        const r: CaptureRecord = {
          id: uid(),
          title: name && /\.\w+$/.test(name) ? name.replace(/\.\w+$/, "") : name || defaultTitle(),
          createdAt: Date.now(),
          updatedAt: Date.now(),
          imageBlob: blob,
          thumbBlob: thumb,
          image: dims,
          annotations: null,
        };
        await host.saveCapture(r);
        openCapture(r);
        await refreshHistory();
      } catch (e) {
        setNotice(`Could not load that image: ${String(e)}`);
      } finally {
        setBusy(false);
      }
    },
    [openCapture, refreshHistory],
  );

  const persistAnnotations = useCallback(
    async (recordId: string, annotations: TldrawState, version: number) => {
      const s = saveTimers.current.get(recordId);
      if (!s || s.version !== version) return;
      await host.updateCaptureAnnotations(recordId, annotations);
      if (saveTimers.current.get(recordId)?.version === version) {
        saveTimers.current.delete(recordId);
        setHistory(await host.listCaptures());
      }
    },
    [],
  );

  const onAnnotationsChange = useCallback(
    (recordId: string, annotations: TldrawState) => {
      setRec((r) => (r && r.id === recordId ? { ...r, annotations, updatedAt: Date.now() } : r));
      const prev = saveTimers.current.get(recordId);
      if (prev) window.clearTimeout(prev.timer);
      const version = (prev?.version ?? 0) + 1;
      const timer = window.setTimeout(() => {
        void persistAnnotations(recordId, annotations, version);
      }, 400);
      saveTimers.current.set(recordId, { version, timer });
    },
    [persistAnnotations],
  );

  const closeCapture = useCallback(() => {
    if (urlRef.current) URL.revokeObjectURL(urlRef.current);
    urlRef.current = null;
    setImageUrl(null);
    setRec(null);
    setOcr({ status: "idle" });
    setNotice(null);
  }, []);

  const deleteCapture = useCallback(
    async (id: string) => {
      const s = saveTimers.current.get(id);
      if (s) {
        window.clearTimeout(s.timer);
        saveTimers.current.delete(id);
      }
      await host.deleteCapture(id);
      await refreshHistory();
      if (recRef.current?.id === id) closeCapture();
    },
    [refreshHistory, closeCapture],
  );

  const copyImage = useCallback(async () => {
    if (!rec) return;
    setNotice(null);
    try {
      const exporter = exporterRef.current;
      const { blob } = exporter ? await exporter() : await flattenToBlob(rec);
      await host.copyImage(blob);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      setNotice("Could not copy the image. Check clipboard permissions and try again.");
    }
  }, [rec]);

  const savePng = useCallback(async () => {
    if (!rec) return;
    setNotice(null);
    try {
      const exporter = exporterRef.current;
      const { blob } = exporter ? await exporter() : await flattenToBlob(rec);
      downloadBlob(blob, `${sanitizeFileName(rec.title)}.png`);
    } catch {
      setNotice("Could not export the PNG. Try again.");
    }
  }, [rec]);

  const runOcr = useCallback(async () => {
    if (!rec) return;
    setOcr({ status: "running", progress: 0 });
    try {
      const res = await host.recognize(rec.imageBlob, (p) => setOcr({ status: "running", progress: p.progress }));
      setOcr({ status: "done", text: res.text });
      setNotice(null);
    } catch (e) {
      setOcr({ status: "error", message: String(e) });
      setNotice("OCR could not read this image. Try again or use a clearer capture.");
    }
  }, [rec]);

  const copyOcrText = useCallback(async () => {
    if (!ocr.text) return;
    try {
      await host.copyText(ocr.text);
      setNotice(null);
    } catch {
      setNotice("Could not copy the recognized text. Check clipboard permissions and try again.");
    }
  }, [ocr.text]);

  const readClipboard = useCallback(async () => {
    setNotice(null);
    try {
      const items = await navigator.clipboard.read();
      for (const it of items) {
        const t = it.types.find((x) => x.startsWith("image/"));
        if (t) {
          const blob = await it.getType(t);
          await newCapture(blob);
          return;
        }
      }
      setNotice("The clipboard has no image.");
    } catch {
      setNotice("Clipboard read was blocked. Press Ctrl+V to paste instead.");
    }
  }, [newCapture]);

  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      const items = e.clipboardData?.items;
      if (!items) return;
      for (const it of items) {
        if (it.kind === "file" && it.type.startsWith("image/")) {
          const f = it.getAsFile();
          if (f) {
            e.preventDefault();
            newCapture(f);
            return;
          }
        }
      }
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [newCapture]);

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark">⌁</span>
          <span>
            CleanShot <em>W</em>
          </span>
        </div>
        <div className="top-actions">
          <button className="btn primary" onClick={() => fileInputRef.current?.click()}>
            New capture
          </button>
          <button className="btn" onClick={() => setShowHistory((s) => !s)}>
            History {history.length > 0 ? `(${history.length})` : ""}
          </button>
        </div>
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          hidden
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) newCapture(f, f.name);
            e.target.value = "";
          }}
        />
      </header>

      <div className="workspace">
        <main className="stage">
          {rec && imageUrl ? (
            <Editor key={rec.id} doc={rec} imageUrl={imageUrl} onChange={onAnnotationsChange} exportRef={exporterRef} />
          ) : (
            <Dropzone
              onFile={(blob, name) => newCapture(blob, name)}
              onPick={() => fileInputRef.current?.click()}
              onReadClipboard={readClipboard}
              busy={busy}
              notice={notice}
            />
          )}
        </main>
        {showHistory && (
          <HistoryRail records={history} currentId={rec?.id ?? null} onOpen={openCapture} onDelete={deleteCapture} />
        )}
      </div>

      {rec && (
        <QuickAccess
          onCopyImage={copyImage}
          onSavePng={savePng}
          onOcr={runOcr}
          onNew={() => {
            closeCapture();
            setShowHistory(true);
          }}
          onClose={closeCapture}
          ocrStatus={ocr.status}
          ocrProgress={ocr.progress}
          copied={copied}
          notice={notice}
        />
      )}

      {ocr.status === "done" && ocr.text && (
        <div className="ocr-panel">
          <div className="ocr-panel-head">
            <strong>Recognized text</strong>
            <button className="link-btn" onClick={copyOcrText}>
              Copy
            </button>
            <button className="link-btn" onClick={() => setOcr({ status: "idle" })}>
              Dismiss
            </button>
          </div>
          <pre>{ocr.text}</pre>
        </div>
      )}

      {ocr.status === "error" && (
        <div className="ocr-panel error">
          <div className="ocr-panel-head">
            <strong>OCR failed</strong>
            <button className="link-btn" onClick={() => setOcr({ status: "idle" })}>
              Dismiss
            </button>
          </div>
          <p>{ocr.message}</p>
        </div>
      )}
    </div>
  );
}

export default App;
