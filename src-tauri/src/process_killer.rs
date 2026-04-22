use crate::types::KillResult;
use std::process::Command;

#[cfg(target_os = "windows")]
pub fn kill_process(pid: u32) -> Result<KillResult, String> {
    let output = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to execute taskkill: {}", e))?;

    if output.status.success() {
        Ok(KillResult {
            pid,
            success: true,
            message: "Process killed".into(),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        Err(if msg.is_empty() {
            format!("taskkill exited with code {:?}", output.status.code())
        } else {
            msg.to_string()
        })
    }
}

#[cfg(not(target_os = "windows"))]
pub fn kill_process(pid: u32) -> Result<KillResult, String> {
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("Failed to execute kill: {}", e))?;

    if output.status.success() {
        Ok(KillResult {
            pid,
            success: true,
            message: "Process killed".into(),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        Err(if msg.is_empty() {
            format!("kill exited with code {:?}", output.status.code())
        } else {
            msg.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_nonexistent_pid() {
        let result = kill_process(99999999);
        assert!(result.is_err(), "Killing a nonexistent PID should return an error");
    }

    #[test]
    fn test_kill_zero_pid() {
        let result = kill_process(0);
        assert!(result.is_err(), "Killing PID 0 should return an error");
    }
}
