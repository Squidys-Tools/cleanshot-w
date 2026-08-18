use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFrame {
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub dpi_scale: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct CaptureState {
    active: Mutex<Option<CaptureSurface>>,
}

struct CaptureSurface {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    origin_x: i32,
    origin_y: i32,
    dpi_scale: f64,
}

impl CaptureSurface {
    fn frame(&self) -> Result<CaptureFrame, String> {
        Ok(CaptureFrame {
            png_base64: BASE64.encode(encode_png(self.width, self.height, &self.rgba)?),
            width: self.width,
            height: self.height,
            origin_x: self.origin_x,
            origin_y: self.origin_y,
            dpi_scale: self.dpi_scale,
        })
    }

    fn crop(&self, selection: &SelectionRect) -> Result<Self, String> {
        validate_selection(selection, self.width, self.height)?;

        let row_bytes = usize::try_from(selection.width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| "The selected area is too large.".to_string())?;
        let output_len = row_bytes
            .checked_mul(
                usize::try_from(selection.height).map_err(|_| "The selected area is too large.")?,
            )
            .ok_or_else(|| "The selected area is too large.".to_string())?;
        let source_width = usize::try_from(self.width).map_err(|_| "The capture is too large.")?;
        let source_row_bytes = source_width
            .checked_mul(4)
            .ok_or_else(|| "The capture is too large.".to_string())?;
        let source_x = usize::try_from(selection.x)
            .map_err(|_| "The selection is outside the capture.")?
            .checked_mul(4)
            .ok_or_else(|| "The selection is outside the capture.".to_string())?;
        let source_y =
            usize::try_from(selection.y).map_err(|_| "The selection is outside the capture.")?;
        let mut rgba = vec![0; output_len];

        for row in 0..selection.height as usize {
            let source_start = (source_y + row)
                .checked_mul(source_row_bytes)
                .and_then(|offset| offset.checked_add(source_x))
                .ok_or_else(|| "The selected area is outside the capture.".to_string())?;
            let output_start = row * row_bytes;
            let source_end = source_start
                .checked_add(row_bytes)
                .ok_or_else(|| "The selected area is outside the capture.".to_string())?;
            if source_end > self.rgba.len() {
                return Err("The selected area is outside the capture.".to_string());
            }
            rgba[output_start..output_start + row_bytes]
                .copy_from_slice(&self.rgba[source_start..source_end]);
        }

        Ok(Self {
            rgba,
            width: selection.width,
            height: selection.height,
            origin_x: self
                .origin_x
                .checked_add(selection.x)
                .ok_or_else(|| "The selection origin is outside the screen.".to_string())?,
            origin_y: self
                .origin_y
                .checked_add(selection.y)
                .ok_or_else(|| "The selection origin is outside the screen.".to_string())?,
            dpi_scale: self.dpi_scale,
        })
    }
}

fn validate_selection(selection: &SelectionRect, width: u32, height: u32) -> Result<(), String> {
    if selection.x < 0 || selection.y < 0 || selection.width == 0 || selection.height == 0 {
        return Err("Select an area larger than zero pixels.".to_string());
    }

    let right = u32::try_from(selection.x)
        .ok()
        .and_then(|x| x.checked_add(selection.width));
    let bottom = u32::try_from(selection.y)
        .ok()
        .and_then(|y| y.checked_add(selection.height));

    if right.is_none_or(|right| right > width) || bottom.is_none_or(|bottom| bottom > height) {
        return Err("The selection is outside the captured screen.".to_string());
    }

    Ok(())
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let expected_len = usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "The capture is too large to encode.".to_string())?;
    if rgba.len() != expected_len {
        return Err("The capture buffer has an invalid size.".to_string());
    }

    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("Could not start PNG encoding: {error}"))?;
    writer
        .write_image_data(rgba)
        .map_err(|error| format!("Could not encode the capture as PNG: {error}"))?;
    writer
        .finish()
        .map_err(|error| format!("Could not finish PNG encoding: {error}"))?;
    Ok(bytes)
}

