# devtopology design

`devtopology` answers one question about the local machine: **what is running right now?** The
v0.1 product is an always-on-top desktop widget (macOS + Windows) that shows the developer
services currently listening on local ports — process name, port, PID, status — refreshed live.

The full product concept (connection detection, dependency graphs, diagnostics) lives in the
workspace memory repo (`catchmeif404memory/projects/upcomming project/DevTopology_Design_Document.md`).
This document is the implementation design for v0.1 and the roadmap after it.

## Decision trail

The concept document proposed a Go agent with a React web dashboard. Implementation design
narrowed it in three steps, each a deliberate scope cut:

1. **Pure viewer first.** v0.1 shows running processes and ports. Project mapping, framework
   detection, connections, and diagnostics are later phases (see Roadmap).
2. **Widget, not web.** The view must live on the monitor like a widget, not in a browser tab.
   A frameless always-on-top Tauri window replaces the HTTP server + web dashboard entirely.
3. **Rust scanner.** Tauri has no Node runtime, so the scanner lives in Rust (`sysinfo` +
   `netstat2` crates). This makes the widget fully self-contained — no Node on the user's
   machine, no shell-command output parsing — and cross-platform from one codebase.

Consequences of step 3: TypeScript exists only in the UI layer (Svelte). The "zero-dependency
TS CLI" idea from earlier drafts is dropped; if a CLI surface is ever needed, it becomes a Rust
subcommand of the same core.

## Stack

| Layer | Choice | Notes |
|---|---|---|
| Widget shell | Tauri 2 | frameless, transparent, always-on-top window |
| UI | Svelte 5 + TypeScript | Vite build; follow the workspace Svelte coding rules |
| Scanner | Rust | `sysinfo` (process list) + `netstat2` (listen sockets, port↔pid) |
| Serialization | `serde` / `serde_json` | one shared JSON shape across the IPC boundary |
| Window state | `tauri-plugin-window-state` | remembers widget position/size across launches |

Development requires the Rust toolchain on both build machines (mac here, Windows machine for
the win target). Users install prebuilt bundles; they need nothing else.

## Architecture

```text
┌─────────────────────────────────────────────┐
│ Tauri process (single binary + webview)     │
│                                             │
│  ┌──────────────┐  invoke('get_services')   │
│  │ Svelte UI    │ ────────────────────────► │
│  │ widget cards │ ◄──────────────────────── │
│  └──────────────┘   Snapshot (JSON, IPC)    │
│                            │                │
│                   ┌────────▼─────────┐      │
│                   │ scanner (Rust)   │      │
│                   │  ├─ sysinfo      │ processes (name, pid)
│                   │  ├─ netstat2     │ listen sockets (port ↔ pid)
│                   │  └─ registry     │ dev-relevance filter
│                   └──────────────────┘      │
└─────────────────────────────────────────────┘
```

- The UI polls `get_services` every 3 seconds (Tauri `invoke`). No events, no daemon, no
  background threads beyond the scan itself; a scan is a single `sysinfo` refresh + socket table
  read.
- No network egress, no file contents are read, nothing leaves the machine.

## Data model

One JSON shape, defined in Rust and mirrored in `src/lib/types.ts`:

```ts
interface Service {
  pid: number;
  process: string;      // "node", "java", "postgres", ...
  ports: number[];      // all TCP ports this pid is listening on
}

interface Snapshot {
  services: Service[];  // filtered to dev-relevant, sorted by port
  generatedAt: string;  // ISO 8601
  host: { os: 'macos' | 'windows'; hostname: string };
}
```

v0.1 has no `status` field beyond presence: a service appears because it is listening, so it is
RUNNING by construction. `CONFIGURED`/`CONFLICT` states arrive with the diagnostics phase.

## Dev-relevance filter

`sysinfo` sees every process; the widget must not. The rule lives in one Rust module
(`scanner/registry.rs`), the only place raw port/process-name constants exist:

- **Allowlist by process name** — `node`, `java`, `python`, `ruby`, `go`, `postgres`,
  `redis-server`, `mongod`, `kafka`, `elasticsearch`, `rabbitmq`, `dotnet`, `php-fpm`, ...
