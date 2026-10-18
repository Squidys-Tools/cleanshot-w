//! Application settings, ported from `src-tauri/src/hotkeys.rs`.
//!
//! The JSON schema and file location (`%LOCALAPPDATA%/Snipsy/settings.json`)
//! are shared with the existing Tauri app so both builds read the same file.
//! `library::app_data_root` migrates the old `CleanShotW` directory once when
//! no canonical `Snipsy` directory exists.

use crate::library::{app_data_root, write_atomic};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedSettings<'a> {
    capture_hotkey: &'a str,
    include_cursor: bool,
    launch_at_startup: bool,
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

fn read_settings() -> Result<AppSettings, String> {
    read_settings_from_root(&app_data_root()?)
}

fn read_settings_from_root(root: &Path) -> Result<AppSettings, String> {
    let bytes = match fs::read(root.join("settings.json")) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("Could not read settings: {error}")),
    };
    let mut settings: AppSettings = bytes
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    settings.capture_hotkey = validate_shortcut(&settings.capture_hotkey)
        .unwrap_or_else(|_| DEFAULT_CAPTURE_HOTKEY.to_string());
    Ok(settings)
}

pub fn current_settings() -> AppSettings {
    read_settings().unwrap_or_default()
}

pub(crate) fn write_settings(settings: &AppSettings) -> Result<(), String> {
    write_settings_to_root(&app_data_root()?, settings)
}

fn write_settings_to_root(root: &Path, settings: &AppSettings) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("Could not create settings folder: {error}"))?;
    let shared = SharedSettings {
        capture_hotkey: &settings.capture_hotkey,
        include_cursor: settings.include_cursor,
        launch_at_startup: settings.launch_at_startup,
    };
    let mut bytes = serde_json::to_vec_pretty(&shared)
        .map_err(|error| format!("Could not encode settings: {error}"))?;
    bytes.push(b'\n');
    write_atomic(&root.join("settings.json"), &bytes)
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

pub fn sync_autostart() -> Result<(), String> {
    set_autostart(read_settings()?.launch_at_startup)
}

pub fn toggle_autostart() -> Result<bool, String> {
    let mut settings = read_settings()?;
    let enabled = !settings.launch_at_startup;
    set_autostart(enabled)?;
    settings.launch_at_startup = enabled;
    if let Err(error) = write_settings(&settings) {
        let _ = set_autostart(!enabled);
        return Err(error);
    }
    Ok(enabled)
}

#[cfg(windows)]
fn set_autostart(enabled: bool) -> Result<(), String> {
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    type Hkey = *mut c_void;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegCloseKey(key: Hkey) -> i32;
        fn RegCreateKeyExW(
            key: Hkey,
            sub_key: *const u16,
            reserved: u32,
            class: *const u16,
            options: u32,
            desired_access: u32,
            security_attributes: *const c_void,
            result: *mut Hkey,
            disposition: *mut u32,
        ) -> i32;
        fn RegDeleteValueW(key: Hkey, value_name: *const u16) -> i32;
        fn RegSetValueExW(
            key: Hkey,
            value_name: *const u16,
            reserved: u32,
            value_type: u32,
            data: *const u8,
            data_size: u32,
        ) -> i32;
    }

    const HKEY_CURRENT_USER: Hkey = 0x8000_0001usize as Hkey;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_SZ: u32 = 1;
    const ERROR_FILE_NOT_FOUND: i32 = 2;
    const ERROR_SUCCESS: i32 = 0;

    let value = if enabled {
        let executable = std::env::current_exe()
            .map_err(|error| format!("Could not locate the application executable: {error}"))?;
        let command = quote_autostart_command(&executable.to_string_lossy());
        let value: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = u32::try_from(value.len() * std::mem::size_of::<u16>())
            .map_err(|_| "The startup command is too long.")?;
        Some((value, byte_len))
    } else {
        None
    };
    let sub_key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "Snipsy\0".encode_utf16().collect();
    let legacy_value_name: Vec<u16> = "CleanShotW\0".encode_utf16().collect();
    let mut key: Hkey = null_mut();
    let mut disposition = 0;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            0,
            null(),
            0,
            KEY_SET_VALUE,
            null(),
            &mut key,
            &mut disposition,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "Could not open the Windows startup registry key ({status})."
        ));
    }

    let result = match value {
        Some((value, byte_len)) => unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                byte_len,
            )
        },
        None => {
            let status = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
            if status == ERROR_FILE_NOT_FOUND {
                ERROR_SUCCESS
            } else {
                status
            }
        }
    };
    let legacy_result = unsafe { RegDeleteValueW(key, legacy_value_name.as_ptr()) };
    unsafe { RegCloseKey(key) };

    let result = if result == ERROR_SUCCESS
        && (legacy_result == ERROR_SUCCESS || legacy_result == ERROR_FILE_NOT_FOUND)
    {
        ERROR_SUCCESS
    } else if result != ERROR_SUCCESS {
        result
    } else {
        legacy_result
    };

    if result == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!(
            "Could not update Windows startup registration ({result})."
        ))
    }
}

#[cfg(not(windows))]
fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        Err("Launch at startup is only available in the Windows app.".to_string())
    } else {
        Ok(())
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn quote_autostart_command(executable: &str) -> String {
    format!("\"{}\" --minimized", executable.replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, DEFAULT_CAPTURE_HOTKEY, quote_autostart_command, validate_shortcut};

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
    fn unknown_settings_fields_are_ignored() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"captureHotkey":"alt+5","includeCursor":true,"launchAtStartup":true,"defaultIconPack":"future-pack"}"#,
        )
        .unwrap();
        assert_eq!(settings.capture_hotkey, "alt+5");
        assert!(settings.include_cursor);
        assert!(settings.launch_at_startup);
    }

    #[test]
    fn shortcut_validation_normalizes_case_and_whitespace() {
        assert_eq!(validate_shortcut(" Ctrl+Shift+4 ").unwrap(), "ctrl+shift+4");
        assert_eq!(validate_shortcut(" Alt+Space ").unwrap(), "alt+space");
        assert!(validate_shortcut("Ctrl+Shift+F10").is_ok());
        assert!(validate_shortcut("Ctrl+F25").is_err());
        assert!(validate_shortcut("\n").is_err());
    }

    #[test]
    fn startup_command_is_quoted_and_minimized() {
        assert_eq!(
            quote_autostart_command(r#"C:\Apps\Snipsy.exe"#),
            r#""C:\Apps\Snipsy.exe" --minimized"#
        );
    }
}
