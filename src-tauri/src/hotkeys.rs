use crate::library::{app_data_root, now_millis};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub const DEFAULT_CAPTURE_HOTKEY: &str = "ctrl+shift+4";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub capture_hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            capture_hotkey: DEFAULT_CAPTURE_HOTKEY.to_string(),
        }
    }
}

fn settings_path() -> Result<std::path::PathBuf, String> {
    Ok(app_data_root()?.join("settings.json"))
}

fn read_settings() -> Result<AppSettings, String> {
    let path = settings_path()?;
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppSettings::default())
        }
        Err(error) => return Err(format!("Could not read settings: {error}")),
    };
    let mut settings: AppSettings = serde_json::from_slice(&bytes).unwrap_or_default();
    settings.capture_hotkey = validate_shortcut(&settings.capture_hotkey)
        .unwrap_or_else(|_| DEFAULT_CAPTURE_HOTKEY.to_string());
    Ok(settings)
}

fn write_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| "The settings path has no parent directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create settings folder: {error}"))?;
    let mut bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("Could not encode settings: {error}"))?;
    bytes.push(b'\n');
    write_atomic(&path, &bytes)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension(format!("{}.tmp", now_millis()));
    fs::write(&temp, bytes).map_err(|error| format!("Could not write settings: {error}"))?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| format!("Could not replace settings: {error}"))?;
    }
    fs::rename(temp, path).map_err(|error| format!("Could not finish writing settings: {error}"))
}

fn validate_shortcut(shortcut: &str) -> Result<String, String> {
    let value = shortcut.trim();
    if value.is_empty() || value.len() > 80 || value.chars().any(char::is_control) {
        return Err("Enter a valid shortcut such as Ctrl+Shift+4.".to_string());
    }
    Ok(value.to_ascii_lowercase())
}

pub fn register_saved_hotkey(app: &AppHandle) -> Result<(), String> {
    let settings = read_settings()?;
    register_hotkey(app, &settings.capture_hotkey)
}

fn register_hotkey(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    if app.global_shortcut().is_registered(shortcut) {
        return Ok(());
    }
    app.global_shortcut().register(shortcut).map_err(|error| {
        format!("Could not register {shortcut}: {error}. Choose another shortcut.")
    })
}

#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    read_settings()
}

#[tauri::command]
pub fn ensure_capture_hotkey(app: AppHandle) -> Result<(), String> {
    let settings = read_settings()?;
    register_hotkey(&app, &settings.capture_hotkey)
}

#[tauri::command]
pub fn set_capture_hotkey(app: AppHandle, shortcut: String) -> Result<AppSettings, String> {
    let next_shortcut = validate_shortcut(&shortcut)?;
    let previous = read_settings()?;
    if previous.capture_hotkey == next_shortcut {
        register_hotkey(&app, &next_shortcut)?;
        return Ok(previous);
    }

    let _ = app
        .global_shortcut()
        .unregister(previous.capture_hotkey.as_str());
    if let Err(error) = register_hotkey(&app, &next_shortcut) {
        let _ = register_hotkey(&app, &previous.capture_hotkey);
        return Err(error);
    }

    let next = AppSettings {
        capture_hotkey: next_shortcut,
    };
    if let Err(error) = write_settings(&next) {
        let _ = app
            .global_shortcut()
            .unregister(next.capture_hotkey.as_str());
        let _ = register_hotkey(&app, &previous.capture_hotkey);
        return Err(error);
    }
    Ok(next)
}
