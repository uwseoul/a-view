use crate::app_state::AppState;
use crate::types::SleepStatus;
use chrono::Utc;

pub fn start_prevent_sleep(state: &AppState, active_agents: i64, reason: &str) -> Result<SleepStatus, String> {
    // Check if already preventing — skip if guard exists
    {
        let guard = state.sleep_guard.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            // Already active — return current status without recreating guard
            let started_at = state.sleep_started_at.lock().ok().and_then(|s| s.clone());
            eprintln!("[SleepManager] Already active - skipping (agents: {})", active_agents);
            return Ok(SleepStatus {
                is_preventing: true,
                reason: reason.to_string(),
                active_agents,
                last_changed_at: Utc::now().to_rfc3339(),
                display: true,
                idle: true,
                started_at,
            });
        }
    }

    let guard = keepawake::Builder::default()
        .display(true)
        .idle(true)
        .create()
        .map_err(|e| e.to_string())?;

    let timestamp = Utc::now().to_rfc3339();

    let mut sleep_guard = state.sleep_guard.lock().map_err(|e| e.to_string())?;
    *sleep_guard = Some(guard);

    // Track when prevention started
    if let Ok(mut started) = state.sleep_started_at.lock() {
        if started.is_none() {
            *started = Some(timestamp.clone());
        }
    }

    let started_at = state.sleep_started_at.lock().ok().and_then(|s| s.clone());

    eprintln!("[SleepManager] Started - reason: {}, agents: {}", reason, active_agents);

    Ok(SleepStatus {
        is_preventing: true,
        reason: reason.to_string(),
        active_agents,
        last_changed_at: timestamp,
        display: true,
        idle: true,
        started_at,
    })
}

pub fn stop_prevent_sleep(state: &AppState, reason: &str) -> Result<SleepStatus, String> {
    // Check if already idle — skip if guard doesn't exist
    {
        let guard = state.sleep_guard.lock().map_err(|e| e.to_string())?;
        if guard.is_none() {
            eprintln!("[SleepManager] Already idle - skipping");
            return Ok(SleepStatus {
                is_preventing: false,
                reason: reason.to_string(),
                active_agents: 0,
                last_changed_at: Utc::now().to_rfc3339(),
                display: false,
                idle: false,
                started_at: None,
            });
        }
    }

    let timestamp = Utc::now().to_rfc3339();

    let mut sleep_guard = state.sleep_guard.lock().map_err(|e| e.to_string())?;
    *sleep_guard = None;

    // Clear started_at
    if let Ok(mut started) = state.sleep_started_at.lock() {
        *started = None;
    }

    eprintln!("[SleepManager] Stopped - reason: {}", reason);

    Ok(SleepStatus {
        is_preventing: false,
        reason: reason.to_string(),
        active_agents: 0,
        last_changed_at: timestamp,
        display: false,
        idle: false,
        started_at: None,
    })
}

pub fn get_sleep_status(state: &AppState) -> SleepStatus {
    let sleep_guard = match state.sleep_guard.lock() {
        Ok(guard) => guard,
        Err(_) => return default_sleep_status(),
    };

    let is_preventing = sleep_guard.is_some();
    let started_at = state.sleep_started_at.lock().ok().and_then(|s| s.clone());

    SleepStatus {
        is_preventing,
        reason: if is_preventing { "agent_running".to_string() } else { "idle".to_string() },
        active_agents: 0,
        last_changed_at: Utc::now().to_rfc3339(),
        display: is_preventing,
        idle: is_preventing,
        started_at,
    }
}

fn default_sleep_status() -> SleepStatus {
    SleepStatus {
        is_preventing: false,
        reason: "idle".to_string(),
        active_agents: 0,
        last_changed_at: Utc::now().to_rfc3339(),
        display: false,
        idle: false,
        started_at: None,
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
        assert_eq!(status.reason, "idle");
        assert_eq!(status.active_agents, 0);
        assert!(!status.last_changed_at.is_empty());
        assert!(status.started_at.is_none());
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
        assert!(status.display);
        assert!(status.idle);
        assert!(status.started_at.is_some());

        let current = get_sleep_status(&state);
        assert!(current.is_preventing);
        assert!(current.started_at.is_some());

        let stop_result = stop_prevent_sleep(&state, "test-stop");
        assert!(stop_result.is_ok());
        let stopped = stop_result.unwrap();
        assert!(!stopped.is_preventing);
        assert!(!stopped.display);
        assert!(!stopped.idle);
        assert!(stopped.started_at.is_none());

        let after = get_sleep_status(&state);
        assert!(!after.is_preventing);
        assert!(after.started_at.is_none());
    }
}
