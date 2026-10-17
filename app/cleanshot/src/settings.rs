//! Application settings, ported from `src-tauri/src/hotkeys.rs`.
//!
//! The JSON schema and file location (`%LOCALAPPDATA%/Snipsy/settings.json`)
//! are shared with the existing Tauri app so both builds read the same file.
//! `library::app_data_root` migrates the old `CleanShotW` directory once when
//! no canonical `Snipsy` directory exists.

use crate::library::{app_data_root, write_atomic};
use serde::{Deserialize, Serialize};
use std::fs;

pub const DEFAULT_CAPTURE_HOTKEY: &str = "ctrl+shift+4";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub capture_hotkey: String,
    #[serde(default)]
    pub include_cursor: bool,
    #[serde(default)]
    pub launch_at_startup: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            capture_hotkey: DEFAULT_CAPTURE_HOTKEY.to_string(),
            include_cursor: false,
            launch_at_startup: false,
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
            return Ok(AppSettings::default());
        }
        Err(error) => return Err(format!("Could not read settings: {error}")),
    };
    let mut settings: AppSettings = serde_json::from_slice(&bytes).unwrap_or_default();
    settings.capture_hotkey = validate_shortcut(&settings.capture_hotkey)
        .unwrap_or_else(|_| DEFAULT_CAPTURE_HOTKEY.to_string());
    Ok(settings)
}

pub fn current_settings() -> AppSettings {
    read_settings().unwrap_or_default()
}

#[allow(dead_code)] // used when a preferences UI lands; schema kept in sync now
pub(crate) fn write_settings(settings: &AppSettings) -> Result<(), String> {
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

pub fn validate_shortcut(shortcut: &str) -> Result<String, String> {
    let value = shortcut.trim();
    if value.is_empty() || value.len() > 80 || value.chars().any(char::is_control) {
        return Err("Enter a valid shortcut such as Ctrl+Shift+4.".to_string());
    }
    let normalized = value.to_ascii_lowercase();
    if crate::shortcut::parse_shortcut(&normalized).is_none() {
        return Err("Enter a valid shortcut such as Ctrl+Shift+4.".to_string());
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, DEFAULT_CAPTURE_HOTKEY, validate_shortcut};

    #[test]
    fn settings_keep_new_defaults_when_loading_old_json() {
        let settings: AppSettings = serde_json::from_str(&format!(
            r#"{{"captureHotkey":"{DEFAULT_CAPTURE_HOTKEY}"}}"#
        ))
        .unwrap();
        assert!(!settings.include_cursor);
        assert!(!settings.launch_at_startup);
    }

    #[test]
    fn shortcut_validation_normalizes_case_and_whitespace() {
        assert_eq!(validate_shortcut(" Ctrl+Shift+4 ").unwrap(), "ctrl+shift+4");
        assert_eq!(validate_shortcut(" Alt+Space ").unwrap(), "alt+space");
        assert!(validate_shortcut("Ctrl+Shift+F10").is_ok());
        assert!(validate_shortcut("Ctrl+F25").is_err());
        assert!(validate_shortcut("\n").is_err());
    }
}
