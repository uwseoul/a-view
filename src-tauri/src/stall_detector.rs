use crate::types::Status;
use chrono::{DateTime, Utc};

/// Constants matching the original JavaScript implementation
pub const RUNNING_THRESHOLD_SEC: u64 = 30;
pub const STALLED_THRESHOLD_SEC: u64 = 45;

/// Clamps a value to a finite non-negative integer, or returns null
/// Mirrors: `Number.isFinite(value) && value >= 0 ? Math.floor(value) : null`
fn clamp_age_sec(value: Option<f64>) -> Option<u64> {
    value.and_then(|v| {
        if v.is_finite() && v >= 0.0 {
            Some(v as u64)
        } else {
            None
        }
    })
}

/// Converts milliseconds since Unix epoch to seconds since last activity
/// Mirrors: `(now - new Date(lastActivityAt).getTime()) / 1000`
fn age_sec_from(last_activity_at: Option<&str>, now: Option<DateTime<Utc>>) -> Option<u64> {
    match (last_activity_at, now) {
        (Some(timestamp), Some(n)) => {
            // Parse ISO 8601 format (e.g., "2026-04-01T19:02:19.600Z")
            if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
                let now_ms = n.timestamp_millis();
                let dt_ms = dt.timestamp_millis();
                if now_ms > dt_ms {
                    Some(((now_ms - dt_ms) / 1000) as u64)
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Classifies agent status based on explicit status, running tool presence, and age
/// Mirrors the classifyStatus function from server/stall-detector.js
pub fn classify_status(
    explicit_status: Option<&str>,
    has_running_tool: bool,
    last_activity_at: Option<&str>,
    now: DateTime<Utc>,
) -> Status {
    let norm = explicit_status.map(|s| s.to_lowercase());

    match norm.as_deref() {
        Some("failed") | Some("error") => Status::Failed,
        Some("running") => Status::Running,
        Some("completed") | Some("success") | Some("done") => {
            if has_running_tool {
                Status::Running
            } else {
                Status::Completed
            }
        }
        _ => {
            // No explicit status, classify based on age
            let age_sec = age_sec_from(last_activity_at, Some(now));

            match age_sec {
                None => {
                    if has_running_tool {
                        Status::Running
                    } else {
                        Status::Delayed
                    }
                }
                Some(age) if age < RUNNING_THRESHOLD_SEC => Status::Running,
                Some(age) if age < STALLED_THRESHOLD_SEC => Status::Delayed,
                _ => Status::Stalled,
            }
        }
    }
}

/// Tauri command to check if a specific agent is stalled
#[tauri::command]
pub fn check_agent_stalled(
    explicit_status: Option<String>,
    has_running_tool: bool,
    last_activity_at: Option<String>,
) -> Result<bool, String> {
    let now = Utc::now();
    let status = classify_status(
        explicit_status.as_deref(),
        has_running_tool,
        last_activity_at.as_deref(),
        now,
    );
    Ok(status == Status::Stalled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_age_sec() {
        assert_eq!(clamp_age_sec(Some(42.0)), Some(42));
        assert_eq!(clamp_age_sec(Some(-5.0)), None);
        assert_eq!(clamp_age_sec(None), None);
        assert_eq!(clamp_age_sec(Some(f64::MAX)), None);
        assert_eq!(clamp_age_sec(Some(f64::INFINITY)), None);
    }

    #[test]
    fn test_classify_status_explicit_failed() {
        let now = Utc::now();
        assert_eq!(
            classify_status(Some("failed"), false, None, now),
            Status::Failed
        );
        assert_eq!(
            classify_status(Some("error"), false, None, now),
            Status::Failed
        );
    }

    #[test]
    fn test_classify_status_explicit_running() {
        let now = Utc::now();
        assert_eq!(
            classify_status(Some("running"), false, None, now),
            Status::Running
        );
    }

    #[test]
    fn test_classify_status_explicit_completed() {
        let now = Utc::now();
        // completed without running tool -> Completed
        assert_eq!(
            classify_status(Some("completed"), false, None, now),
            Status::Completed
        );
        // success without running tool -> Completed
        assert_eq!(
            classify_status(Some("success"), false, None, now),
            Status::Completed
        );
        // done without running tool -> Completed
        assert_eq!(
            classify_status(Some("done"), false, None, now),
            Status::Completed
        );
        // completed WITH running tool -> Running
        assert_eq!(
            classify_status(Some("completed"), true, None, now),
            Status::Running
        );
    }

    #[test]
    fn test_classify_status_by_age() {
        let now = Utc::now();
        let recent_time = (now - chrono::Duration::seconds(20)).to_rfc3339();
        let delayed_time = (now - chrono::Duration::seconds(35)).to_rfc3339();
        let stalled_time = (now - chrono::Duration::seconds(50)).to_rfc3339();

        // No explicit status, no running tool, recent activity -> Running
        assert_eq!(
            classify_status(None, false, Some(&recent_time), now),
            Status::Running
        );

        // No explicit status, no running tool, delayed activity -> Delayed
        assert_eq!(
            classify_status(None, false, Some(&delayed_time), now),
            Status::Delayed
        );

        // No explicit status, no running tool, stalled activity -> Stalled
        assert_eq!(
            classify_status(None, false, Some(&stalled_time), now),
            Status::Stalled
        );

        // No explicit status, WITH running tool -> Running (regardless of age)
        assert_eq!(
            classify_status(None, true, Some(&stalled_time), now),
            Status::Running
        );
    }

    #[test]
    fn test_classify_status_null_age_no_tool() {
        let now = Utc::now();
        // null age, no running tool -> Delayed
        assert_eq!(classify_status(None, false, None, now), Status::Delayed);
        // null age, with running tool -> Running
        assert_eq!(classify_status(None, true, None, now), Status::Running);
    }
}
