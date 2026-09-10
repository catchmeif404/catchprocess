//! 카드의 "중지" 버튼이 호출하는 프로세스 종료.
//!
//! 안전 규칙:
//! - SIGTERM만 보낸다 (정상 종료 신호). 강제 종료(SIGKILL)는 일부러 제공하지 않는다.
//! - 종료 권한이 없으면(OS가 거부) 그대로 에러로 반환한다. 권한 상승은 하지 않는다.
//! - Windows는 우아한 종료 신호가 없어 `taskkill`(강제)로 대체한다. 문서에 명시됨.

use std::process::Command;

/// 프로세스에 정상 종료 신호를 보낸다. 성공해도 프로세스가 즉시 사라진다는 보장은 없다
/// (신호를 받아 정리 중일 수 있다) — 위젯은 다음 폴링에서 사라짐을 확인한다.
pub fn terminate(pid: u32) -> Result<(), String> {
    let output = terminate_command(pid)
        .output()
        .map_err(|error| format!("failed to spawn kill command: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(if stderr.trim().is_empty() {
            format!("failed to stop process {pid}")
        } else {
            format!("failed to stop process {pid}: {}", stderr.trim())
        })
    }
}

#[cfg(unix)]
fn terminate_command(pid: u32) -> Command {
    let mut command = Command::new("kill");
    command.arg("-TERM").arg(pid.to_string());
    command
}

#[cfg(windows)]
fn terminate_command(pid: u32) -> Command {
    let mut command = Command::new("taskkill");
    command.arg("/PID").arg(pid.to_string()).arg("/F");
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn terminate_reports_error_for_nonexistent_pid() {
        // 실존하지 않는 pid로 OS 거부 응답이 에러로 매핑되는지 확인.
        let result = terminate(4_000_000_000);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("4000000000"));
    }
}
