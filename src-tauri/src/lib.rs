mod commands;
mod scanner;

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default()
            .with_state_flags(tauri_plugin_window_state::StateFlags::empty())
            .build())
        .setup(|app| {
            let show = MenuItemBuilder::with_id("show", "Show DevTopology").build(app)?;
            let hide = MenuItemBuilder::with_id("hide", "Hide window").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show, &hide, &separator, &quit])
                .build()?;
            TrayIconBuilder::new()
                .icon(tauri::image::Image::new(&[163, 43, 43, 255], 1, 1))
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let Some(window) = app.get_webview_window("main") else { return; };
                    match event.id().as_ref() {
                        "show" => {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        "hide" => { let _ = window.hide(); }
                        "quit" => app.exit(0),
                        _ => {}
                    }
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_services,
            commands::stop_service,
            commands::start_service,
            commands::build_service
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
