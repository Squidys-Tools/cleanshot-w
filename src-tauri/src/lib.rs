// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod capture;
mod clipboard;
mod hotkeys;
mod library;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = app.emit("global-hotkey-pressed", shortcut.to_string());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let _ = hotkeys::register_saved_hotkey(app.handle());
            Ok(())
        })
        .manage(capture::CaptureState::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            capture::capture_screen,
            capture::list_capture_windows,
            capture::capture_window,
            capture::get_active_capture,
            capture::start_area_capture,
            capture::complete_area_capture,
            capture::cancel_area_capture,
            clipboard::copy_image_to_clipboard,
            clipboard::read_image_from_clipboard,
            clipboard::copy_file_to_clipboard,
            clipboard::copy_text_to_clipboard,
            library::library_save_capture,
            library::library_list_captures,
            library::library_get_capture,
            library::library_update_annotations,
            library::library_delete_capture,
            hotkeys::get_settings,
            hotkeys::ensure_capture_hotkey,
            hotkeys::set_capture_hotkey
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
