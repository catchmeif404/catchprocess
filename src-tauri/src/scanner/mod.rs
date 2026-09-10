//! Collects live process and listening-socket data from the OS and reduces it to the widget
//! snapshot. All OS access lives in this module (sysinfo + netstat2); filtering is in `filter`.

pub mod filter;
pub mod registry;

use std::time::{SystemTime, UNIX_EPOCH};

use netstat2::{
    AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info,
};
use serde::Serialize;
use sysinfo::{ProcessesToUpdate, System};

use filter::{ListenRow, ProcessRow, Service};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    /// `std::env::consts::OS` — "macos", "windows", "linux".
    pub os: &'static str,
    pub hostname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub services: Vec<Service>,
    /// Unix epoch milliseconds; the UI renders the relative "updated Ns ago" itself.
    pub generated_at: u64,
    pub host: HostInfo,
}

fn collect_processes() -> Vec<ProcessRow> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .processes()
        .iter()
        .map(|(pid, process)| ProcessRow {
            pid: pid.as_u32(),
            name: process
                .name()
                .to_string_lossy()
                .trim_end_matches(".exe")
                .to_lowercase(),
        })
        .collect()
}

fn collect_listeners() -> Vec<ListenRow> {
    let families = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let protocols = ProtocolFlags::TCP;
    let sockets = match get_sockets_info(families, protocols) {
        Ok(sockets) => sockets,
        // A failed socket-table read degrades to an empty list, not a widget error.
        Err(_) => return Vec::new(),
    };
    sockets
        .into_iter()
        .filter_map(|socket| match socket.protocol_socket_info {
            // v0.1은 TCP 리슨 소켓만 관심 대상 (UDP는 서비스 탐지에 노이즈만 추가).
            ProtocolSocketInfo::Tcp(tcp) if tcp.state == TcpState::Listen => Some(ListenRow {
                port: tcp.local_port,
                pid: socket.associated_pids.first().copied(),
            }),
            _ => None,
        })
        .collect()
}

fn host_info() -> HostInfo {
    HostInfo {
        os: std::env::consts::OS,
        hostname: System::host_name().unwrap_or_else(|| "localhost".to_string()),
    }
}

fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        // Clock before the epoch is unrecoverable; 0 marks "unknown".
        .unwrap_or(0)
}

/// Take one full snapshot of the dev-relevant local topology.
pub fn scan() -> Snapshot {
    let processes = collect_processes();
    let listeners = collect_listeners();
    Snapshot {
        services: filter::filter_services(
            &processes,
            &listeners,
            registry::INCLUDE_ALL_LISTENERS,
        ),
        generated_at: epoch_millis(),
        host: host_info(),
    }
}
