//! Copy a capture to the clipboard as an image (arboard), ported from
//! `src-tauri/src/clipboard.rs`. arboard expects RGBA and handles the
//! platform formats (CF_DIB on Windows) itself.

use arboard::{Clipboard, ImageData};
use std::borrow::Cow;

pub fn copy_png_image(png: &[u8]) -> Result<(), String> {
    let decoded = image::ImageReader::new(std::io::Cursor::new(png))
        .with_guessed_format()
        .map_err(|error| format!("Could not open the image for the clipboard: {error}"))?
        .decode()
        .map_err(|error| format!("Could not decode the image for the clipboard: {error}"))?
        .into_rgba8();
    let (width, height) = decoded.dimensions();
    let mut clipboard =
        Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
    clipboard
        .set_image(ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(decoded.into_raw()),
        })
        .map_err(|error| format!("Could not copy image to clipboard: {error}"))
}

/// Read an image from the clipboard as PNG bytes (for the editor's paste-image
/// command). Returns `Ok(None)` when the clipboard holds no image.
pub fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    let decoded = {
        let mut clipboard =
            Clipboard::new().map_err(|error| format!("Could not open clipboard: {error}"))?;
        let data = match clipboard.get_image() {
            Ok(data) => data,
            Err(_) => return Ok(None),
        };
        image::RgbaImage::from_raw(
            data.width as u32,
            data.height as u32,
            data.bytes.into_owned(),
        )
        .ok_or("The clipboard image buffer has an invalid size.".to_string())?
    };
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(decoded)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|error| format!("Could not encode the pasted image: {error}"))?;
    Ok(Some(png))
}
