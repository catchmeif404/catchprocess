# devtopology design

`devtopology` answers one question about the local machine: **what is running, and what is it
connected to?** It scans live processes, listening ports, working directories, and project
configuration files, then renders the result as a topology graph. The v0.1 milestone is a thin
end-to-end slice: detect processes and ports, classify them into services, and show them as nodes
in a web dashboard. Connection detection (edges) starts in v0.2.

The product concept and long-term vision live in the workspace memory repo
(`catchmeif404memory/projects/upcomming project/DevTopology_Design_Document.md`). This document is
the implementation design: what is built now, how, and what comes next.

## Stack decisions

- **TypeScript everywhere.** CLI, scanner core, local API, and web dashboard are one language.
  This matches `gitguard` (ESM, TS strict, Node >= 20) and keeps the toolchain small. The concept
  document proposed Go; the trade-off was decided in favor of a single-language npm-distributed
  tool. If process scanning ever needs Go-level performance or a single binary, the scanner
  interface is the seam to swap.
- **Zero runtime dependencies in the core.** Argument parsing, table rendering, and the HTTP
  server are hand-rolled on `node:*` builtins, like gitguard. The `web/` app is the only place
  with a dependency tree (React, React Flow, Vite).
- **macOS first, Linux behind the same interface.** Process and port discovery is isolated behind
  a `PlatformAdapter` so Linux (`ss`, `/proc`) can be added without touching the resolver.
  Windows is out of scope for v0.1.

## Current architecture (v0.1 target)

```text
CLI entrypoint
  └─ commands
      ├─ PlatformAdapter        (process list, listen table, cwd)
      ├─ FrameworkDetector      (marker files -> framework)
      ├─ ServiceResolver        (process + cwd + framework -> service node)
      ├─ InfraClassifier        (known process/port -> infra node)
      ├─ TopologyBuilder        (nodes -> Topology model)
      └─ Renderers
          ├─ table (scan, ports)
          └─ json (--json)
  └─ serve
      ├─ LocalApi (127.0.0.1:4747, GET /api/topology)
      └─ static web/ build (React + React Flow node graph)
```

`src/index.ts` only starts `runCli`. Scanner output parsing belongs in pure functions that take
command output strings, so every parser is unit-testable against recorded fixtures without
spawning processes.

## Repository layout

```text
devtopology/
├── src/
│   ├── index.ts                 # thin entrypoint
│   ├── cli/
│   │   ├── runCli.ts
│   │   └── commands/
│   │       ├── scan.ts
│   │       ├── ports.ts
│   │       └── serve.ts
│   ├── platform/
│   │   ├── types.ts             # PlatformAdapter interface
│   │   ├── macos.ts             # ps / lsof adapters
│   │   ├── linux.ts             # v0.1: stub, throws UnsupportedPlatformError
│   │   └── fixtures/            # recorded command output for tests
│   ├── project/
│   │   └── frameworkDetector.ts
│   ├── resolver/
│   │   ├── serviceResolver.ts
│   │   └── infraClassifier.ts
│   ├── topology/
│   │   ├── model.ts
│   │   └── buildTopology.ts
│   ├── registry/
│   │   └── devPorts.ts          # typed known-port/process registry
│   ├── render/
│   │   └── table.ts
│   ├── server/
│   │   └── httpServer.ts
│   └── types.ts
├── web/
│   ├── src/
│   │   ├── App.tsx
│   │   ├── api/client.ts        # fetch /api/topology, 5s polling
│   │   ├── graph/TopologyCanvas.tsx
│   │   ├── graph/ServiceNode.tsx
│   │   └── i18n/                # en + ko dictionaries from day one
│   ├── package.json
│   └── vite.config.ts
├── docs/DESIGN.md
├── package.json                 # bin: devtopology
├── tsconfig.json
└── vitest.config.ts
```

## Data model

The concept document's model, narrowed to v0.1. `Edge` and `Evidence` are defined now but always
empty; the resolver only produces nodes.

```ts
type NodeType =
  | 'FRONTEND' | 'BACKEND' | 'DATABASE' | 'CACHE'
  | 'MESSAGE_BROKER' | 'EXTERNAL_API' | 'CONTAINER' | 'UNKNOWN';

type RuntimeStatus = 'RUNNING' | 'CONFIGURED' | 'NOT_RUNNING' | 'UNKNOWN' | 'CONFLICT';

type Framework = 'VITE' | 'REACT' | 'NEXT' | 'SPRING_BOOT' | 'NODE' | 'PYTHON' | 'DOCKER_COMPOSE';

interface PortInfo { port: number; address: string; }

interface NodeRuntime {
  pid?: number;              // absent for infra nodes detected by port only
  status: RuntimeStatus;     // v0.1 produces RUNNING or UNKNOWN only
  ports: PortInfo[];
}

interface NodeProject {
  path: string;
  framework?: Framework;
  gitBranch?: string;
}

interface TopologyNode {
  id: string;                // `proc:{pid}` or `infra:{kind}:{port}`
  type: NodeType;
  name: string;              // service name or process name
  runtime: NodeRuntime;
  project?: NodeProject;
}

interface Edge { id: string; source: string; target: string; } // reserved for v0.2

interface Topology {
  nodes: TopologyNode[];
  edges: Edge[];
  generatedAt: string;       // ISO 8601
  host: { os: string; hostname: string };
}
```

Rule that follows the workspace coding guidelines: no dynamic types, no hardcoded port/process
strings outside `registry/devPorts.ts`. That module is the single source of known dev
infrastructure:

