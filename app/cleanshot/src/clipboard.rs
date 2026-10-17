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
