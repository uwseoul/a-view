use crate::app_state::AppState;
use crate::types::SleepStatus;
use chrono::Utc;

pub fn start_prevent_sleep(state: &AppState, active_agents: i64, reason: &str) -> Result<SleepStatus, String> {
    let guard = keepawake::Builder::default()
        .display(true)
        .idle(true)
        .create()
        .map_err(|e| e.to_string())?;

    let timestamp = Utc::now().to_rfc3339();

    let mut sleep_guard = state.sleep_guard.lock().map_err(|e| e.to_string())?;
    *sleep_guard = Some(guard);

    eprintln!("[SleepManager] Started - reason: {}, agents: {}", reason, active_agents);

    Ok(SleepStatus {
        is_preventing: true,
        reason: reason.to_string(),
        active_agents,
        last_changed_at: timestamp,
    })
}

pub fn stop_prevent_sleep(state: &AppState, reason: &str) -> Result<SleepStatus, String> {
    let timestamp = Utc::now().to_rfc3339();

    let mut sleep_guard = state.sleep_guard.lock().map_err(|e| e.to_string())?;
    *sleep_guard = None;

    eprintln!("[SleepManager] Stopped - reason: {}", reason);

    Ok(SleepStatus {
        is_preventing: false,
        reason: reason.to_string(),
        active_agents: 0,
        last_changed_at: timestamp,
    })
}

pub fn get_sleep_status(state: &AppState) -> SleepStatus {
    let sleep_guard = match state.sleep_guard.lock() {
        Ok(guard) => guard,
        Err(_) => return default_sleep_status(),
    };

    let is_preventing = sleep_guard.is_some();
    SleepStatus {
        is_preventing,
        reason: if is_preventing { "Active".to_string() } else { "Idle".to_string() },
        active_agents: 0,
        last_changed_at: Utc::now().to_rfc3339(),
    }
}

fn default_sleep_status() -> SleepStatus {
    SleepStatus {
        is_preventing: false,
        reason: "Idle".to_string(),
        active_agents: 0,
        last_changed_at: Utc::now().to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_sleep_status_default() {
        let state = AppState::default();
        let status = get_sleep_status(&state);

        assert!(!status.is_preventing);
        assert_eq!(status.reason, "Idle");
        assert_eq!(status.active_agents, 0);
        assert!(!status.last_changed_at.is_empty());
    }

    #[test]
    fn test_start_stop_cycle() {
        let state = AppState::default();

        let result = start_prevent_sleep(&state, 3, "test-reason");
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.is_preventing);
        assert_eq!(status.active_agents, 3);
        assert_eq!(status.reason, "test-reason");

        let current = get_sleep_status(&state);
        assert!(current.is_preventing);

        let stop_result = stop_prevent_sleep(&state, "test-stop");
        assert!(stop_result.is_ok());
        let stopped = stop_result.unwrap();
        assert!(!stopped.is_preventing);

        let after = get_sleep_status(&state);
        assert!(!after.is_preventing);
    }
}
