mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(storage::StorageState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            storage::initialize_storage,
            storage::list_active_items,
            storage::create_note,
            storage::update_item,
            storage::archive_item,
            storage::search_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
