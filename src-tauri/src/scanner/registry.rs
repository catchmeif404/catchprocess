//! The only place where raw port and process-name constants may live.
//!
//! A listening process is considered dev-relevant when its (lowercased, extension-stripped)
//! process name is in [`KNOWN_PROCESS_NAMES`] or any of its listening ports is in
//! [`KNOWN_DEV_PORTS`]. Everything else is assumed to be system/app noise.

/// Well-known development process names.
pub const KNOWN_PROCESS_NAMES: &[&str] = &[
    // runtimes / app servers
    "node",
    "deno",
    "bun",
    "java",
    "python",
    "python3",
    "ruby",
    "go",
    "cargo",
    "dotnet",
    "php",
    // data stores
    "postgres",
    "mysqld",
    "mariadbd",
    "redis-server",
    "mongod",
    "elasticsearch",
    // brokers (kafka runs as `java`; matched by port instead)
    "beam.smp",
    "rabbitmq",
];

/// Well-known development listening ports.
///
/// Deliberately excluded: 5000 and 7000 (macOS AirPlay Receiver binds them and would surface
/// `ControlCenter` as a fake dev service).
pub const KNOWN_DEV_PORTS: &[u16] = &[
    // frontend dev servers
    3000, 3001, 3333, 4000, 4173, 4200, 5173, 5174, 1420,
    // backend servers
    8000, 8080, 8081, 8082, 8888, 9000,
    // data stores
    5432, 3306, 1521, 6379, 27017,
    // brokers / search
    9092, 2181, 9200, 9300, 5672, 15672,
    // localstack
    4566,
];

/// Debug aid: widen the filter to every TCP listener, ignoring the registries above.
pub const INCLUDE_ALL_LISTENERS: bool = false;
