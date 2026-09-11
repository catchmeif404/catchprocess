//! Pure dev-relevance filtering over process and listening-socket rows.
//!
//! No OS access happens here: the scanner collects [`ProcessRow`] / [`ListenRow`] inputs at the
//! edge (sysinfo / netstat2) and this module decides what becomes a widget card.

use std::net::{IpAddr, Ipv4Addr};

use serde::{Deserialize, Serialize};

use super::project::{ApiTarget, DatabaseTarget, ProjectInfo};
use super::registry::{KNOWN_DEV_PORTS, KNOWN_PROCESS_NAMES};

/// One process from the OS process table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRow {
    pub pid: u32,
    /// Lowercased executable name, `.exe` suffix stripped (Windows).
    pub name: String,
}

/// One TCP listen socket from the OS socket table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListenRow {
    pub port: u16,
    /// Owning pid; `None` when the OS does not attribute the socket to a process.
    pub pid: Option<u32>,
}

/// One established TCP connection collected from the OS socket table.
///
/// The scanner converts `netstat2` rows to this small, testable shape before applying the
/// mapping rules below. A row may be associated with more than one pid on some platforms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionRow {
    pub pids: Vec<u32>,
    pub local_port: u16,
    pub remote_addr: IpAddr,
    pub remote_port: u16,
}

/// A service card in the widget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub pid: u32,
    pub process: String,
    /// All listening ports of this pid, deduplicated and sorted ascending.
    pub ports: Vec<u16>,
    /// Full command line, capped at 200 chars; omitted when unavailable.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub command: String,
    /// Project derived from the process cwd (git repo name/branch or folder name);
    /// omitted when the cwd could not be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectInfo>,
    /// Outbound TCP connections observed for this process.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<Connection>,
    /// API endpoints discovered from this process's working directory.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub api_targets: Vec<ApiTarget>,
    /// Database targets discovered from this process's working directory.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub database_targets: Vec<DatabaseTarget>,
}

/// A connection from a displayed service to a local listener or external endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub target: String,
    pub port: u16,
    pub local: bool,
}

/// Some macOS socket APIs expose IPv4 loopback as an IPv6-compatible address such as
/// `::7f00:1` instead of `127.0.0.1`. Treat both forms as local loopback.
fn is_loopback_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => address.is_loopback(),
        IpAddr::V6(address) => {
            if address.is_loopback() {
                return true;
            }
            let segments = address.segments();
            let mapped = segments[..6].iter().all(|segment| *segment == 0)
                || (segments[..5].iter().all(|segment| *segment == 0) && segments[5] == 0xffff);
            mapped && Ipv4Addr::from(((segments[6] as u32) << 16) | segments[7] as u32).is_loopback()
        }
    }
}

fn is_known_process(name: &str) -> bool {
    // Defense in depth: compare lowercased with a Windows `.exe` suffix stripped, even though
    // the scanner is expected to normalize the row name when building it.
    let lowered = name.to_lowercase();
    let bare = lowered.strip_suffix(".exe").unwrap_or(&lowered);
    KNOWN_PROCESS_NAMES.contains(&bare)
}

fn is_known_port(port: u16) -> bool {
    KNOWN_DEV_PORTS.contains(&port)
}

/// Reduce process and socket rows into the deduplicated, port-sorted service list.
///
/// A pid becomes a service when `include_all` is set, or its name is known, or it listens on a
/// known dev port. Listeners without an owning pid are dropped (the card has nothing to show).
pub fn filter_services(
    processes: &[ProcessRow],
    listeners: &[ListenRow],
    include_all: bool,
) -> Vec<Service> {
    let names: std::collections::HashMap<u32, &str> =
        processes.iter().map(|p| (p.pid, p.name.as_str())).collect();

    // pid -> ports, preserving first-seen order before the final sort.
    let mut by_pid: std::collections::BTreeMap<u32, Vec<u16>> = std::collections::BTreeMap::new();
    for row in listeners {
        let Some(pid) = row.pid else { continue };
        let known = include_all || is_known_port(row.port) || {
            names.get(&pid).is_some_and(|name| is_known_process(name))
        };
        if known {
            let ports = by_pid.entry(pid).or_default();
            if !ports.contains(&row.port) {
                ports.push(row.port);
            }
        }
    }

    let mut services: Vec<Service> = by_pid
        .into_iter()
        .map(|(pid, mut ports)| {
            ports.sort_unstable();
            let name = names
                .get(&pid)
                .map(|n| (*n).to_string())
                .unwrap_or_else(|| "unknown".to_string());
            Service {
                pid,
                process: name,
                ports,
                command: String::new(),
                project: None,
                connections: Vec::new(),
                api_targets: Vec::new(),
                database_targets: Vec::new(),
            }
        })
        .collect();
    services.sort_by(|a, b| a.ports[0].cmp(&b.ports[0]).then(a.pid.cmp(&b.pid)));
    services
}

