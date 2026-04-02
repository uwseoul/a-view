pub mod commands;
pub mod dashboard_service;
pub mod opencode_adapter;
pub mod stall_detector;
pub mod types;

pub use types::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_snapshot,
            commands::get_health,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
