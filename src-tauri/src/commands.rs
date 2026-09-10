//! Tauri invoke commands exposed to the Svelte widget.

use crate::scanner::{self, Snapshot};

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
