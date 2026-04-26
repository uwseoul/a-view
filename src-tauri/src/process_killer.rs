use crate::types::KillResult;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Windows 콘솔 출력을 적절한 인코딩으로 디코딩합니다.
/// Windows 한국어판은 CP949(EUC-KR), 최신 Windows 터미널은 UTF-8을 사용할 수 있습니다.
fn decode_windows_output(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    // 1. 먼저 UTF-8 시도 (최신 Windows 터미널, 영문 시스템 등)
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.trim().to_string();
    }

    // 2. CP949(EUC-KR) 디코딩 시도 - Windows 한국어판
    let (cow, _, _had_errors) = encoding_rs::EUC_KR.decode(bytes);
    let decoded = cow.into_owned();
    if !decoded.is_empty() {
        return decoded.trim().to_string();
    }

    // 3. 최후의 수단: lossy UTF-8
    String::from_utf8_lossy(bytes).trim().to_string()
}

/// 프로세스가 실행 중인지 확인합니다.
#[cfg(target_os = "windows")]
fn is_process_running(pid: u32) -> bool {
    let output = match Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let out = decode_windows_output(&output.stdout);
    out.contains(&pid.to_string())
}

#[cfg(not(target_os = "windows"))]
fn is_process_running(pid: u32) -> bool {
    Command::new("ps")
        .args(["-p", &pid.to_string()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// 프로세스가 종료될 때까지 최대 retries 횟수만큼 대기합니다.
fn wait_for_exit(pid: u32, retries: u32, interval_ms: u64) -> bool {
    for _ in 0..retries {
        if !is_process_running(pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(interval_ms));
    }
    false
}

#[cfg(target_os = "windows")]
pub fn kill_process(pid: u32) -> Result<KillResult, String> {
    // 1. 프로세스 존재 여부 사전 확인
    if !is_process_running(pid) {
        return Ok(KillResult {
            pid,
            success: true,
            message: "프로세스가 이미 종료되어 있습니다.".into(),
        });
    }

    // 2. taskkill 실행 (강제 종료 + 프로세스 트리 종료)
    let output = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("종료 명령을 실행할 수 없습니다: {}", e))?;

    let stdout = decode_windows_output(&output.stdout);
    let stderr = decode_windows_output(&output.stderr);

    // 3. taskkill 결과 판단
    //    - exit code 0이고 stderr가 비어있으면 일반적으로 성공
    //    - 하지만 /T(트리 종료) 사용 시 일부는 성공하고 일부 실패할 수 있음
    let has_success = stdout.contains("성공")
        || stdout.to_lowercase().contains("success")
        || stdout.to_lowercase().contains("terminated");
    let has_error = stderr.contains("오류")
        || stderr.to_lowercase().contains("error")
        || stderr.contains("찾을 수 없")
        || stderr.contains("액세스 거부")
        || stderr.contains("access is denied");

    // taskkill이 성공 코드를 반환했거나, stdout에 성공 키워드가 있고 stderr에 오류가 없는 경우
    let command_succeeded = (output.status.success() && !has_error) || (has_success && !has_error);

    if command_succeeded {
        // 4. 실제로 종료되었는지 확인 (최대 5회, 300ms 간격)
        if wait_for_exit(pid, 5, 300) {
            Ok(KillResult {
                pid,
                success: true,
                message: if stdout.is_empty() {
                    "프로세스가 종료되었습니다.".into()
                } else {
                    format!("프로세스가 종료되었습니다. ({})", stdout)
                },
            })
        } else {
            // 명령은 성공했으나 프로세스가 남아있음 (보호된 프로세스, 권한 문제 등)
            Err(format!(
                "종료 명령은 실행되었으나 프로세스(PID: {})가 여전히 실행 중입니다. 관리자 권한이 필요하거나 시스템 프로세스일 수 있습니다.",
                pid
            ))
        }
    } else {
        // 5. 실패 처리 - 사용자 친화적인 메시지 생성
        let mut reasons = Vec::new();

        if has_error {
            if stderr.contains("찾을 수 없") || stderr.to_lowercase().contains("cannot find") {
                reasons.push("지정된 PID의 프로세스를 찾을 수 없습니다.".to_string());
            } else if stderr.contains("액세스 거부") || stderr.to_lowercase().contains("access is denied")
            {
                reasons.push("접근이 거부되었습니다. 관리자 권한으로 실행해 보세요.".to_string());
            } else {
                reasons.push(format!("시스템 메시지: {}", stderr));
            }
        } else if !stderr.is_empty() {
            reasons.push(format!("시스템 메시지: {}", stderr));
        }

        if !stdout.is_empty() {
            reasons.push(format!("출력: {}", stdout));
        }

        if let Some(code) = output.status.code() {
            if code != 0 {
                reasons.push(format!("종료 코드: {}", code));
            }
        }

        if reasons.is_empty() {
            Err(format!("프로세스(PID: {}) 종료에 실패했습니다.", pid))
        } else {
            Err(format!(
                "프로세스(PID: {}) 종료에 실패했습니다.\n{}",
                pid,
                reasons.join("\n")
            ))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn kill_process(pid: u32) -> Result<KillResult, String> {
    // 1. 사전 확인
    if !is_process_running(pid) {
        return Ok(KillResult {
            pid,
            success: true,
            message: "프로세스가 이미 종료되어 있습니다.".into(),
        });
    }

    // 2. kill -9 실행
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("kill 명령 실행 실패: {}", e))?;

    if output.status.success() {
        if wait_for_exit(pid, 5, 300) {
            Ok(KillResult {
                pid,
                success: true,
                message: "프로세스가 종료되었습니다.".into(),
            })
        } else {
            Err(format!(
                "종료 명령은 실행되었으나 프로세스(PID: {})가 여전히 실행 중입니다.",
                pid
            ))
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = stderr.trim();
        Err(if msg.is_empty() {
            format!("kill 명령이 비정상 종료되었습니다 (코드: {:?})", output.status.code())
        } else {
            format!("프로세스 종료 실패: {}", msg)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_nonexistent_pid() {
        let result = kill_process(99999999);
        // 존재하지 않는 프로세스는 "이미 종료"로 처리하여 사용자에게 친화적으로 응답
        assert!(result.is_ok(), "존재하지 않는 PID는 성공으로 처리해야 함");
        let kr = result.unwrap();
        assert!(kr.success);
        assert!(kr.message.contains("이미 종료"));
    }

    #[test]
    fn test_kill_zero_pid() {
        let result = kill_process(0);
        // PID 0은 System Idle Process로 tasklist에 존재할 수 있으나 종료할 수 없음.
        // taskkill이 접근 거부 또는 실패를 반환해야 함.
        assert!(result.is_err(), "PID 0 종료는 실패해야 함");
    }
}
