use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use std::borrow::Cow;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

fn decode_base64(value: &str) -> Result<Vec<u8>, String> {
    BASE64
        .decode(value)
        .map_err(|error| format!("Could not decode clipboard data: {error}"))
}

fn decode_png(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("Could not read PNG data: {error}"))?;
    if reader.info().bit_depth != png::BitDepth::Eight {
        return Err("Only 8-bit PNG images can be copied to the clipboard.".to_string());
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| format!("Could not decode PNG data: {error}"))?;
    let source = &buffer[..info.buffer_size()];
    let width = usize::try_from(info.width).map_err(|_| "The image is too wide.")?;
    let height = usize::try_from(info.height).map_err(|_| "The image is too tall.")?;
    let pixel_count = width
        .checked_mul(height)
        .ok_or_else(|| "The image is too large.".to_string())?;
    let rgba_len = pixel_count
        .checked_mul(4)
        .ok_or_else(|| "The image is too large.".to_string())?;
    let mut rgba = Vec::with_capacity(rgba_len);

    match info.color_type {
        png::ColorType::Rgba => rgba.extend_from_slice(source),
        png::ColorType::Rgb => {
            for pixel in source.chunks_exact(3) {
                rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for value in source {
                rgba.extend_from_slice(&[*value, *value, *value, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for pixel in source.chunks_exact(2) {
                rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
        }
        png::ColorType::Indexed => {
            return Err(
                "Indexed PNG images are not supported by the clipboard bridge.".to_string(),
            );
        }
    }

    let expected = pixel_count
        .checked_mul(4)
        .ok_or_else(|| "The image is too large.".to_string())?;
    if rgba.len() != expected {
        return Err("The decoded image has an invalid pixel buffer.".to_string());
    }
    Ok((width, height, rgba))
}

fn encode_png(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "The clipboard image is too large.".to_string())?;
    if rgba.len() != expected {
        return Err("The clipboard image has an invalid pixel buffer.".to_string());
    }
    let width = u32::try_from(width).map_err(|_| "The clipboard image is too wide.")?;
    let height = u32::try_from(height).map_err(|_| "The clipboard image is too tall.")?;
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("Could not start clipboard PNG encoding: {error}"))?;
    writer
        .write_image_data(rgba)
        .map_err(|error| format!("Could not encode clipboard PNG: {error}"))?;
    writer
        .finish()
        .map_err(|error| format!("Could not finish clipboard PNG: {error}"))?;
    Ok(bytes)
}

#[tauri::command]
pub fn copy_image_to_clipboard(png_base64: String) -> Result<(), String> {
    let bytes = decode_base64(&png_base64)?;
    let (width, height, rgba) = decode_png(&bytes)?;
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
    clipboard
        .set_image(ImageData {
            width,
            height,
            bytes: Cow::Owned(rgba),
        })
        .map_err(|error| format!("Could not copy image to clipboard: {error}"))
}

#[tauri::command]
pub fn read_image_from_clipboard() -> Result<Option<String>, String> {
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
    let image = match clipboard.get_image() {
        Ok(image) => image,
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(error) => return Err(format!("Could not read image from clipboard: {error}")),
    };
    let png = encode_png(image.width, image.height, image.bytes.as_ref())?;
    Ok(Some(BASE64.encode(png)))
}

#[tauri::command]
pub fn copy_file_to_clipboard(png_base64: String) -> Result<(), String> {
    let bytes = decode_base64(&png_base64)?;
    decode_png(&bytes)?;
    let directory = std::env::temp_dir().join("CleanShotW");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Could not create the clipboard temp folder: {error}"))?;
    let path = clipboard_temp_path(&directory);
    fs::write(&path, bytes).map_err(|error| format!("Could not write clipboard file: {error}"))?;

    let mut clipboard =
        Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
    let paths = [path.as_path()];
    clipboard
        .set()
        .file_list(&paths)
        .map_err(|error| format!("Could not copy file to clipboard: {error}"))
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
    clipboard
        .set_text(text)
        .map_err(|error| format!("Could not copy text to clipboard: {error}"))
}

fn clipboard_temp_path(directory: &Path) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    directory.join(format!("capture-{}-{timestamp}.png", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::{decode_base64, decode_png, encode_png};

    #[test]
    fn rgba_png_round_trip_preserves_transparent_pixels() {
        let pixels = [10, 20, 30, 255, 200, 150, 100, 42];
        let png = encode_png(2, 1, &pixels).unwrap();
        let (width, height, decoded) = decode_png(&png).unwrap();
        assert_eq!((width, height), (2, 1));
        assert_eq!(decoded, pixels);
    }

    #[test]
    fn clipboard_payload_validation_rejects_bad_or_incomplete_data() {
        assert!(decode_base64("not base64").is_err());
        assert!(encode_png(1, 1, &[0, 0, 0]).is_err());
        assert!(decode_png(b"not a png").is_err());
    }
}
