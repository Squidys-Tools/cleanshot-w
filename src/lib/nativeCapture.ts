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

export type NativeSettings = {
  captureHotkey: string;
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

export function getSettings(): Promise<NativeSettings> {
  return invoke<NativeSettings>("get_settings");
}

export function ensureCaptureHotkey(): Promise<NativeSettings> {
  return invoke<NativeSettings>("ensure_capture_hotkey");
}

export function setCaptureHotkey(captureHotkey: string): Promise<NativeSettings> {
  return invoke<NativeSettings>("set_capture_hotkey", { captureHotkey });
}

export function captureFrameUrl(frame: NativeCaptureFrame): string {
  return `data:image/png;base64,${frame.pngBase64}`;
}

export async function captureFrameToBlob(frame: NativeCaptureFrame): Promise<Blob> {
  const response = await fetch(captureFrameUrl(frame));
  if (!response.ok) {
    throw new Error(`Could not decode the native capture (${response.status}).`);
  }
  return response.blob();
}
