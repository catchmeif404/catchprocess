mod commands;
mod scanner;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_services,
            commands::stop_service
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
