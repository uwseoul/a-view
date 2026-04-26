use tauri::Manager;

pub mod app_state;
pub mod commands;
pub mod dashboard_service;
pub mod opencode_adapter;
pub mod stall_detector;
pub mod port_scanner;
pub mod process_killer;
pub mod sleep_manager;
pub mod types;

pub use types::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(app_state::AppState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_snapshot,
            commands::get_health,
            commands::get_sleep_status,
            commands::set_sleep_prevention,
            commands::scan_ports,
            commands::kill_port_process,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
