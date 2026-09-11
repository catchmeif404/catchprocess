//! Collects live process and listening-socket data from the OS and reduces it to the widget
//! snapshot. All OS access lives in this module (sysinfo + netstat2); filtering is in `filter`
//! and project derivation in `project`.

pub mod filter;
pub mod project;
pub mod registry;
pub mod stop;

use std::time::{SystemTime, UNIX_EPOCH};

use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use filter::{ConnectionRow, ListenRow, ProcessRow, Service};

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

fn process_rows(system: &System) -> Vec<ProcessRow> {
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

fn collect_tcp_sockets() -> Vec<netstat2::SocketInfo> {
    let families = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let protocols = ProtocolFlags::TCP;
    get_sockets_info(families, protocols).unwrap_or_default()
}

fn collect_listeners(sockets: &[netstat2::SocketInfo]) -> Vec<ListenRow> {
    sockets
        .iter()
        .filter_map(|socket| match &socket.protocol_socket_info {
            // v0.1은 TCP 리슨 소켓만 관심 대상 (UDP는 서비스 탐지에 노이즈만 추가).
            ProtocolSocketInfo::Tcp(tcp) if tcp.state == TcpState::Listen => Some(ListenRow {
                port: tcp.local_port,
                pid: socket.associated_pids.first().copied(),
            }),
            _ => None,
        })
        .collect()
}

/// Convert established OS sockets to the small, pure mapping input used by `filter`.
fn collect_connections(
    sockets: &[netstat2::SocketInfo],
    processes: &[ProcessRow],
    listeners: &[ListenRow],
    services: &mut [Service],
) {
    let rows: Vec<ConnectionRow> = sockets
        .iter()
        .filter_map(|socket| match &socket.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp) if tcp.state == TcpState::Established => {
                Some(ConnectionRow {
                    pids: socket.associated_pids.clone(),
                    local_port: tcp.local_port,
                    remote_addr: tcp.remote_addr,
                    remote_port: tcp.remote_port,
                })
            }
            _ => None,
        })
        .collect();
    filter::attach_connections(processes, listeners, &rows, services);
}

/// 명령줄 배열을 카드용 한 줄 문자열로 합친다. 과도한 JSON을 막기 위해 200자로 제한.
fn summarize_command(args: &[std::ffi::OsString]) -> String {
    let joined = args
        .iter()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.chars().count() > 200 {
        let truncated: String = joined.chars().take(200).collect();
        format!("{truncated}…")
    } else {
        joined
    }
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
    let mut system = System::new();
    // 1단계: 전체 프로세스 테이블(이름) — dev 관련성 필터링용. 기본 리프레시는
    // cwd/cmd를 읽지 않는다(ProcessRefreshKind 기본값이 전부 never라 sysinfo가 의도한 것).
    system.refresh_processes(ProcessesToUpdate::All, true);
    let processes = process_rows(&system);
    let sockets = collect_tcp_sockets();
    let listeners = collect_listeners(&sockets);
    let mut services =
        filter::filter_services(&processes, &listeners, registry::INCLUDE_ALL_LISTENERS);

    // 2단계: 필터를 통과한 서비스 pid에 대해서만 cwd/명령줄을 추가로 읽는다.
    // 전체 프로세스에 대해 cwd(proc_pidinfo)를 읽으면 스캔이 불필요하게 무거워진다.
    let service_pids: Vec<Pid> = services.iter().map(|s| Pid::from_u32(s.pid)).collect();
    if !service_pids.is_empty() {
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&service_pids),
            true,
            ProcessRefreshKind::nothing()
                .with_cwd(UpdateKind::Always)
                .with_cmd(UpdateKind::Always),
        );
    }

    // 카드 보강: 명령줄 + cwd 기반 프로젝트 정보. cwd 읽기 실패는 그 필드만 생략된다.
    for service in &mut services {
        if let Some(process) = system.process(Pid::from_u32(service.pid)) {
            service.command = summarize_command(process.cmd());
            let command = process.cmd().iter().map(|arg| arg.to_string_lossy()).collect::<Vec<_>>().join(" ");
            service.framework = if command.contains("next-server") || command.contains("next/dist/") {
                Some("nextjs".into())
            } else if command.contains("spring-boot") || command.contains("org.springframework.boot") || command.contains("springframework") {
                Some("spring".into())
            } else { None };
            service.project = project::resolve(process.cwd());
        }
    }

    for service in &mut services {
        if let Some(process) = system.process(Pid::from_u32(service.pid)) {
            if let Some(cwd) = process.cwd() {
                service.api_targets = project::discover_api_targets(cwd);
                service.database_targets = project::discover_database_targets(cwd);
            }
        }
    }

    // Gradle daemons are Java processes with listening control ports, but they are build
    // infrastructure rather than services owned by a project. Hide them after cwd enrichment
    // so ordinary Java application servers remain visible.
    services.retain(|service| {
        !service.project.as_ref().is_some_and(|project| {
            project::is_gradle_daemon_path(std::path::Path::new(&project.path))
        })
    });
    collect_connections(&sockets, &processes, &listeners, &mut services);

    Snapshot {
        services,
        generated_at: epoch_millis(),
        host: host_info(),
    }
}

#[cfg(test)]
mod tests {
    use super::summarize_command;

    #[test]
    fn summarize_command_joins_args() {
        assert_eq!(
            summarize_command(&["node".into(), "vite".into(), "--port".into(), "5173".into()]),
            "node vite --port 5173"
        );
    }

    #[test]
    fn summarize_command_truncates_long_lines() {
        let long = "a".repeat(500);
        let summarized = summarize_command(&[long.clone().into()]);
        assert_eq!(summarized.chars().count(), 201); // 200자 + 생략 기호
        assert!(summarized.ends_with('…'));
        assert_ne!(summarized, long);
    }

    #[test]
    fn summarize_command_handles_empty() {
        assert_eq!(summarize_command(&[]), "");
    }

    #[test]
    fn summarize_command_keeps_multibyte_chars_intact() {
        // 200자 제한이 유니코드를 중간에 자르지 않는지 확인.
        let hangul = "가".repeat(250);
        let summarized = summarize_command(&[hangul.into()]);
        assert_eq!(summarized.chars().count(), 201);
    }
}
