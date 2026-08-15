mod jobs;
mod ocr;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(storage::StorageState::default())
        .manage(jobs::ProcessingState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            storage::initialize_storage,
            storage::list_active_items,
            storage::create_note,
            storage::create_url,
            storage::save_file,
            storage::resolve_asset_path,
            storage::update_item,
            storage::archive_item,
            storage::search_items,
            jobs::enqueue_ocr_job,
            jobs::get_job_status,
            jobs::count_active_jobs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
