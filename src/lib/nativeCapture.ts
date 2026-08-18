import { invoke } from "@tauri-apps/api/core";

export type NativeCaptureFrame = {
  pngBase64: string;
  width: number;
  height: number;
  originX: number;
  originY: number;
  dpiScale: number;
};

export type SelectionRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type NativeWindowInfo = {
  id: string;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
};

export type CaptureViewport = {
  left: number;
  top: number;
  width: number;
  height: number;
};

export function capturePointFromClient(
  clientX: number,
  clientY: number,
  viewport: CaptureViewport,
  frame: Pick<NativeCaptureFrame, "width" | "height">,
): { x: number; y: number } {
  const x = viewport.width > 0 ? ((clientX - viewport.left) / viewport.width) * frame.width : 0;
  const y = viewport.height > 0 ? ((clientY - viewport.top) / viewport.height) * frame.height : 0;
  return {
    x: Math.min(Math.max(x, 0), frame.width),
    y: Math.min(Math.max(y, 0), frame.height),
  };
}

export type NativeSettings = {
  captureHotkey: string;
  includeCursor: boolean;
  launchAtStartup: boolean;
};

export type PinnedCapture = {
  id: string;
  title: string;
  pngBase64: string;
  width: number;
  height: number;
};

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function startAreaCapture(): Promise<void> {
  return invoke<void>("start_area_capture");
}

export function getActiveCapture(): Promise<NativeCaptureFrame> {
  return invoke<NativeCaptureFrame>("get_active_capture");
}

export function completeAreaCapture(selection: SelectionRect): Promise<void> {
  return invoke<void>("complete_area_capture", { selection });
}

export function cancelAreaCapture(): Promise<void> {
  return invoke<void>("cancel_area_capture");
}

export function listCaptureWindows(): Promise<NativeWindowInfo[]> {
  return invoke<NativeWindowInfo[]>("list_capture_windows");
}

export function captureWindow(windowId: string): Promise<NativeCaptureFrame> {
  return invoke<NativeCaptureFrame>("capture_window", { windowId });
}

export function captureFullscreen(): Promise<NativeCaptureFrame> {
  return invoke<NativeCaptureFrame>("capture_screen");
}

export function getSettings(): Promise<NativeSettings> {
  return invoke<NativeSettings>("get_settings");
}

export function ensureCaptureHotkey(): Promise<void> {
  return invoke<void>("ensure_capture_hotkey");
}

export function setCaptureSettings(
  captureHotkey: string,
  includeCursor: boolean,
  launchAtStartup: boolean,
): Promise<NativeSettings> {
  return invoke<NativeSettings>("set_capture_settings", {
    captureHotkey,
    includeCursor,
    launchAtStartup,
  });
}

export function showPinnedCapture(
  pngBase64: string,
  width: number,
  height: number,
  title: string,
): Promise<string> {
  return invoke<string>("show_pinned_capture", { pngBase64, width, height, title });
}

export function getPinnedCapture(id: string): Promise<PinnedCapture | null> {
  return invoke<PinnedCapture | null>("get_pinned_capture", { id });
}

export function closePinnedCapture(id: string): Promise<void> {
  return invoke<void>("close_pinned_capture", { id });
}

export function captureFrameUrl(frame: NativeCaptureFrame): string {
  return `data:image/png;base64,${frame.pngBase64}`;
}

export async function blobToBase64(blob: Blob): Promise<string> {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return btoa(binary);
}

export async function captureFrameToBlob(frame: NativeCaptureFrame): Promise<Blob> {
  const response = await fetch(captureFrameUrl(frame));
  if (!response.ok) {
    throw new Error(`Could not decode the native capture (${response.status}).`);
  }
  return response.blob();
}
