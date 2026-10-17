//! Native screen capture, ported from `src-tauri/src/capture.rs`.
//!
//! All coordinates are physical Win32 pixels. The app sets per-monitor-v2 DPI
//! awareness at startup (`configure_dpi_awareness`), so `GetSystemMetrics` and
//! `BitBlt` speak true physical pixels; the GPUI overlay converts its logical
//! selection via `platform::selection_to_physical` before calling `crop`.

#[cfg(windows)]
pub fn configure_dpi_awareness() {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetProcessDpiAwarenessContext(context: isize) -> i32;
        fn SetProcessDPIAware() -> i32;
    }

    // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 is represented by -4.
    // GPUI does not set process DPI awareness, and GetDpiForWindow (which
    // feeds gpui's scale_factor) only reports true per-monitor DPI when the
    // process is aware — so this call is what makes the GPUI-logical ↔
    // Win32-physical conversion below exact.
    let configured = unsafe { SetProcessDpiAwarenessContext(-4) } != 0;
    if !configured {
        unsafe {
            let _ = SetProcessDPIAware();
        }
    }
}

#[cfg(not(windows))]
pub fn configure_dpi_awareness() {}

/// A full-virtual-screen (or window) capture in RGBA, with its physical
/// origin in global Win32 coordinates.
pub struct CaptureSurface {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub origin_x: i32,
    pub origin_y: i32,
}

impl CaptureSurface {
    /// Crop a physical rect (global coordinates) out of the surface,
    /// returning the cropped RGBA and the cropped surface's global origin.
    pub fn crop(&self, x: i32, y: i32, width: u32, height: u32) -> Result<CaptureSurface, String> {
        let x = x
            .checked_sub(self.origin_x)
            .ok_or_else(|| "The selected area is outside the capture.".to_string())?;
        let y = y
            .checked_sub(self.origin_y)
            .ok_or_else(|| "The selected area is outside the capture.".to_string())?;
        validate_selection(x, y, width, height, self.width, self.height)?;

        let row_bytes = usize::try_from(width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or_else(|| "The selected area is too large.".to_string())?;
        let output_len = row_bytes
            .checked_mul(usize::try_from(height).map_err(|_| "The selected area is too large.")?)
            .ok_or_else(|| "The selected area is too large.".to_string())?;
        let source_width = usize::try_from(self.width).map_err(|_| "The capture is too large.")?;
        let source_row_bytes = source_width
            .checked_mul(4)
            .ok_or_else(|| "The capture is too large.".to_string())?;
        let source_x = usize::try_from(x)
            .map_err(|_| "The selection is outside the capture.")?
            .checked_mul(4)
            .ok_or_else(|| "The selection is outside the capture.".to_string())?;
        let source_y = usize::try_from(y).map_err(|_| "The selection is outside the capture.")?;
        let mut rgba = vec![0; output_len];

        for row in 0..height as usize {
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
            width,
            height,
            origin_x: self
                .origin_x
                .checked_add(x)
                .ok_or_else(|| "The selection origin is outside the screen.".to_string())?,
            origin_y: self
                .origin_y
                .checked_add(y)
                .ok_or_else(|| "The selection origin is outside the screen.".to_string())?,
        })
    }
}

pub fn validate_selection(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    surface_width: u32,
    surface_height: u32,
) -> Result<(), String> {
    if x < 0 || y < 0 || width == 0 || height == 0 {
        return Err("Select an area larger than zero pixels.".to_string());
    }

    let right = u32::try_from(x).ok().and_then(|x| x.checked_add(width));
    let bottom = u32::try_from(y).ok().and_then(|y| y.checked_add(height));

    if right.is_none_or(|right| right > surface_width)
        || bottom.is_none_or(|bottom| bottom > surface_height)
    {
        return Err("The selection is outside the captured screen.".to_string());
    }

    Ok(())
}

/// Encode RGBA pixels as a PNG.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
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
    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    image::ImageEncoder::write_image(
        encoder,
        rgba,
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|error| format!("Could not encode the capture as PNG: {error}"))?;
    Ok(bytes)
}

#[cfg(windows)]
pub fn capture_surface(include_cursor: bool) -> Result<CaptureSurface, String> {
    windows_capture::capture_surface(include_cursor)
}