#[tauri::command]
pub fn capture_screen() -> Result<CaptureFrame, String> {
    capture_surface()?.frame()
}

#[tauri::command]
pub fn list_capture_windows() -> Result<Vec<WindowInfo>, String> {
    platform_list_windows()
}

#[tauri::command]
pub fn capture_window(window_id: String) -> Result<CaptureFrame, String> {
    let window_id = window_id
        .parse::<u64>()
        .map_err(|_| "The selected window id is invalid.".to_string())?;
    platform_capture_window(window_id)?.frame()
}

#[tauri::command]
pub fn get_active_capture(state: State<'_, CaptureState>) -> Result<CaptureFrame, String> {
    let active = state
        .active
        .lock()
        .map_err(|_| "The capture state is unavailable.".to_string())?;
    active
        .as_ref()
        .ok_or_else(|| "There is no active capture.".to_string())?
        .frame()
}

#[tauri::command]
pub async fn start_area_capture(
    app: AppHandle,
    state: State<'_, CaptureState>,
) -> Result<(), String> {
    let surface = capture_surface()?;
    let width = surface.width;
    let height = surface.height;
    let origin_x = surface.origin_x;
    let origin_y = surface.origin_y;

    {
        let mut active = state
            .active
            .lock()
            .map_err(|_| "The capture state is unavailable.".to_string())?;
        *active = Some(surface);
    }

    if let Some(existing) = app.get_webview_window("capture-overlay") {
        existing
            .close()
            .map_err(|error| format!("Could not close the previous capture overlay: {error}"))?;
    }

    let overlay = WebviewWindowBuilder::new(
        &app,
        "capture-overlay",
        WebviewUrl::App("index.html?overlay=capture".into()),
    )
    .title("CleanShot W Capture")
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .resizable(false)
    .skip_taskbar(true)
    .visible(false)
    .position(0.0, 0.0)
    .inner_size(width as f64, height as f64)
    .build()
    .map_err(|error| format!("Could not open the capture overlay: {error}"))?;

    overlay
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
            origin_x, origin_y,
        )))
        .map_err(|error| format!("Could not position the capture overlay: {error}"))?;
    overlay
        .set_size(tauri::Size::Physical(tauri::PhysicalSize::new(
            width, height,
        )))
        .map_err(|error| format!("Could not size the capture overlay: {error}"))?;

    overlay
        .show()
        .map_err(|error| format!("Could not show the capture overlay: {error}"))?;
    overlay
        .set_focus()
        .map_err(|error| format!("Could not focus the capture overlay: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn complete_area_capture(
    app: AppHandle,
    state: State<'_, CaptureState>,
    selection: SelectionRect,
) -> Result<(), String> {
    let cropped = {
        let active = state
            .active
            .lock()
            .map_err(|_| "The capture state is unavailable.".to_string())?;
        active
            .as_ref()
            .ok_or_else(|| "There is no active capture.".to_string())?
            .crop(&selection)?
    };
    let frame = cropped.frame()?;

    if let Some(main) = app.get_webview_window("main") {
        main.emit("capture:completed", &frame)
            .map_err(|error| format!("Could not deliver the capture to the editor: {error}"))?;
    } else {
        return Err("The editor window is unavailable.".to_string());
    }

    state
        .active
        .lock()
        .map_err(|_| "The capture state is unavailable.".to_string())?
        .take();

    if let Some(overlay) = app.get_webview_window("capture-overlay") {
        overlay
            .close()
            .map_err(|error| format!("Could not close the capture overlay: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_area_capture(app: AppHandle, state: State<'_, CaptureState>) -> Result<(), String> {
    state
        .active
        .lock()
        .map_err(|_| "The capture state is unavailable.".to_string())?
        .take();
    if let Some(overlay) = app.get_webview_window("capture-overlay") {
        overlay
            .close()
            .map_err(|error| format!("Could not close the capture overlay: {error}"))?;
    }
    Ok(())
}

#[cfg(windows)]
fn capture_surface() -> Result<CaptureSurface, String> {
    windows_capture::capture_surface()
}

#[cfg(windows)]
fn platform_list_windows() -> Result<Vec<WindowInfo>, String> {
    windows_capture::list_windows()
}

#[cfg(windows)]
fn platform_capture_window(window_id: u64) -> Result<CaptureSurface, String> {
    windows_capture::capture_window(window_id)
}

#[cfg(not(windows))]
fn capture_surface() -> Result<CaptureSurface, String> {
    Err("Native screen capture is only available in the Windows app.".to_string())
}

#[cfg(not(windows))]
fn platform_list_windows() -> Result<Vec<WindowInfo>, String> {
    Err("Window capture is only available in the Windows app.".to_string())
}

#[cfg(not(windows))]
fn platform_capture_window(_window_id: u64) -> Result<CaptureSurface, String> {
    Err("Window capture is only available in the Windows app.".to_string())
}

#[cfg(windows)]
mod windows_capture {
    use super::{CaptureSurface, WindowInfo};
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT,
        DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, SRCCOPY,
    };
    use windows_sys::Win32::Storage::Xps::PrintWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetSystemMetrics, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        IsWindow, IsWindowVisible, PW_RENDERFULLCONTENT, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    pub(super) fn list_windows() -> Result<Vec<WindowInfo>, String> {
        let mut windows = Vec::new();
        let result = unsafe { EnumWindows(Some(enum_window), &mut windows as *mut _ as LPARAM) };
        if result == 0 {
            return Err(last_error("Windows could not enumerate windows"));
        }
        Ok(windows)
    }

    pub(super) fn capture_window(window_id: u64) -> Result<CaptureSurface, String> {
        let hwnd = window_id as usize as HWND;
        if hwnd.is_null() || unsafe { IsWindow(hwnd) } == 0 {
            return Err("The selected window no longer exists.".to_string());
        }
        let mut rect: RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return Err(last_error(
                "Windows could not read the selected window bounds",
            ));
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return Err("The selected window has no visible area.".to_string());
        }

        let width = u32::try_from(width).map_err(|_| "The selected window is too large.")?;
        let height = u32::try_from(height).map_err(|_| "The selected window is too large.")?;
        let width_i32 = i32::try_from(width).map_err(|_| "The selected window is too large.")?;
        let height_i32 = i32::try_from(height).map_err(|_| "The selected window is too large.")?;
        let screen_dc = unsafe { GetDC(null_mut()) };
        if screen_dc.is_null() {
            return Err(last_error("Could not access the Windows screen"));
        }
        let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
        if memory_dc.is_null() {
            unsafe { ReleaseDC(null_mut(), screen_dc) };
            return Err(last_error("Could not create a Windows capture buffer"));
        }
        let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width_i32, height_i32) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(null_mut(), screen_dc);
            }
            return Err(last_error("Could not allocate a Windows capture bitmap"));
        }
        let previous = unsafe { SelectObject(memory_dc, bitmap as _) };
        let printed = unsafe { PrintWindow(hwnd, memory_dc, PW_RENDERFULLCONTENT) };
        let copied = if printed == 0 {
            unsafe {
                BitBlt(
                    memory_dc,
                    0,
                    0,
                    width_i32,
                    height_i32,
                    screen_dc,
                    rect.left,
                    rect.top,
                    SRCCOPY | CAPTUREBLT,
                )
            }
        } else {
            1
        };
        if copied == 0 {
            cleanup(screen_dc, memory_dc, bitmap, previous);
            return Err(last_error("Windows could not capture the selected window"));
        }

        let rgba = read_bitmap(memory_dc, bitmap, width, height);
        cleanup(screen_dc, memory_dc, bitmap, previous);
        let rgba = rgba?;
        Ok(CaptureSurface {
            rgba,
            width,
            height,
            origin_x: rect.left,
            origin_y: rect.top,
            dpi_scale: 1.0,
        })
    }

    pub fn capture_surface() -> Result<CaptureSurface, String> {
        let origin_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let origin_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        if width <= 0 || height <= 0 {
            return Err("Windows returned an empty virtual screen.".to_string());
        }

        let width = u32::try_from(width).map_err(|_| "The virtual screen is too large.")?;
        let height = u32::try_from(height).map_err(|_| "The virtual screen is too large.")?;
        let width_i32 = i32::try_from(width).map_err(|_| "The virtual screen is too large.")?;
        let height_i32 = i32::try_from(height).map_err(|_| "The virtual screen is too large.")?;
        let screen_dc = unsafe { GetDC(null_mut()) };
        if screen_dc.is_null() {
            return Err(last_error("Could not access the Windows screen."));
        }

        let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
        if memory_dc.is_null() {
            unsafe { ReleaseDC(null_mut(), screen_dc) };
            return Err(last_error("Could not create a Windows capture buffer."));
        }

        let bitmap = unsafe { CreateCompatibleBitmap(screen_dc, width_i32, height_i32) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory_dc);
                ReleaseDC(null_mut(), screen_dc);
            }
            return Err(last_error("Could not allocate a Windows capture bitmap."));
        }

        let previous = unsafe { SelectObject(memory_dc, bitmap as _) };
        let copied = unsafe {
            BitBlt(
                memory_dc,
                0,
                0,
                width_i32,
                height_i32,
                screen_dc,
                origin_x,
                origin_y,
                SRCCOPY | CAPTUREBLT,
            )
        };
        if copied == 0 {
            cleanup(screen_dc, memory_dc, bitmap, previous);
            return Err(last_error("Windows could not copy the screen."));
        }

        let rgba = read_bitmap(memory_dc, bitmap, width, height);
        cleanup(screen_dc, memory_dc, bitmap, previous);
        let rgba = rgba?;

        Ok(CaptureSurface {
            rgba,
            width,
            height,
            origin_x,
            origin_y,
            dpi_scale: 1.0,
        })
    }

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let length = GetWindowTextLengthW(hwnd);
        if length <= 0 {
            return 1;
        }
        let mut title = vec![0u16; length as usize + 1];
        let written = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
        if written <= 0 {
            return 1;
        }
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 || rect.right <= rect.left || rect.bottom <= rect.top
        {
            return 1;
        }
        let windows = &mut *(lparam as *mut Vec<WindowInfo>);
        windows.push(WindowInfo {
            id: (hwnd as usize as u64).to_string(),
            title: String::from_utf16_lossy(&title[..written as usize]),
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        });
        1
    }

    fn read_bitmap(
        memory_dc: HDC,
        bitmap: HBITMAP,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, String> {
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let pixel_count = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or_else(|| "The capture is too large.".to_string())?;
        let buffer_len = pixel_count
            .checked_mul(4)
            .ok_or_else(|| "The capture is too large.".to_string())?;
        let mut bgra = vec![0u8; buffer_len];
        let copied_lines = unsafe {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height,
                bgra.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        if copied_lines == 0 {
            return Err(last_error("Windows could not read the capture bitmap"));
        }
        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        Ok(bgra)
    }

    fn cleanup(screen_dc: HDC, memory_dc: HDC, bitmap: HBITMAP, previous: HGDIOBJ) {
        unsafe {
            SelectObject(memory_dc, previous);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(memory_dc);
            ReleaseDC(null_mut(), screen_dc);
        }
    }

    fn last_error(context: &str) -> String {
        let error = std::io::Error::last_os_error();
        format!("{context}: {error}")
    }
}
