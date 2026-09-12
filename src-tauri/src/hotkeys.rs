use crate::library::{app_data_root, now_millis};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
#[cfg(feature = "app")]
use tauri::AppHandle;
#[cfg(feature = "app")]
use tauri_plugin_global_shortcut::GlobalShortcutExt;

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
            return Ok(AppSettings::default())
        }
        Err(error) => return Err(format!("Could not read settings: {error}")),
    };
    let mut settings: AppSettings = serde_json::from_slice(&bytes).unwrap_or_default();
    settings.capture_hotkey = validate_shortcut(&settings.capture_hotkey)
        .unwrap_or_else(|_| DEFAULT_CAPTURE_HOTKEY.to_string());
    Ok(settings)
}

pub(crate) fn current_settings() -> AppSettings {
    read_settings().unwrap_or_default()
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

#[cfg(feature = "app")]
pub fn register_saved_hotkey(app: &AppHandle) -> Result<(), String> {
    let settings = read_settings()?;
    register_hotkey(app, &settings.capture_hotkey)
}

#[cfg(feature = "app")]
fn register_hotkey(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    if app.global_shortcut().is_registered(shortcut) {
        return Ok(());
    }
    app.global_shortcut().register(shortcut).map_err(|error| {
        format!("Could not register {shortcut}: {error}. Choose another shortcut.")
    })
}

#[cfg(feature = "app")]
fn unregister_hotkey(app: &AppHandle, shortcut: &str) {
    let _ = app.global_shortcut().unregister(shortcut);
}

#[cfg(feature = "app")]
fn apply_settings(app: &AppHandle, next: AppSettings) -> Result<AppSettings, String> {
    let previous = read_settings()?;
    let shortcut_changed = previous.capture_hotkey != next.capture_hotkey;

    if shortcut_changed {
        unregister_hotkey(app, &previous.capture_hotkey);
        if let Err(error) = register_hotkey(app, &next.capture_hotkey) {
            let _ = register_hotkey(app, &previous.capture_hotkey);
            return Err(error);
        }
    } else {
        register_hotkey(app, &next.capture_hotkey)?;
    }

    if let Err(error) = set_autostart(next.launch_at_startup) {
        if shortcut_changed {
            unregister_hotkey(app, &next.capture_hotkey);
            let _ = register_hotkey(app, &previous.capture_hotkey);
        }
        return Err(error);
    }

    if let Err(error) = write_settings(&next) {
        let _ = set_autostart(previous.launch_at_startup);
        if shortcut_changed {
            unregister_hotkey(app, &next.capture_hotkey);
            let _ = register_hotkey(app, &previous.capture_hotkey);
        }
        return Err(error);
    }

    Ok(next)
}

pub fn sync_autostart() -> Result<(), String> {
    let settings = read_settings()?;
    set_autostart(settings.launch_at_startup)
}

pub fn should_start_minimized() -> bool {
    std::env::args().any(|arg| arg == "--minimized")
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

    let sub_key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "CleanShotW\0".encode_utf16().collect();
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

    let result = if enabled {
        let executable = std::env::current_exe()
            .map_err(|error| format!("Could not locate the application executable: {error}"))?;
        let command = quote_autostart_command(&executable.to_string_lossy());
        let value: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = u32::try_from(value.len() * std::mem::size_of::<u16>())
            .map_err(|_| "The startup command is too long.".to_string())?;
        unsafe {
            RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                byte_len,
            )
        }
    } else {
        let status = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
        if status == ERROR_FILE_NOT_FOUND {
            ERROR_SUCCESS
        } else {
            status
        }
    };
    unsafe { RegCloseKey(key) };

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

fn quote_autostart_command(executable: &str) -> String {
    format!("\"{}\" --minimized", executable.replace('"', "\\\""))
}

#[cfg(feature = "app")]
#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    read_settings()
}

#[cfg(feature = "app")]
#[tauri::command]
pub fn ensure_capture_hotkey(app: AppHandle) -> Result<(), String> {
    let settings = read_settings()?;
    register_hotkey(&app, &settings.capture_hotkey)
}

#[cfg(feature = "app")]
#[tauri::command]
pub fn set_capture_settings(
    app: AppHandle,
    capture_hotkey: String,
    include_cursor: bool,
    launch_at_startup: bool,
) -> Result<AppSettings, String> {
    let previous = read_settings()?;
    let capture_hotkey = validate_shortcut(&capture_hotkey)?;
    apply_settings(
        &app,
        AppSettings {
            capture_hotkey,
            include_cursor,
            launch_at_startup,
        },
    )
    .inspect_err(|_error| {
        let _ = write_settings(&previous);
    })
}

#[cfg(feature = "app")]
#[tauri::command]
pub fn set_capture_hotkey(app: AppHandle, shortcut: String) -> Result<AppSettings, String> {
    let previous = read_settings()?;
    let capture_hotkey = validate_shortcut(&shortcut)?;
    apply_settings(
        &app,
        AppSettings {
            capture_hotkey,
            include_cursor: previous.include_cursor,
            launch_at_startup: previous.launch_at_startup,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{quote_autostart_command, validate_shortcut, AppSettings, DEFAULT_CAPTURE_HOTKEY};

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
        assert!(validate_shortcut("\n").is_err());
    }

    #[test]
    fn startup_command_is_quoted_and_minimized() {
        let command = quote_autostart_command(r#"C:\Apps\CleanShot W.exe"#);
        assert_eq!(command, r#""C:\Apps\CleanShot W.exe" --minimized"#);
    }
}