```ts
const KNOWN_PORTS = {
  3000: 'NODE', 5173: 'VITE', 8000: 'PYTHON', 8080: 'SPRING_BOOT', 8081: 'SPRING_BOOT',
  5432: 'POSTGRES', 3306: 'MYSQL', 27017: 'MONGODB',
  6379: 'REDIS', 9092: 'KAFKA', 5672: 'RABBITMQ', 9200: 'ELASTICSEARCH',
} as const;

const KNOWN_PROCESS_NAMES = ['postgres', 'redis-server', 'mongod', 'kafka', 'elasticsearch',
  'rabbitmq', 'java', 'node', 'python', 'ruby'] as const;
```

## Scan pipeline

1. **Process list.** `ps -axo pid=,ppid=,comm=,args=` parsed into `ProcessEntry`.
2. **Listen table.** `lsof -nP -iTCP -sTCP:LISTEN` parsed into `port -> pid` entries (v0.1:
   TCP only, IPv4+IPv6 deduplicated).
3. **Working directory.** For each listening pid, `lsof -a -p {pid} -d cwd -Fn` yields the cwd.
   Cached per pid for one scan.
4. **Framework detection.** At the cwd, check marker files in priority order: `next.config.*`
   (NEXT), `vite.config.*` (VITE), `pom.xml`/`build.gradle*` (SPRING_BOOT), `package.json`
   (NODE/REACT), `manage.py`/`pyproject.toml` (PYTHON), `compose.y*ml` (DOCKER_COMPOSE).
   A project with no marker files is skipped, not guessed.
5. **Service resolution.** A listening process becomes a node when any of: known dev process
   name, known dev port, or a detected framework project at its cwd. Everything else is noise
   and filtered.
6. **Infra classification.** Processes matching known infra names/ports become DATABASE / CACHE /
   MESSAGE_BROKER nodes named after the kind (`postgres`, `redis`, ...), independent of cwd.
7. **Topology assembly.** Nodes sorted by type then name; `edges: []`; host metadata attached.
   Port conflicts are detected (two processes claiming one configured port) but only rendered in
   the summary count in v0.1 — the warning panel is a v0.2 surface.

Docker containers are intentionally out of v0.1 (the concept doc puts Docker integration in
v0.4); `CONTAINER` exists in the type so nodes can appear without a model change later.

## CLI surface (v0.1)

```text
devtopology scan            # full report: services table + status summary
devtopology ports           # port-centric table: PORT PID PROCESS SERVICE
devtopology serve [--port]  # 127.0.0.1:4747, serves web/ build + GET /api/topology
```

- `scan` and `ports` accept `--json` for stable machine output (the seam for the later agent
  integration; keep the shape identical to `Topology`).
- Unknown options exit with code 2, like gitguard.
- Output values that never appear: environment variable values, connection strings, tokens.
  v0.1 never reads file contents, only file existence — there is nothing to leak yet, and the
  redaction layer arrives with the v0.2 config parsers before any content is read.

## Local API and web dashboard

- `GET /api/topology` returns the current `Topology` as JSON, re-scanning on request (scans are
  cheap: three subprocess calls; no daemon state to invalidate).
- `GET /api/health` returns `{ ok: true }`.
- The server binds `127.0.0.1` only, and serves the built `web/dist` statically.
- The web app is Vite + React + React Flow. v0.1 renders **nodes only**, laid out in a typed
  auto-layout (infra on the bottom row, services above, grouped by type). Each node card shows
  name, type badge, port(s), status dot, framework, and branch.
- The topology poll refetches every 5 seconds; there is no websocket in v0.1.
- Bilingual from day one (EN/KO dictionary modules with a toggle), per the workspace localization
  rule — the same pattern as `aws-cost-calculator`'s locale support, simplified for a local
  dashboard (no routing, a `LanguageContext`).

## Security and privacy

- All analysis is local; nothing leaves the machine. No telemetry, no outbound requests from the
  core.
- The API binds to loopback only.
- v0.1 reads no file contents. When v0.2 adds `.env`/`application.yml` parsing, values matching
  secret-shaped keys are redacted at parse time, before they ever enter the topology model.

## Testing

- Vitest, colocated `*.test.ts`.
- Scanner parsers are pure functions tested against fixture files under
  `src/platform/fixtures/` — recorded `ps` and `lsof` output from a real macOS session, plus
  synthetic edge cases (empty listen table, pid with no cwd, IPv6 duplicates).
- `frameworkDetector` tested against temporary directory trees.
- Resolver and topology builder tested with synthetic `ProcessEntry`/listen-table inputs — no
  subprocess spawning in unit tests.
- `npm run check` = typecheck + build + test, mirroring the gitguard convention.

## Release policy

- Conventional Commits; `main` is always buildable.
- CI (GitHub Actions): build + test on every push/PR.
- Releases are versioned tags (`vX.Y.Z`) that trigger the npm publish workflow (and later attach
  platform notes). Regular pushes to `main` never publish.

## Later phases (summary)

- **v0.2 — Connections.** `.env` / `.env.local` / `application.yml` / `application.properties`
  parsers, frontend→backend URL matching, `Evidence` attached to edges, confidence levels,
  broken-connection and port-conflict warning panel in the web UI.
- **v0.3 — Dependencies.** Backend→DB/Redis/Kafka edges, environment mismatch detection
  (local process → remote/prod host warning), `dangerous_hosts` config.
- **v0.4 — Docker.** `docker ps` + Compose file analysis; container nodes join the graph.
- **v0.5 — Diagnostics.** `devtopology doctor` as a single exit-coded command combining all
  diagnostics; `.devtopology.yml` project registration and manual annotations.
- **v0.6 — Agent context.** `devtopology context` emitting a compact text summary for AI coding
  agents; MCP server wrapper.
- **v0.7 — Suite integration.** Shared surface with `gitguard` / Workbound (the "who is
  committing / what can change / what is running" trio).
