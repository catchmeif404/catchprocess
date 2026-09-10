//! 카드의 "중지" 버튼이 호출하는 프로세스 종료.
//!
//! 동작: 먼저 SIGTERM(정상 종료)을 보내고 짧은 유예 시간 안에 죽는지 확인한다.
//! 안 죽으면 SIGKILL로 강제 종료한다 — 사용자는 "중지 즉시 꺼짐"을 기대하므로
//! 최악의 경우에도 KILL_GRACE 안에는 사라진다.
//! 권한이 없으면(OS가 거부) 그대로 에러로 반환한다. 권한 상승은 하지 않는다.
//! Windows에는 우아한 종료 신호가 없어 처음부터 `taskkill /F`(강제)로 처리한다.

use std::process::Command;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// SIGTERM 후 정상 종료를 기다리는 유예 시간.
const TERM_GRACE: Duration = Duration::from_millis(1500);
/// SIGKILL 후에도 남아 있으면 에러 처리하기까지의 대기.
const KILL_GRACE: Duration = Duration::from_millis(800);
/// 프로세스 소멸 확인 주기.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Copy)]
enum Signal {
    Term,
    Kill,
}

/// 프로세스를 종료하고, 실제로 사라진 것을 확인한 뒤 반환한다.
/// 이미 죽어 있는 pid는 성공으로 본다 (목표 달성).
pub fn terminate(pid: u32) -> Result<(), String> {
    match send(pid, Signal::Term) {
        Ok(()) => {}
        Err(error) => {
            // 신호 전송 실패: 이미 죽어 있으면 성공, 아니면 그 이유를 반환.
            return if is_gone(pid) { Ok(()) } else { Err(error) };
        }
    }
    if wait_until_gone(pid, TERM_GRACE) {
        return Ok(());
    }
    // 유예 시간 내 미종료 → 강제 종료.
    send(pid, Signal::Kill)?;
    if wait_until_gone(pid, KILL_GRACE) {
        return Ok(());
    }
    Err(format!("process {pid} did not exit"))
}

fn send(pid: u32, signal: Signal) -> Result<(), String> {
    let mut command = terminate_command(pid, signal);
    let output = command
        .output()
        .map_err(|error| format!("failed to spawn kill command: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(format!("failed to stop process {pid}"))
    } else {
        Err(format!("failed to stop process {pid}: {stderr}"))
    }
}

#[cfg(unix)]
fn terminate_command(pid: u32, signal: Signal) -> Command {
    let mut command = Command::new("kill");
    command
        .arg(match signal {
            Signal::Term => "-TERM",
            Signal::Kill => "-KILL",
        })
        .arg(pid.to_string());
    command
}

#[cfg(windows)]
fn terminate_command(pid: u32, _signal: Signal) -> Command {
    // Windows는 우아한 종료 신호가 없다 — 처음부터 강제 종료.
    let mut command = Command::new("taskkill");
    command.arg("/PID").arg(pid.to_string()).arg("/F");
    command
}

/// 프로세스가 더 이상 존재하지 않는지 확인한다.
fn is_gone(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        false,
        ProcessRefreshKind::nothing(),
    );
    system.process(Pid::from_u32(pid)).is_none()
}

/// 유예 시간 안에 프로세스가 사라지는지 폴링한다.
fn wait_until_gone(pid: u32, grace: Duration) -> bool {
    let deadline = Instant::now() + grace;
    loop {
        if is_gone(pid) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminate_succeeds_when_already_gone() {
        // 실존하지 않는 pid: 목표(프로세스 없음)가 이미 달성된 상태이므로 성공.
        assert!(terminate(4_000_000_000).is_ok());
    }

    #[test]
    fn wait_until_gone_returns_true_for_nonexistent_pid() {
        assert!(wait_until_gone(4_000_000_000, Duration::from_millis(200)));
    }
}
