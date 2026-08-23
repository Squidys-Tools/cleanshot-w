use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[derive(Default)]
pub struct PinState {
    captures: Mutex<HashMap<String, PinnedCapture>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedCapture {
    pub id: String,
    pub title: String,
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
}

static NEXT_PIN_ID: AtomicU64 = AtomicU64::new(1);

fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("The pinned capture id is invalid.".to_string());
    }
    Ok(())
}

fn window_label(id: &str) -> String {
    format!("pin-{id}")
}

fn window_size(width: u32, height: u32) -> (f64, f64) {
    let width = f64::from(width);
    let height = f64::from(height);
    let fit_scale = (720.0 / width).min(540.0 / height).min(1.0);
    let min_scale = (220.0 / width).max(120.0 / height);
    // Keep the image's aspect ratio. Enlarge small captures only when the
    // minimum size still fits inside the normal maximum window bounds.
    let scale = if min_scale <= (720.0 / width).min(540.0 / height) {
        fit_scale.max(min_scale)
    } else {
        fit_scale
    };
    (width * scale, height * scale)
}

#[tauri::command]
pub fn show_pinned_capture(
    app: AppHandle,
    state: State<'_, PinState>,
    png_base64: String,
    width: u32,
    height: u32,
    title: String,
) -> Result<String, String> {
    if width == 0 || height == 0 || width > 100_000 || height > 100_000 {
        return Err("The pinned capture dimensions are invalid.".to_string());
    }
    let bytes = BASE64
        .decode(&png_base64)
        .map_err(|error| format!("Could not decode the pinned capture: {error}"))?;
    if bytes.len() < 8 || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err("The pinned capture is not a PNG image.".to_string());
    }
    if title.len() > 200 {
        return Err("The pinned capture title is too long.".to_string());
    }

    let id = format!(
        "{}-{}",
        crate::library::now_millis(),
        NEXT_PIN_ID.fetch_add(1, Ordering::Relaxed)
    );
    let label = window_label(&id);
    let capture = PinnedCapture {
        id: id.clone(),
        title: if title.trim().is_empty() {
            "Pinned capture".to_string()
        } else {
            title
        },
        png_base64,
        width,
        height,
    };
    state
        .captures
        .lock()
        .map_err(|_| "The pin state is unavailable.".to_string())?
        .insert(id.clone(), capture.clone());

    let (window_width, window_height) = window_size(width, height);
    let result = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::App(format!("index.html?overlay=pin&id={id}").into()),
    )
    .title(&capture.title)
    .always_on_top(true)
    .resizable(true)
    .inner_size(window_width, window_height)
    .min_inner_size(180.0, 120.0)
    .build();

    if let Err(error) = result {
        let _ = state
            .captures
            .lock()
            .map(|mut captures| captures.remove(&id));
        return Err(format!("Could not open the pinned capture: {error}"));
    }

    Ok(id)
}

#[tauri::command]
pub fn get_pinned_capture(
    state: State<'_, PinState>,
    id: String,
) -> Result<Option<PinnedCapture>, String> {
    validate_id(&id)?;
    let captures = state
        .captures
        .lock()
        .map_err(|_| "The pin state is unavailable.".to_string())?;
    Ok(captures.get(&id).cloned())
}

#[tauri::command]
pub fn close_pinned_capture(
    app: AppHandle,
    state: State<'_, PinState>,
    id: String,
) -> Result<(), String> {
    validate_id(&id)?;
    remove_pinned_capture(&state, &id)?;
    if let Some(window) = app.get_webview_window(&window_label(&id)) {
        window
            .close()
            .map_err(|error| format!("Could not close the pinned capture: {error}"))?;
    }
    Ok(())
}

pub fn remove_pinned_capture(state: &State<'_, PinState>, id: &str) -> Result<(), String> {
    state
        .captures
        .lock()
        .map_err(|_| "The pin state is unavailable.".to_string())?
        .remove(id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_id, window_size};

    #[test]
    fn pin_window_size_preserves_aspect_ratio_and_caps_large_images() {
        let (width, height) = window_size(1920, 1080);
        assert!(width <= 720.0);
        assert!(height <= 540.0);
        assert!((width / height - 1920.0 / 1080.0).abs() < 0.01);
    }

    #[test]
    fn small_pin_windows_are_enlarged_without_distorting_the_image() {
        let (width, height) = window_size(100, 50);
        assert!(width >= 220.0);
        assert!(height >= 120.0);
        assert!((width / height - 2.0).abs() < 0.01);
    }

    #[test]
    fn pin_ids_reject_path_like_values() {
        assert!(validate_id("1700000000000-1").is_ok());
        assert!(validate_id("../capture").is_err());
        assert!(validate_id("pin capture").is_err());
    }
}
