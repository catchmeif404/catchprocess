//! Tauri invoke commands exposed to the Svelte widget.

use crate::scanner::{self, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessCommandRequest {
    pub command: String,
    pub cwd: String,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedService {
    pub key: String,
    pub name: String,
    pub cwd: String,
    pub build_command: String,
    pub run_command: String,
    pub env_text: String,
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
pub fn get_managed_services() -> Result<Vec<ManagedService>, String> {
    let path = managed_services_path()?;
    if !path.exists() { return Ok(Vec::new()); }
    let text = fs::read_to_string(&path).map_err(|error| format!("failed to read service config: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid service config: {error}"))
}

#[tauri::command]
pub fn save_managed_services(services: Vec<ManagedService>) -> Result<(), String> {
    let path = managed_services_path()?;
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|error| format!("failed to create config directory: {error}"))?; }
    let text = serde_json::to_string_pretty(&services).map_err(|error| format!("failed to encode service config: {error}"))?;
    fs::write(&path, format!("{text}\n")).map_err(|error| format!("failed to write service config: {error}"))?;
    Ok(())
}

fn managed_services_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(PathBuf::from).map(|home| home.join("Library/Application Support/DevTopology"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from).map(|appdata| appdata.join("DevTopology"));
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))).map(|path| path.join("devtopology"));
    base.map(|path| path.join("services.json")).ok_or_else(|| "could not determine config directory".into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_forwards_environment_and_working_directory() {
        let cwd = std::env::temp_dir();
        let result = build_service(ProcessCommandRequest {
            command: "printf '%s|%s' \"$DEVTOPOLOGY_TEST_VALUE\" \"$PWD\"".into(),
            cwd: cwd.to_string_lossy().into_owned(),
            env: HashMap::from([(String::from("DEVTOPOLOGY_TEST_VALUE"), String::from("sample-value"))]),
        }).expect("build command should succeed");
        let expected_cwd = cwd.canonicalize().expect("temp directory should resolve");
        assert_eq!(result, format!("sample-value|{}", expected_cwd.display()));
    }

    #[test]
    fn start_spawns_a_configured_command() {
        let pid = start_service(ProcessCommandRequest {
            command: "test \"$DEVTOPOLOGY_TEST_VALUE\" = sample-value".into(),
            cwd: std::env::temp_dir().to_string_lossy().into_owned(),
            env: HashMap::from([(String::from("DEVTOPOLOGY_TEST_VALUE"), String::from("sample-value"))]),
        }).expect("start command should spawn");
        assert!(pid > 0);
    }
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("cmd");
    shell.args(["/C", command]);
    shell
}