- **Allowlist by known dev port** — 3000, 5173, 5174, 8000, 8080, 8081, 5432, 3306, 6379, 9092,
  27017, 9200, 5672, ...
- A pid is shown when it matches either list; a listening pid matching neither is dropped.
- `include_all_listeners` (compile-time const in v0.1) widens this to "every TCP listener" for
  debugging the filter itself.

The filter is a pure function over `(process table, socket table)`, unit-tested with synthetic
input — the sysinfo/netstat2 calls stay at the edge.

## Widget shell spec (Tauri window)

- Frameless, transparent, always-on-top, skip-taskbar; background opacity ~0.9.
- The header row is the drag region. It shows the app name, the live service count, and a close
  button. Right-click opens a small context menu (v0.1: quit; language toggle if implemented).
- Position and size persist across launches via `tauri-plugin-window-state`.
- Click-through is intentionally out of v0.1 (it makes the close button unusable); revisit if
  asked for.
- Launch is manual in v0.1. Autostart (`tauri-plugin-autostart`) and tray mode are roadmap items.

## UI spec (Svelte)

- A single column of service cards sorted by port: process name, `:port` list, PID in muted
  text, a green status dot.
- Header: `DevTopology` + count of running services + relative "updated Ns ago" timestamp.
- Empty state: "no dev services detected" (both locales).
- UI strings come from one `i18n.ts` module with `en`/`ko` maps — bilingual from day one per the
  workspace rule; the widget surface is small enough that this is one file, not a framework.
- Styling targets the case-file identity later; v0.1 keeps a neutral dark translucent card
  look. No graph library, no router, no state library — Svelte runes and one polling `setInterval`.

## Repository layout

```text
devtopology/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs              # tauri builder, window config, command registration
│   │   ├── commands.rs         # get_services invoke handler
│   │   └── scanner/
│   │       ├── mod.rs          # Service, Snapshot; scan() assembly
│   │       ├── registry.rs     # the only file with port/name constants
│   │       └── filter.rs       # pure filtering logic (unit-tested)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.svelte
│   ├── main.ts
│   └── lib/
│       ├── api.ts              # invoke('get_services') + 3s poll
│       ├── types.ts            # mirror of the Rust Snapshot
│       ├── i18n.ts             # en / ko strings
│       └── ServiceCard.svelte
├── docs/DESIGN.md
├── package.json                # @tauri-apps/cli, vite, svelte, vitest
└── README.md
```

## Testing

- Rust: `cargo test` covers the filter/registry logic with synthetic process/socket inputs; the
  sysinfo/netstat2 boundary is kept thin and unmocked.
- Svelte: `vitest` + `@testing-library/svelte` render `ServiceCard`/`App` against fixture
  snapshots (including empty and many-services cases).
- `npm run tauri dev` for local verification; `npm run tauri build` for release bundles.

## Release policy

- Conventional Commits; `main` is always buildable.
- CI (GitHub Actions): `cargo test` + frontend check on every push/PR; `tauri-action` builds
  macOS (dmg) and Windows (msi/nsis) bundles only on `vX.Y.Z` tags. Pushes to `main` never ship.
- Workspace git identity (`catchmeif404`) and the tag-triggered release policy apply as
  everywhere else in this workspace.

## Roadmap

- **v0.2 — Which project is this?** Map pid → working directory (mac: `proc_pidpath`/lsof cwd;
  Windows: harder, best-effort) and show the repo folder/branch on each card. Menu-bar tray mode
  with the live count.
- **v0.3 — Connections.** Frontend→backend URL detection from `.env`/config files (the concept
  document's §10–§12 pipeline), with evidence and confidence. Secret redaction lands here, at
  parse time, before any value enters the model.
- **v0.4 — Diagnostics.** Port conflicts, broken connections (frontend expects a port nobody
  listens on), local→remote/prod host warnings.
- **v0.5 — Agent context.** `get_services` JSON exposed as a CLI subcommand / MCP tool so AI
  coding agents can read the local topology.
- **v0.6 — Suite integration.** Shared surface with `gitguard` / Workbound ("what is running" /
  "who is committing" / "what can change").

The concept document's remaining ideas (request flow tracing, log viewing, process control) stay
parked there until the widget proves itself.