#[cfg(windows)]
mod windows_capture {
    use super::CaptureSurface;
    use std::ptr::null_mut;
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, HBITMAP, HDC,
        HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CURSORINFO, DI_NORMAL, DrawIconEx, GetCursorInfo, GetIconInfo, GetSystemMetrics, ICONINFO,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    pub(super) fn capture_surface(include_cursor: bool) -> Result<CaptureSurface, String> {
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
        if include_cursor {
            draw_cursor(memory_dc, origin_x, origin_y, width, height);
        }

        unsafe {
            SelectObject(memory_dc, previous);
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
        })
    }

    fn draw_cursor(memory_dc: HDC, origin_x: i32, origin_y: i32, width: u32, height: u32) {
        let mut cursor = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            flags: 0,
            hCursor: std::ptr::null_mut(),
            ptScreenPos: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
        };
        if unsafe { GetCursorInfo(&mut cursor) } == 0
            || cursor.flags == 0
            || cursor.hCursor.is_null()
        {
            return;
        }
        let mut icon_info: ICONINFO = unsafe { std::mem::zeroed() };
        let (hotspot_x, hotspot_y) = if unsafe { GetIconInfo(cursor.hCursor, &mut icon_info) } != 0
        {
            let hotspot = (icon_info.xHotspot as i32, icon_info.yHotspot as i32);
            unsafe {
                if !icon_info.hbmMask.is_null() {
                    let _ = DeleteObject(icon_info.hbmMask as HGDIOBJ);
                }
                if !icon_info.hbmColor.is_null() {
                    let _ = DeleteObject(icon_info.hbmColor as HGDIOBJ);
                }
            }
            hotspot
        } else {
            (0, 0)
        };
        let x = cursor.ptScreenPos.x - origin_x - hotspot_x;
        let y = cursor.ptScreenPos.y - origin_y - hotspot_y;
        if x >= width as i32 || y >= height as i32 || x + 32 <= 0 || y + 32 <= 0 {
            return;
        }
        unsafe {
            let _ = DrawIconEx(
                memory_dc,
                x,
                y,
                cursor.hCursor,
                0,
                0,
                0,
                std::ptr::null_mut(),
                DI_NORMAL,
            );
        }
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
        // GetDIBits hands back BGRA (top-down thanks to the negative height);
        // store RGBA and let the renderer's upload swizzle back to BGRA.
        for pixel in bgra.as_chunks_mut::<4>().0 {
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

#[cfg(not(windows))]
pub fn capture_surface(_include_cursor: bool) -> Result<CaptureSurface, String> {
    // The Linux dev loop has no GDI; hand back a synthetic pattern so the
    // whole pipeline (crop → library → pin → copy/save) stays runnable and
    // honest about what it is.
    dev_loop::synthetic_surface()
}

#[cfg(not(windows))]
mod dev_loop {
    use super::CaptureSurface;

    pub(super) fn synthetic_surface() -> Result<CaptureSurface, String> {
        let width: u32 = 800;
        let height: u32 = 500;
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let warm = (x * 255 / width) as u8;
                let cool = (y * 255 / height) as u8;
                rgba.extend_from_slice(&[warm, 80, cool, 255]);
            }
        }
        eprintln!("Snipsy: synthetic capture (Linux dev loop, no GDI)");
        Ok(CaptureSurface {
            rgba,
            width,
            height,
            origin_x: 0,
            origin_y: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureSurface, encode_png, validate_selection};

    fn sample_surface() -> CaptureSurface {
        CaptureSurface {
            rgba: (0..24).collect(),
            width: 3,
            height: 2,
            origin_x: -120,
            origin_y: 48,
        }
    }

    #[test]
    fn selection_accepts_edges_and_rejects_out_of_bounds_rectangles() {
        assert!(validate_selection(2, 1, 1, 1, 3, 2).is_ok());
        assert!(validate_selection(3, 0, 1, 1, 3, 2).is_err());
        assert!(validate_selection(i32::MAX, 0, 2, 1, 3, 2).is_err());
        assert!(validate_selection(-1, 0, 1, 1, 3, 2).is_err());
        assert!(validate_selection(0, 0, 0, 1, 3, 2).is_err());
    }

    #[test]
    fn crop_preserves_pixels_and_origin() {
        let cropped = sample_surface().crop(-119, 48, 2, 2).unwrap();
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        assert_eq!(cropped.origin_x, -119);
        assert_eq!(cropped.origin_y, 48);
        assert_eq!(
            cropped.rgba,
            vec![4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23]
        );
    }

    #[test]
    fn png_encoding_returns_a_rgba_png() {
        let bytes = encode_png(1, 1, &[10, 20, 30, 40]).unwrap();
        assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(encode_png(1, 1, &[10, 20, 30]).is_err());
    }
}
