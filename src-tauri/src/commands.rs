use crate::app_state::AppState;
use crate::dashboard_service;
use crate::port_scanner;
use crate::process_killer;
use crate::sleep_manager;
use crate::types::{KillResult, PortScanResult, SleepStatus, Snapshot};
use chrono::Utc;
use tauri::State;

#[tauri::command]
pub fn get_dashboard_snapshot() -> Result<Snapshot, String> {
    let now = Utc::now();
    let limit = 50;
    let db_path = None;

    dashboard_service::build_snapshot(now, limit, db_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_health() -> Result<serde_json::Value, String> {
    use std::env;

    let home = env::var("HOME").unwrap_or_else(|_| "NOT_SET".to_string());
    let userprofile = env::var("USERPROFILE").unwrap_or_else(|_| "NOT_SET".to_string());

    // Test the SAME path resolution as read_raw_sessions
    let default_path = "~/.local/share/opencode/opencode.db";
    let resolved_path = if default_path.starts_with("~/") {
        if let Ok(h) = env::var("HOME") {
            default_path.replacen("~", &h, 1)
        } else if let Ok(h) = env::var("USERPROFILE") {
            default_path.replacen("~", &h, 1)
        } else {
            default_path.to_string()
        }
    } else {
        default_path.to_string()
    };

    let db_status = match rusqlite::Connection::open(&resolved_path) {
        Ok(conn) => {
            // Test exact same query as read_raw_sessions
            let count_result: Result<i64, _> = conn.query_row(
                "SELECT count(*) FROM session WHERE time_archived IS NULL",
                [],
                |row| row.get(0),
            );
            let count = match count_result {
                Ok(n) => format!("{} rows", n),
                Err(e) => format!("count err: {}", e),
            };

            // Test row mapping like read_raw_sessions does
            let mut stmt = conn.prepare(
                "SELECT id, title, directory, parent_id, time_created, time_updated FROM session WHERE time_archived IS NULL ORDER BY time_updated DESC LIMIT 1"
            ).unwrap();

            let row_result: Result<String, _> = stmt.query_row([], |row| {
                let id: String = row.get(0)?;
                Ok(id)
            });

            let row_test = match row_result {
                Ok(id) => format!("row ok id={}", &id[..12.min(id.len())]),
                Err(e) => format!("row err: {}", e),
            };

            format!("path_ok, count={}, test={}", count, row_test)
        }
        Err(e) => format!("open_err: {} (path={})", e, resolved_path),
    };

    Ok(serde_json::json!({
        "HOME": home,
        "USERPROFILE": userprofile,
        "resolved_path": resolved_path,
        "db_status": db_status,
    }))
}

#[tauri::command]
pub fn get_sleep_status(state: State<'_, AppState>) -> Result<SleepStatus, String> {
    Ok(sleep_manager::get_sleep_status(&state))
}

#[tauri::command]
pub fn set_sleep_prevention(
    state: State<'_, AppState>,
    prevent: bool,
    reason: String,
    active_agents: i64,
) -> Result<SleepStatus, String> {
    if prevent {
        sleep_manager::start_prevent_sleep(&state, active_agents, &reason)
    } else {
        sleep_manager::stop_prevent_sleep(&state, &reason)
    }
}

#[tauri::command]
pub fn scan_ports() -> Result<PortScanResult, String> {
    port_scanner::scan_ports()
}

#[tauri::command]
pub fn kill_port_process(pid: u32) -> Result<KillResult, String> {
    process_killer::kill_process(pid)
}
