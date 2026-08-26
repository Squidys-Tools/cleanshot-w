import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CaptureRecord, RegionMode, TldrawState } from "./types";
import { uid } from "./types";
import { host, setHost } from "./lib/bridge";
import { nativeHost, readNativeClipboardImage } from "./lib/nativeHost";
import { cropImage, loadImage, makeThumb } from "./lib/storage";
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
import type { OcrStatus } from "./components/QuickAccess";
import SettingsPopover from "./components/SettingsPopover";
import WindowPicker from "./components/WindowPicker";
import PinnedCapture from "./components/PinnedCapture";
import { getIconComponent, useIconLib } from "./components/editor/IconLibrary";
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
  const [historyState, setHistoryState] = useState({ canUndo: false, canRedo: false });
  const [ocr, setOcr] = useState<OcrState>({ status: "idle" });
  const [copied, setCopied] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<"saved" | "saving" | "error">("saved");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [regionMode, setRegionMode] = useState<RegionMode | null>(null);
  const [moreOpen, setMoreOpen] = useState(false);
  const [shortcutsOpen, setShortcutsOpen] = useState(false);
  const [exportOpen, setExportOpen] = useState(false);
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
  const moreRef = useRef<HTMLDivElement>(null);
  const retrySaveRef = useRef<(() => Promise<void>) | null>(null);

  useEffect(() => {
    if (isTauriRuntime()) setHost(nativeHost);
  }, []);

  useEffect(() => {
    recRef.current = rec;
  }, [rec]);

  const openCapture = useCallback((r: CaptureRecord) => {
    if (urlRef.current) URL.revokeObjectURL(urlRef.current);
    const url = URL.createObjectURL(r.imageBlob);
    urlRef.current = url;
    setImageUrl(url);
    setRec(r);
    setOcr({ status: "idle" });
    setCopied(false);
    setNotice(null);
    setSaveState("saved");
    setSaveError(null);
    setRegionMode(null);
  }, []);

  useEffect(() => {
    if (!moreOpen) return;
    const onPointerDown = (event: PointerEvent) => {
      if (moreRef.current && !moreRef.current.contains(event.target as Node)) setMoreOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [moreOpen]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "?" || event.ctrlKey || event.metaKey || event.altKey) return;
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)) return;
      event.preventDefault();
      setShortcutsOpen((open) => !open);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
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
      } catch (e: unknown) {
        setNotice(`Could not load that image: ${nativeErrorMessage(e, "The image could not be loaded.")}`);
      } finally {
        setBusy(false);
      }
    },
    [openCapture],
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
      setSaveState("saving");
      setSaveError(null);
      retrySaveRef.current = () => persistAnnotations(recordId, annotations, version);
      await host.updateCaptureAnnotations(recordId, annotations);
      if (saveTimers.current.get(recordId)?.version === version) {
        saveTimers.current.delete(recordId);
        setSaveState("saved");
        retrySaveRef.current = null;
      }
    },
    [],
  );

  const onAnnotationsChange = useCallback(
    (recordId: string, annotations: TldrawState) => {
      setRec((r) => (r && r.id === recordId ? { ...r, annotations, updatedAt: Date.now() } : r));
      setSaveState("saving");
      setSaveError(null);
      retrySaveRef.current = null;
      const prev = saveTimers.current.get(recordId);
      if (prev) window.clearTimeout(prev.timer);
      const version = (prev?.version ?? 0) + 1;
      const timer = window.setTimeout(() => {
        void persistAnnotations(recordId, annotations, version).catch((error: unknown) => {
          const message = nativeErrorMessage(error, "Could not save the capture annotations.");
          setSaveState("error");
          setSaveError(message);
          setNotice(message);
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
    setExportOpen(false);
    setHistoryState({ canUndo: false, canRedo: false });
    setSaveState("saved");
    setSaveError(null);
    setRegionMode(null);
    setMoreOpen(false);
  }, []);

  const onHistoryState = useCallback((next: { canUndo: boolean; canRedo: boolean }) => {
    setHistoryState((current) => (current.canUndo === next.canUndo && current.canRedo === next.canRedo ? current : next));
  }, []);

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

  const startOcrRegion = useCallback(() => {
    if (!rec) return;
    setMoreOpen(false);
    setOcr({ status: "idle" });
    setRegionMode("ocr");
  }, [rec]);

  const runOcrRegion = useCallback(async (rect: { x: number; y: number; width: number; height: number }) => {
    if (!rec) return;
    setOcr({ status: "running", progress: 0 });
    try {
      const region = await cropImage(rec.imageBlob, rect);
      const res = await host.recognize(region, (p) => setOcr({ status: "running", progress: p.progress }));
      setOcr({ status: "done", text: res.text });
      setNotice(null);
    } catch (error: unknown) {
      const message = nativeErrorMessage(error, "OCR could not read that region.");
      setOcr({ status: "error", message });
      setNotice(message);
    }
  }, [rec]);

  const applyCroppedCapture = useCallback(async (
    recordId: string,
    imageBlob: Blob,
    image: { width: number; height: number },
    annotations: TldrawState,
  ) => {
    const current = recRef.current;
    if (!current || current.id !== recordId) return;
    setSaveState("saving");
    setSaveError(null);
    try {
      const next: CaptureRecord = {
        ...current,
        imageBlob,
        thumbBlob: await makeThumb(imageBlob),
        image,
        annotations,
        updatedAt: Date.now(),
      };
      retrySaveRef.current = () => applyCroppedCapture(recordId, imageBlob, image, annotations);
      await host.saveCapture(next);
      openCapture(next);
      setSaveState("saved");
      retrySaveRef.current = null;
    } catch (error: unknown) {
      const message = nativeErrorMessage(error, "Could not save the cropped capture.");
      setSaveState("error");
      setSaveError(message);
      setNotice(message);
    }
  }, [openCapture]);

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
    <div className={shortcutsOpen ? "app shortcuts-open" : "app"}>
      <header className="topbar" data-chrome>
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
          <div className="command-separator" />
          <button className="command-btn copy-command" onClick={copyImage} disabled={!rec}>
            {copied ? "Copied" : "Copy"}
          </button>
          <button className="command-btn action-strong save-command" onClick={savePng} disabled={!rec}>
            Save PNG
          </button>
          <div className="topbar-menu-wrap" ref={moreRef}>
            <button className="command-btn" onClick={() => setMoreOpen((open) => !open)} disabled={!rec} aria-expanded={moreOpen}>
              More
            </button>
            {moreOpen && (
              <div className="topbar-menu" role="menu">
                <div className="topbar-menu-label">Export and share</div>
                <button role="menuitem" onClick={() => { void copyFile(); setMoreOpen(false); }} disabled={!rec}>Copy as file</button>
                {isTauriRuntime() && (
                  <button role="menuitem" onClick={() => { void pinCapture(); setMoreOpen(false); }} disabled={!rec}>Pin above windows</button>
                )}
                <div className="topbar-menu-divider" />
                <div className="topbar-menu-label">Recognize text</div>
                <button role="menuitem" onClick={() => { void runOcr(); setMoreOpen(false); }} disabled={!rec || ocr.status === "running"}>
                  {ocr.status === "running" ? `OCR ${Math.round((ocr.progress ?? 0) * 100)}%` : "OCR entire image"}
                </button>
                <button role="menuitem" onClick={startOcrRegion} disabled={!rec || ocr.status === "running"}>OCR selected region</button>
              </div>
            )}
          </div>
        </nav>
        <div className="topbar-spacer" />
        <div className="top-actions top-actions-secondary">
          {isTauriRuntime() && (
            <button className="command-btn" onClick={() => { setSettingsError(null); setSettingsOpen((open) => !open); }}>
              Settings
            </button>
          )}
          <button className="command-btn quiet" onClick={() => setShortcutsOpen((open) => !open)} aria-expanded={shortcutsOpen}>
            Shortcuts
          </button>
          {rec && <button className="command-btn quiet" onClick={closeCapture}>Close</button>}
          {rec && (
            <button className="command-btn primary done-command" onClick={() => setExportOpen(true)}>
              Done
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
            exportOpen ? (
              <ExportPage
                rec={rec}
                imageUrl={imageUrl}
                copied={copied}
                onBack={() => setExportOpen(false)}
                onCopyImage={() => void copyImage()}
                onSavePng={() => void savePng()}
                onCopyFile={() => void copyFile()}
              />
            ) : (
              <Editor
                key={rec.id}
                doc={rec}
                imageUrl={imageUrl}
                onChange={onAnnotationsChange}
                controllerRef={controllerRef}
                onHistoryState={onHistoryState}
                saveState={saveState}
                saveError={saveError}
                onRetrySave={() => { void retrySaveRef.current?.(); }}
                regionMode={regionMode}
                onRegionModeChange={setRegionMode}
                onCrop={applyCroppedCapture}
                onOcrRegion={(rect) => void runOcrRegion(rect)}
              />
            )
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

      {notice && <div className="topbar-notice" role="status">{notice}</div>}

      <ShortcutPanel open={shortcutsOpen} onClose={() => setShortcutsOpen(false)} />

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

function ExportPage({
  rec,
  imageUrl,
  copied,
  onBack,
  onCopyImage,
  onSavePng,
  onCopyFile,
}: {
  rec: CaptureRecord;
  imageUrl: string;
  copied: boolean;
  onBack: () => void;
  onCopyImage: () => void;
  onSavePng: () => void;
  onCopyFile: () => void;
}) {
  return (
    <div className="export-page">
      <section className="export-card">
        <header className="export-head">
          <div>
            <h2>Export capture</h2>
            <span>Choose where this capture goes.</span>
          </div>
          <button className="command-btn quiet" onClick={onBack}>
            Back to editing
          </button>
        </header>
        <div className="export-preview">
          <img src={imageUrl} alt={rec.title} />
        </div>
        <div className="export-actions">
          <button className="export-action primary" onClick={onCopyImage}>
            <strong>{copied ? "Copied to clipboard" : "Copy to clipboard"}</strong>
            <small>Paste it straight into chats, docs, and issues.</small>
          </button>
          <button className="export-action" onClick={onSavePng}>
            <strong>Save as PNG</strong>
            <small>Write the flattened image to a file.</small>
          </button>
          <button className="export-action" onClick={onCopyFile}>
            <strong>Copy as file</strong>
            <small>A PNG file on the clipboard, ready to attach.</small>
          </button>
        </div>
      </section>
    </div>
  );
}

function ShortcutPanel({ open, onClose }: { open: boolean; onClose: () => void }) {
  const groups = [
    {
      label: "Tools",
      items: [
        ["V", "Select"], ["K", "Highlight"], ["D", "Draw"], ["A", "Arrow"], ["T", "Text"], ["B", "Blur"], ["X", "Redact"],
      ],
    },
    {
      label: "Editor",
      items: [["Ctrl + Z", "Undo"], ["Ctrl + Y", "Redo"], ["Ctrl + D", "Duplicate"], ["Delete", "Delete selection"], ["Esc", "Cancel or clear selection"], ["?", "Show shortcuts"]],
    },
  ];
  return (
    <section className={open ? "shortcut-panel open" : "shortcut-panel"} role="dialog" aria-labelledby="shortcut-panel-title" aria-hidden={!open}>
      <div className="shortcut-panel-head">
        <div>
          <strong id="shortcut-panel-title">Keyboard shortcuts</strong>
          <span>Keep your hands on the capture.</span>
        </div>
        <button className="close-btn" onClick={onClose} aria-label="Close keyboard shortcuts">×</button>
      </div>
      {groups.map((group) => (
        <div className="shortcut-group" key={group.label}>
          <div className="shortcut-group-label">{group.label}</div>
          {group.items.map(([key, label]) => (
            <div className="shortcut-row" key={key}>
              <kbd>{key}</kbd>
              <span>{label}</span>
            </div>
          ))}
        </div>
      ))}
    </section>
  );
}

function CommandIcon({ name }: { name: "undo" | "redo" }) {
  const [iconLib] = useIconLib();
  const LibIcon = iconLib !== "svg" ? getIconComponent(iconLib, name) : null;
  if (LibIcon) {
    return (
      <LibIcon
        size={18}
        strokeWidth={1.75}
        weight={iconLib === "phosphor" ? "bold" : undefined}
        className="lib-icon"
        aria-hidden
      />
    );
  }
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
