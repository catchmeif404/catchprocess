//! Tauri invoke commands exposed to the Svelte widget.

use crate::scanner::{self, Snapshot};

/// One live scan of dev-relevant services; called on a 3s poll from the UI.
#[tauri::command]
pub fn get_services() -> Snapshot {
    scanner::scan()
}
