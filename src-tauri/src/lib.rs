// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod capture;
mod clipboard;
mod hotkeys;
mod library;
mod pin;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::ShortcutState;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    capture::configure_dpi_awareness();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app);
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
            let _ = hotkeys::sync_autostart();

            let new_capture =
                MenuItem::with_id(app, "new-capture", "New capture", true, None::<&str>)?;
            let open_library =
                MenuItem::with_id(app, "open-library", "Open CleanShot W", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&new_capture, &open_library, &quit])?;
            let icon = app
                .default_window_icon()
                .cloned()
                .expect("the application must provide a default tray icon");

            TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "new-capture" => {
                        let _ = app.emit("tray-new-capture", ());
                    }
                    "open-library" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            if hotkeys::should_start_minimized() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            if let WindowEvent::Destroyed = event {
                let app = window.app_handle();
                let label = window.label();
                if label == "capture-overlay" {
                    let state = app.state::<capture::CaptureState>();
                    capture::overlay_destroyed(app, state);
                } else if let Some(id) = label.strip_prefix("pin-") {
                    let state = app.state::<pin::PinState>();
                    let _ = pin::remove_pinned_capture(&state, id);
                }
            }
        })
        .manage(capture::CaptureState::default())
        .manage(pin::PinState::default())
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
            hotkeys::set_capture_hotkey,
            hotkeys::set_capture_settings,
            pin::show_pinned_capture,
            pin::get_pinned_capture,
            pin::close_pinned_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
