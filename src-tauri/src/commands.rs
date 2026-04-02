use crate::dashboard_service;
use crate::types::Snapshot;
use chrono::Utc;

#[tauri::command]
pub fn get_dashboard_snapshot() -> Result<Snapshot, String> {
    let now = Utc::now();
    let limit = 100;
    let db_path = None;

    dashboard_service::build_snapshot(now, limit, db_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_health() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "ok": true }))
}
