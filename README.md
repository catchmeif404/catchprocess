<div align="center">

# catchprocess

**Exhibit C. A live case file for everything running on your machine.**

[한국어](README.ko.md) · **English**

`catchmeif404`

</div>

---

catchprocess is a small desktop widget for developers who have lost track of their own local
services. It watches listening TCP ports, groups processes by project, and turns the evidence into
a board you can inspect and control.

No browser tab. No server. No telemetry. Just one answer: **what is running right now?**

## What it does

- **Live service discovery** — scans listening TCP ports every three seconds and shows the process,
  ports, PID, project, and Git branch when available.
- **Project boards** — organize services into named boards and arrange them as a topology map.
- **Connection evidence** — display observed local and outbound TCP connections separately from
  source-code API targets and database configuration candidates.
- **Start and stop controls** — launch configured services, start all offline services on a board,
  or stop the running services on that board.
- **Two views** — switch between a topology map and grouped service cards.
- **Menu-bar mode** — hide the widget to the system tray/menu bar and bring it back when needed.
- **Bilingual UI** — switch between English and Korean inside the widget.

## Install

Download the installer for your platform from the [latest release](https://github.com/catchmeif404/catchprocess/releases/latest).

The first macOS launch may require opening the app from Finder because the current build is not
Apple-signed or notarized yet.

## Run from source

Requires Node.js 20 or later and the Rust toolchain.

```bash
npm ci
npm run tauri dev
```

To create a local installer:

```bash
npm run tauri build
```

The resulting `.dmg` and `.app` files are written under `src-tauri/target/release/bundle/`.

## Release

Production builds are created by GitHub Actions from version tags. A tag creates a draft release
with macOS Apple Silicon, macOS Intel, and Windows installers attached.

```bash
git tag v0.1.0
git push origin v0.1.0
```

Publish the draft release after checking the generated assets. The workflow is in
`.github/workflows/release.yml`.

## How it works

The Svelte UI invokes one Tauri command, `get_services`, every three seconds. The Rust scanner uses
`sysinfo` for the process table and `netstat2` for TCP sockets. A central registry filters out
system noise while retaining common development processes and ports.

Project context is derived from the process working directory and Git metadata. The scanner does
not read environment files or process environment values. Connection rows are observations of
current sockets, not claims that every configured dependency is live.

## Stack

| Area | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| UI | Svelte 5, TypeScript, Vite |
| Scanner | Rust, `sysinfo`, `netstat2` |
| Persistence | Local JSON files and window state |
| Platforms | macOS and Windows |

## Development checks

```bash
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

## Privacy

Scanning runs locally. catchprocess does not send process, path, port, or connection data to a
remote service. Source and configuration files may be read locally to identify API targets and
database candidates; environment values are intentionally excluded.

## Docs

- [`api.md`](api.md) — Tauri IPC API
- [`docs/DESIGN.md`](docs/DESIGN.md) — implementation design and roadmap

## License

MIT — see [`LICENSE`](LICENSE).

<div align="center">

Built by `catchmeif404` — building things nobody asked for.

</div>