/// Attach observed outbound connections to the displayed services.
///
/// A connection is considered local only when its remote address is loopback and its remote port
/// belongs to a displayed listener. Port equality alone is not enough: an external endpoint can
/// use the same port number as a local service. Server-side sockets are skipped when their local
/// port is already one of the service's listeners.
pub fn attach_connections(
    processes: &[ProcessRow],
    listeners: &[ListenRow],
    rows: &[ConnectionRow],
    services: &mut [Service],
) {
    let names: std::collections::HashMap<u32, &str> = processes
        .iter()
        .map(|process| (process.pid, process.name.as_str()))
        .collect();
    let local_targets: std::collections::HashMap<u16, String> = listeners
        .iter()
        .filter_map(|listener| {
            listener.pid.and_then(|pid| {
                names
                    .get(&pid)
                    .map(|name| (listener.port, (*name).to_string()))
            })
        })
        .collect();

    for row in rows {
        let loopback = is_loopback_address(row.remote_addr);
        // A loopback connection to an unknown port is commonly a browser/dev-server internal
        // socket. It is not a useful service edge, so do not surface ephemeral port noise.
        if loopback && !local_targets.contains_key(&row.remote_port) {
            continue;
        }
        let local = loopback;
        let target = if local {
            local_targets
                .get(&row.remote_port)
                .cloned()
                .expect("local target was checked above")
        } else {
            row.remote_addr.to_string()
        };

        for pid in &row.pids {
            let Some(service) = services.iter_mut().find(|service| service.pid == *pid) else {
                continue;
            };
            if service.ports.contains(&row.local_port) {
                continue;
            }
            if !service
                .connections
                .iter()
                .any(|connection| connection.target == target && connection.port == row.remote_port)
            {
                service.connections.push(Connection {
                    target: target.clone(),
                    port: row.remote_port,
                    local,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, name: &str) -> ProcessRow {
        ProcessRow {
            pid,
            name: name.to_string(),
        }
    }

    fn listen(port: u16, pid: Option<u32>) -> ListenRow {
        ListenRow { port, pid }
    }

    fn connection(
        pids: &[u32],
        local_port: u16,
        remote_addr: &str,
        remote_port: u16,
    ) -> ConnectionRow {
        ConnectionRow {
            pids: pids.to_vec(),
            local_port,
            remote_addr: remote_addr.parse().expect("valid test IP"),
            remote_port,
        }
    }

    #[test]
    fn known_process_name_is_included() {
        let services = filter_services(
            &[proc(1, "node"), proc(2, "chrome")],
            &[listen(5173, Some(1)), listen(9001, Some(2))],
            false,
        );
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].process, "node");
        assert_eq!(services[0].ports, vec![5173]);
    }

    #[test]
    fn unknown_process_on_known_port_is_included() {
        let services = filter_services(
            &[proc(7, "some-custom-server")],
            &[listen(8080, Some(7))],
            false,
        );
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].pid, 7);
        assert_eq!(services[0].ports, vec![8080]);
    }

    #[test]
    fn unknown_process_on_unknown_port_is_dropped() {
        let services = filter_services(&[proc(3, "dropbox")], &[listen(17500, Some(3))], false);
        assert!(services.is_empty());
    }

    #[test]
    fn unattributed_listener_is_dropped() {
        let services = filter_services(&[proc(1, "node")], &[listen(6379, None)], false);
        assert!(services.is_empty());
    }

    #[test]
    fn include_all_widens_filter() {
        let services = filter_services(&[proc(3, "dropbox")], &[listen(17500, Some(3))], true);
        assert_eq!(services.len(), 1);
    }

    #[test]
    fn ports_are_deduplicated_and_sorted_across_families() {
        // IPv4 + IPv6 duplicates of the same port, plus an extra port.
        let services = filter_services(
            &[proc(9, "postgres")],
            &[
                listen(5432, Some(9)),
                listen(5432, Some(9)),
                listen(5433, Some(9)),
            ],
            false,
        );
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].ports, vec![5432, 5433]);
    }

    #[test]
    fn services_are_sorted_by_lowest_port() {
        let services = filter_services(
            &[proc(1, "java"), proc(2, "node")],
            &[listen(8080, Some(1)), listen(3000, Some(2))],
            false,
        );
        assert_eq!(services[0].pid, 2, "port 3000 must come before 8080");
        assert_eq!(services[1].pid, 1);
    }

    #[test]
    fn windows_exe_suffix_is_ignored_for_matching() {
        // Unknown port, so inclusion can only come from the (suffix-stripped) name match.
        let services = filter_services(&[proc(1, "NODE.EXE")], &[listen(9999, Some(1))], false);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].process, "NODE.EXE");
    }

    #[test]
    fn loopback_connection_maps_to_local_listener_process() {
        let processes = [proc(1, "node"), proc(2, "postgres")];
        let listeners = [listen(5173, Some(1)), listen(5432, Some(2))];
        let mut services = filter_services(&processes, &listeners, false);

        attach_connections(
            &processes,
            &listeners,
            &[connection(&[1], 41000, "127.0.0.1", 5432)],
            &mut services,
        );

        assert_eq!(
            services[0].connections,
            vec![Connection {
                target: "postgres".to_string(),
                port: 5432,
                local: true,
            }]
        );
    }

    #[test]
    fn external_endpoint_with_local_port_number_stays_external() {
        let processes = [proc(1, "node"), proc(2, "postgres")];
        let listeners = [listen(5173, Some(1)), listen(5432, Some(2))];
        let mut services = filter_services(&processes, &listeners, false);

        attach_connections(
            &processes,
            &listeners,
            &[connection(&[1], 41000, "203.0.113.10", 5432)],
            &mut services,
        );

        assert_eq!(
            services[0].connections,
            vec![Connection {
                target: "203.0.113.10".to_string(),
                port: 5432,
                local: false,
            }]
        );
    }

    #[test]
    fn ipv6_loopback_also_maps_to_local_listener() {
        let processes = [proc(1, "node"), proc(2, "redis-server")];
        let listeners = [listen(5173, Some(1)), listen(6379, Some(2))];
        let mut services = filter_services(&processes, &listeners, false);

        attach_connections(
            &processes,
            &listeners,
            &[connection(&[1], 41000, "::1", 6379)],
            &mut services,
        );

        assert_eq!(services[0].connections[0].target, "redis-server");
        assert!(services[0].connections[0].local);
    }

    #[test]
    fn duplicate_connections_are_collapsed_and_server_socket_is_skipped() {
        let processes = [proc(1, "node"), proc(2, "postgres")];
        let listeners = [listen(5173, Some(1)), listen(5432, Some(2))];
        let mut services = filter_services(&processes, &listeners, false);

        attach_connections(
            &processes,
            &listeners,
            &[
                connection(&[1], 41000, "127.0.0.1", 5432),
                connection(&[1], 41000, "127.0.0.1", 5432),
                connection(&[2], 5432, "127.0.0.1", 41000),
            ],
            &mut services,
        );

        assert_eq!(services[0].connections.len(), 1);
        assert!(services[1].connections.is_empty());
    }

    #[test]
    fn macos_ipv4_compat_loopback_maps_to_local_listener() {
        assert!(is_loopback_address("::7f00:1".parse().expect("valid IPv6")));
        let processes = [proc(1, "java"), proc(2, "postgres")];
        let listeners = [listen(8080, Some(1)), listen(5432, Some(2))];
        let mut services = filter_services(&processes, &listeners, false);

        attach_connections(
            &processes,
            &listeners,
            &[connection(&[1], 41000, "::7f00:1", 5432)],
            &mut services,
        );

        let java = services.iter().find(|service| service.pid == 1).expect("java service");
        assert_eq!(java.connections[0].target, "postgres");
        assert!(java.connections[0].local);
    }

    #[test]
    fn unknown_loopback_port_is_ignored() {
        let processes = [proc(1, "node")];
        let listeners = [listen(3000, Some(1))];
        let mut services = filter_services(&processes, &listeners, false);

        attach_connections(
            &processes,
            &listeners,
            &[connection(&[1], 41000, "127.0.0.1", 59803)],
            &mut services,
        );

        assert!(services[0].connections.is_empty());
    }

    #[test]
    fn unknown_process_ids_are_ignored() {
        let processes = [proc(1, "node")];
        let listeners = [listen(5173, Some(1))];
        let mut services = filter_services(&processes, &listeners, false);

        attach_connections(
            &processes,
            &listeners,
            &[connection(&[999], 41000, "127.0.0.1", 5432)],
            &mut services,
        );

        assert!(services[0].connections.is_empty());
    }
}
