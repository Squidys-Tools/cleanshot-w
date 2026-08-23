import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CaptureRecord, TldrawState } from "./types";
import { uid } from "./types";
import { host, setHost } from "./lib/bridge";
import { nativeHost, readNativeClipboardImage } from "./lib/nativeHost";
import { loadImage, makeThumb } from "./lib/storage";
import { normalizeCaptureTitle } from "./lib/history";
import { downloadBlob, flattenToBlob, sanitizeFileName } from "./lib/export";
import {
  blobToBase64,
  captureFullscreen,
  captureFrameToBlob,
  captureWindow as captureNativeWindow,
  ensureCaptureHotkey,
  getSettings,
  isTauriRuntime,
  listCaptureWindows,
  nativeErrorMessage,
  setCaptureSettings,
  showPinnedCapture,
  startAreaCapture,
  type NativeSettings,
  type NativeCaptureFrame,
  type NativeWindowInfo,
} from "./lib/nativeCapture";
import type { EditorController } from "./lib/tldrawDoc";
import CaptureOverlay from "./components/CaptureOverlay";
import Editor from "./components/Editor";
import Dropzone from "./components/Dropzone";
import HistoryRail from "./components/HistoryRail";
import type { OcrStatus } from "./components/QuickAccess";
import SettingsPopover from "./components/SettingsPopover";
import WindowPicker from "./components/WindowPicker";
import PinnedCapture from "./components/PinnedCapture";
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
  const [showHistory, setShowHistory] = useState(false);
  const [historyState, setHistoryState] = useState({ canUndo: false, canRedo: false });
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
  const controllerRef = useRef<EditorController | null>(null);
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
    refreshHistory().catch((error: unknown) => setNotice(nativeErrorMessage(error, "Could not open the local library.")));
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
      } catch (e: unknown) {
        setNotice(`Could not load that image: ${nativeErrorMessage(e, "The image could not be loaded.")}`);
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
        .catch((error: unknown) => setNotice(nativeErrorMessage(error, "Could not open the captured area in the editor.")));
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
    void startAreaCapture().catch((error: unknown) => setNotice(nativeErrorMessage(error, "Could not start screen capture.")));
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let disposed = false;
    let unlistenHotkey: UnlistenFn | undefined;
    let unlistenTray: UnlistenFn | undefined;
    void getSettings()
      .then((value) => {
        if (!disposed) setSettings(value);
      })
      .catch((error: unknown) => {
        if (!disposed) setNotice(nativeErrorMessage(error, "Could not load capture settings."));
      });
    void ensureCaptureHotkey().catch((error: unknown) => {
      if (!disposed) setNotice(nativeErrorMessage(error, "Could not register the global capture shortcut."));
    });
    void listen<string>("global-hotkey-pressed", () => {
      if (!disposed) startCapture();
    }).then((stop) => {
      if (disposed) stop();
      else unlistenHotkey = stop;
    });
    void listen("tray-new-capture", () => {
      if (!disposed) startCapture();
    }).then((stop) => {
      if (disposed) stop();
      else unlistenTray = stop;
    });

    return () => {
      disposed = true;
      unlistenHotkey?.();
      unlistenTray?.();
    };
  }, [startCapture]);

  const openWindowPicker = useCallback(() => {
    if (!isTauriRuntime()) return;
    setWindowPickerOpen(true);
    setWindowBusy(true);
    setWindowError(null);
    void listCaptureWindows()
      .then((items) => setCaptureWindows(items))
      .catch((error: unknown) => setWindowError(nativeErrorMessage(error, "Could not list the open windows.")))
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
    } catch (error: unknown) {
      setWindowError(nativeErrorMessage(error, "Could not capture that window. It may have closed or become unavailable."));
    } finally {
      setWindowBusy(false);
    }
  }, [newCapture]);

  const captureFullScreen = useCallback(async () => {
    setBusy(true);
    setNotice(null);
    try {
      const frame = await captureFullscreen();
      await newCapture(await captureFrameToBlob(frame));
    } catch (error: unknown) {
      setNotice(nativeErrorMessage(error, "Could not capture the full screen."));
    } finally {
      setBusy(false);
    }
  }, [newCapture]);

  const saveSettings = useCallback(async (captureHotkey: string, includeCursor: boolean, launchAtStartup: boolean) => {
    setSettingsSaving(true);
    setSettingsError(null);
    try {
      const value = await setCaptureSettings(captureHotkey, includeCursor, launchAtStartup);
      setSettings(value);
      setSettingsOpen(false);
      setNotice("Capture settings saved.");
    } catch (error: unknown) {
      setSettingsError(nativeErrorMessage(error, "Could not save capture settings."));
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
        void persistAnnotations(recordId, annotations, version).catch((error: unknown) => {
          setNotice(nativeErrorMessage(error, "Could not save the capture annotations."));
        });
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
    setShowHistory(false);
    setHistoryState({ canUndo: false, canRedo: false });
  }, []);

  const onHistoryState = useCallback((next: { canUndo: boolean; canRedo: boolean }) => {
    setHistoryState((current) => (current.canUndo === next.canUndo && current.canRedo === next.canRedo ? current : next));
  }, []);

  const deleteCapture = useCallback(
    async (id: string) => {
      const s = saveTimers.current.get(id);
      if (s) {
        window.clearTimeout(s.timer);
        saveTimers.current.delete(id);
      }
      try {
        await host.deleteCapture(id);
        await refreshHistory();
        if (recRef.current?.id === id) closeCapture();
      } catch (error: unknown) {
        setNotice(nativeErrorMessage(error, "Could not delete that capture."));
      }
    },
    [refreshHistory, closeCapture],
  );

  const renameCapture = useCallback(
    async (id: string, title: string): Promise<boolean> => {
      const nextTitle = normalizeCaptureTitle(title);
      if (!nextTitle) {
        setNotice("Capture titles cannot be empty.");
        return false;
      }
      try {
        const updated = await host.updateCaptureTitle(id, nextTitle);
        if (!updated) {
          setNotice("That capture is no longer in the local library.");
          return false;
        }
        setRec((current) => (current && current.id === id ? { ...current, title: nextTitle, updatedAt: Date.now() } : current));
        await refreshHistory();
        setNotice(null);
        return true;
      } catch (error: unknown) {
        setNotice(nativeErrorMessage(error, "Could not rename that capture."));
        return false;
      }
    },
    [refreshHistory],
  );

  const copyImage = useCallback(async () => {
    if (!rec) return;
    setNotice(null);
    try {
      const exporter = controllerRef.current?.exportImage;
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
      const exporter = controllerRef.current?.exportImage;
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
      const exporter = controllerRef.current?.exportImage;
      const { blob } = exporter ? await exporter() : await flattenToBlob(rec);
      downloadBlob(blob, `${sanitizeFileName(rec.title)}.png`);
    } catch {
      setNotice("Could not export the PNG. Try again.");
    }
  }, [rec]);

  const pinCapture = useCallback(async () => {
    if (!rec || !isTauriRuntime()) return;
    setNotice(null);
    try {
      const exporter = controllerRef.current?.exportImage;
      const { blob, width, height } = exporter ? await exporter() : await flattenToBlob(rec);
      await showPinnedCapture(await blobToBase64(blob), width, height, rec.title);
      setNotice("Capture pinned above other windows.");
    } catch (error: unknown) {
      setNotice(nativeErrorMessage(error, "Could not pin this capture."));
    }
  }, [rec]);

  const runOcr = useCallback(async () => {
    if (!rec) return;
    setOcr({ status: "running", progress: 0 });
    try {
      const res = await host.recognize(rec.imageBlob, (p) => setOcr({ status: "running", progress: p.progress }));
      setOcr({ status: "done", text: res.text });
      setNotice(null);
    } catch (e: unknown) {
      setOcr({ status: "error", message: nativeErrorMessage(e, "OCR could not read this image.") });
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
          <span className="brand-mark" aria-hidden="true"><span /></span>
          <span>
            CleanShot <em>W</em>
          </span>
        </div>
        <div className="command-actions" aria-label="Edit commands">
          <button
            className="command-icon"
            onClick={() => controllerRef.current?.undo()}
            disabled={!rec || !historyState.canUndo}
            title="Undo (Ctrl+Z)"
            aria-label="Undo"
          >
            <CommandIcon name="undo" />
          </button>
          <button
            className="command-icon"
            onClick={() => controllerRef.current?.redo()}
            disabled={!rec || !historyState.canRedo}
            title="Redo (Ctrl+Y)"
            aria-label="Redo"
          >
            <CommandIcon name="redo" />
          </button>
        </div>
        <div className="command-separator" />
        <nav className="top-actions" aria-label="Capture and export commands">
          <button className="command-btn primary" onClick={startCapture} disabled={busy}>
            New capture
          </button>
          {isTauriRuntime() && (
            <>
              <button className="command-btn" onClick={openWindowPicker}>
                Window
              </button>
              <button className="command-btn" onClick={() => void captureFullScreen()} disabled={busy}>
                Full screen
              </button>
            </>
          )}
          <button className="command-btn" onClick={copyImage} disabled={!rec}>
            {copied ? "Copied" : "Copy image"}
          </button>
          <button className="command-btn" onClick={copyFile} disabled={!rec}>
            Copy file
          </button>
          <button className="command-btn" onClick={savePng} disabled={!rec}>
            Save PNG
          </button>
          <button className="command-btn" onClick={runOcr} disabled={!rec || ocr.status === "running"}>
            {ocr.status === "running" ? `OCR ${Math.round((ocr.progress ?? 0) * 100)}%` : "Copy text (OCR)"}
          </button>
          {isTauriRuntime() && rec && (
            <button className="command-btn" onClick={() => void pinCapture()}>
              Pin
            </button>
          )}
        </nav>
        <div className="topbar-spacer" />
        <div className="top-actions top-actions-secondary">
          <button className="command-btn" onClick={() => setShowHistory((s) => !s)}>
            History {history.length > 0 ? `(${history.length})` : ""}
          </button>
          {isTauriRuntime() && (
            <button className="command-btn" onClick={() => { setSettingsError(null); setSettingsOpen((open) => !open); }}>
              Settings
            </button>
          )}
          {rec && <button className="command-btn quiet" onClick={closeCapture}>Close</button>}
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
            <Editor
              key={rec.id}
              doc={rec}
              imageUrl={imageUrl}
              onChange={onAnnotationsChange}
              controllerRef={controllerRef}
              onHistoryState={onHistoryState}
            />
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
      </div>

      {showHistory && (
        <div className="history-flyout">
          <HistoryRail
            records={history}
            currentId={rec?.id ?? null}
            onOpen={(capture) => { openCapture(capture); setShowHistory(false); }}
            onRename={renameCapture}
            onDelete={deleteCapture}
          />
        </div>
      )}

      {notice && <div className="topbar-notice" role="status">{notice}</div>}

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
          includeCursor={settings.includeCursor}
          launchAtStartup={settings.launchAtStartup}
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

function CommandIcon({ name }: { name: "undo" | "redo" }) {
  return (
    <svg className="command-icon-svg" viewBox="0 0 24 24" aria-hidden="true">
      {name === "undo" ? (
        <path d="M9 7H5l3-3M5 7c5.7-3.1 12.5-.2 12.5 5.9 0 3.4-2.5 6.1-5.9 6.1-2.8 0-5.1-1.4-6.1-3.6" />
      ) : (
        <path d="M15 7h4l-3-3M19 7c-5.7-3.1-12.5-.2-12.5 5.9 0 3.4 2.5 6.1 5.9 6.1 2.8 0 5.1-1.4 6.1-3.6" />
      )}
    </svg>
  );
}

function App() {
  const params = new URLSearchParams(window.location.search);
  const overlay = params.get("overlay");
  if (overlay === "capture") return <CaptureOverlay />;
  if (overlay === "pin") {
    const id = params.get("id");
    return id ? <PinnedCapture id={id} /> : <div className="pin-state pin-error">Pinned capture id is missing.</div>;
  }
  return <EditorApp />;
}

export default App;
