//! Tauri invoke commands exposed to the Svelte widget.

use crate::scanner::{self, Snapshot};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessCommandRequest {
    pub command: String,
    pub cwd: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// One live scan of dev-relevant services; called on a 3s poll from the UI.
#[tauri::command]
pub fn get_services() -> Snapshot {
    scanner::scan()
}

/// 카드의 중지 버튼: 서비스 pid에 정상 종료 신호를 보낸다.
#[tauri::command]
pub fn stop_service(pid: u32) -> Result<(), String> {
    scanner::stop::terminate(pid)
}

#[tauri::command]
pub fn start_service(request: ProcessCommandRequest) -> Result<u32, String> {
    if request.command.trim().is_empty() { return Err("run command is empty".into()); }
    let mut command = shell_command(&request.command);
    command.current_dir(&request.cwd).envs(request.env);
    command.spawn().map(|child| child.id()).map_err(|error| format!("failed to start service: {error}"))
}

#[tauri::command]
pub fn build_service(request: ProcessCommandRequest) -> Result<String, String> {
    if request.command.trim().is_empty() { return Err("build command is empty".into()); }
    let mut command = shell_command(&request.command);
    command.current_dir(&request.cwd).envs(request.env);
    let output = command.output().map_err(|error| format!("failed to run build: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() { Ok(if stdout.is_empty() { stderr } else { stdout }) }
    else { Err(if stderr.is_empty() { format!("build exited with {}", output.status) } else { stderr }) }
}

#[cfg(unix)]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("sh");
    shell.args(["-lc", command]);
    shell
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("cmd");
    shell.args(["/C", command]);
    shell
}
