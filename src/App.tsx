import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CaptureRecord, TldrawState } from "./types";
import { uid } from "./types";
import { host, setHost } from "./lib/bridge";
import { nativeHost, readNativeClipboardImage } from "./lib/nativeHost";
import { loadImage, makeThumb } from "./lib/storage";
import { downloadBlob, flattenToBlob, sanitizeFileName } from "./lib/export";
import {
  captureFrameToBlob,
  captureWindow as captureNativeWindow,
  ensureCaptureHotkey,
  getSettings,
  isTauriRuntime,
  listCaptureWindows,
  setCaptureHotkey,
  startAreaCapture,
  type NativeSettings,
  type NativeCaptureFrame,
  type NativeWindowInfo,
} from "./lib/nativeCapture";
import type { Exporter } from "./lib/tldrawDoc";
import CaptureOverlay from "./components/CaptureOverlay";
import Editor from "./components/Editor";
import Dropzone from "./components/Dropzone";
import HistoryRail from "./components/HistoryRail";
import QuickAccess, { type OcrStatus } from "./components/QuickAccess";
import SettingsPopover from "./components/SettingsPopover";
import WindowPicker from "./components/WindowPicker";
import "./App.css";

type OcrState = { status: OcrStatus; text?: string; progress?: number; message?: string };

function defaultTitle(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `Capture ${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function EditorApp() {
  const [rec, setRec] = useState<CaptureRecord | null>(null);
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [history, setHistory] = useState<CaptureRecord[]>([]);
  const [showHistory, setShowHistory] = useState(true);
  const [ocr, setOcr] = useState<OcrState>({ status: "idle" });
  const [copied, setCopied] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [settings, setSettings] = useState<NativeSettings | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsSaving, setSettingsSaving] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [windowPickerOpen, setWindowPickerOpen] = useState(false);
  const [captureWindows, setCaptureWindows] = useState<NativeWindowInfo[]>([]);
  const [windowBusy, setWindowBusy] = useState(false);
  const [windowError, setWindowError] = useState<string | null>(null);

  const urlRef = useRef<string | null>(null);
  const recRef = useRef<CaptureRecord | null>(null);
  const exporterRef = useRef<Exporter | null>(null);
  const saveTimers = useRef(new Map<string, { version: number; timer: number }>());
  const fileInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isTauriRuntime()) setHost(nativeHost);
  }, []);

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

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    void listen<NativeCaptureFrame>("capture:completed", (event) => {
      if (disposed) return;
      void captureFrameToBlob(event.payload)
        .then((blob) => newCapture(blob))
        .catch(() => setNotice("Could not open the captured area in the editor."));
    }).then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [newCapture]);

  const startCapture = useCallback(() => {
    if (!isTauriRuntime()) {
      fileInputRef.current?.click();
      return;
    }
    setNotice(null);
    void startAreaCapture().catch(() => setNotice("Could not start screen capture."));
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let disposed = false;
    let unlisten: UnlistenFn | undefined;
    void getSettings()
      .then((value) => {
        if (!disposed) setSettings(value);
      })
      .catch(() => {
        if (!disposed) setNotice("Could not load capture settings.");
      });
    void ensureCaptureHotkey().catch(() => {
      if (!disposed) setNotice("Could not register the global capture shortcut.");
    });
    void listen<string>("global-hotkey-pressed", () => {
      if (!disposed) startCapture();
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [startCapture]);

  const openWindowPicker = useCallback(() => {
    if (!isTauriRuntime()) return;
    setWindowPickerOpen(true);
    setWindowBusy(true);
    setWindowError(null);
    void listCaptureWindows()
      .then((items) => setCaptureWindows(items))
      .catch(() => setWindowError("Could not list the open windows."))
      .finally(() => setWindowBusy(false));
  }, []);

  const captureSelectedWindow = useCallback(async (windowId: string) => {
    setWindowBusy(true);
    setWindowError(null);
    try {
      const frame = await captureNativeWindow(windowId);
      const blob = await captureFrameToBlob(frame);
      await newCapture(blob);
      setWindowPickerOpen(false);
    } catch {
      setWindowError("Could not capture that window. It may have closed or become unavailable.");
    } finally {
      setWindowBusy(false);
    }
  }, [newCapture]);

  const saveSettings = useCallback(async (captureHotkey: string) => {
    setSettingsSaving(true);
    setSettingsError(null);
    try {
      const value = await setCaptureHotkey(captureHotkey);
      setSettings(value);
      setSettingsOpen(false);
      setNotice(`Shortcut saved: ${value.captureHotkey}`);
    } catch (error) {
      setSettingsError(String(error));
    } finally {
      setSettingsSaving(false);
    }
  }, []);

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

  const copyFile = useCallback(async () => {
    if (!rec) return;
    setNotice(null);
    try {
      const exporter = exporterRef.current;
      const { blob } = exporter ? await exporter() : await flattenToBlob(rec);
      await host.copyFile(blob, `${sanitizeFileName(rec.title)}.png`);
      setNotice("PNG copied to the clipboard as a file.");
    } catch {
      setNotice("Could not copy the PNG file. Check clipboard permissions and try again.");
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
      if (isTauriRuntime()) {
        const blob = await readNativeClipboardImage();
        if (blob) {
          await newCapture(blob);
          return;
        }
        setNotice("The clipboard has no image.");
        return;
      }
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
          <button className="btn primary" onClick={startCapture}>
            New capture
          </button>
          {isTauriRuntime() && (
            <button className="btn" onClick={openWindowPicker}>
              Window
            </button>
          )}
          <button className="btn" onClick={() => setShowHistory((s) => !s)}>
            History {history.length > 0 ? `(${history.length})` : ""}
          </button>
          {isTauriRuntime() && (
            <button className="btn" onClick={() => { setSettingsError(null); setSettingsOpen((open) => !open); }}>
              Settings
            </button>
          )}
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
              onPick={startCapture}
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
          onCopyFile={copyFile}
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

      {settingsOpen && settings && (
        <SettingsPopover
          hotkey={settings.captureHotkey}
          saving={settingsSaving}
          error={settingsError}
          onSave={saveSettings}
          onClose={() => setSettingsOpen(false)}
        />
      )}

      {windowPickerOpen && (
        <WindowPicker
          windows={captureWindows}
          busy={windowBusy}
          error={windowError}
          onSelect={(windowId) => void captureSelectedWindow(windowId)}
          onClose={() => setWindowPickerOpen(false)}
        />
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

function App() {
  const isOverlay = new URLSearchParams(window.location.search).get("overlay") === "capture";
  return isOverlay ? <CaptureOverlay /> : <EditorApp />;
}

export default App;
