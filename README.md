# DevTopology

> Exhibit C: a live map of everything hiding on your own machine.
> 전시품 C: 당신 머신 위에 숨어 있는 모든 것의 실시간 지도.

An always-on-top desktop widget (macOS / Windows) that shows which development services are
currently running on your machine — process name, port, PID — refreshed every 3 seconds.
No browser tab, no server, no telemetry. A small card that sits on your monitor and answers one
question: **what is running right now?**

모니터 위에 항상 떠 있는 데스크톱 위젯(macOS / Windows)으로, 지금 머신에서 실행 중인 개발
서비스 — 프로세스 이름, 포트, PID — 를 3초마다 갱신해 보여준다. 브라우저 탭도, 서버도,
원격 전송도 없다. 화면 구석에 작은 카드 하나가 하나의 질문에만 답한다: **지금 뭐가 돌아가고 있나?**

```text
┌───────────────────────────┐
│ DevTopology    4 running ×│
│ ● node       :5173  #44121│
│ ● java       :8080  #55231│
│ ● postgres   :5432  #3332 │
│ ● redis      :6379  #8421 │
└───────────────────────────┘
```

## Features / 기능

- Live scan of listening TCP ports, attributed to processes (3s refresh) /
  리슨 중인 TCP 포트를 프로세스별로 묶어 실시간 표시 (3초 갱신)
- Dev-relevance filter: known dev process names and ports; system noise stays out /
  알려진 개발 프로세스명·포트 기반 필터로 시스템 노이즈 제외
- Project context: show the Git repository name and branch from each process working directory /
  프로세스 작업 경로에서 Git 저장소명과 브랜치를 자동으로 표시
- Search services by process or project name /
  프로세스명·프로젝트명으로 서비스 검색
- Connection mapping: show observed outbound TCP edges to local listeners or remote endpoints /
  로컬 리스너 또는 외부 엔드포인트로 향하는 outbound TCP 연결을 표시
- Stop a running service from its card (SIGTERM, then SIGKILL after a short grace period) /
  카드에서 실행 중인 서비스 중지 (SIGTERM 후 짧은 유예 뒤 SIGKILL)
- Frameless, translucent, always-on-top; drag anywhere; position is remembered /
  프레임리스 반투명 항상 위 창. 어디로든 드래그, 위치는 기억됨
- EN / KO UI with an in-widget language toggle /
  위젯 안에서 전환하는 영어·한국어 UI

## Install / 설치

Prebuilt bundles (macOS `.dmg`, Windows `.msi`) are attached to tagged releases.
빌드된 설치 파일(macOS `.dmg`, Windows `.msi`)은 태그 릴리스에 첨부된다.

### Build from source / 소스에서 빌드

Requires Node.js >= 20 and the Rust toolchain. / Node.js 20 이상과 Rust 툴체인 필요.

```bash
npm install
npm run tauri build   # macOS: .app/.dmg · Windows: .msi/.exe
npm run tauri dev     # 개발 모드 실행
```

## How it works / 동작 방식

The Svelte widget calls a single Tauri command, `get_services`, every 3 seconds. The Rust side
reads the OS process table (`sysinfo`) and the TCP socket table (`netstat2`), groups listening
ports by pid, and filters them through an allowlist of known dev process names and ports —
defined in one place, `src-tauri/src/scanner/registry.rs`. For matching services, it reads only
the working directory and command line needed for card context. Git metadata is used to derive
the repository name and branch; package-manager checkouts such as Homebrew and build daemons
such as Gradle are excluded from project detection.
Established outbound TCP sockets are mapped to known local listeners when possible; otherwise
the remote IP address and port are shown. This is live socket evidence, not configuration
analysis, so an idle or short-lived request may not appear.

Svelte 위젯은 단일 Tauri 커맨드 `get_services`를 3초마다 호출한다. Rust 쪽은 OS 프로세스
테이블(`sysinfo`)과 TCP 소켓 테이블(`netstat2`)을 읽어 리슨 포트를 pid별로 묶고, 알려진 개발
프로세스명·포트 allowlist로 걸러낸다. 기준은 `src-tauri/src/scanner/registry.rs` 한 곳에만
정의된다. 매칭된 서비스에 한해 카드에 필요한 작업 경로와 명령줄만 추가로 읽는다. Git
메타데이터로 저장소명과 브랜치를 도출하며, Homebrew 같은 패키지 관리자 checkout과 Gradle
daemon 같은 빌드 보조 프로세스는 프로젝트 감지에서 제외한다.
확립된 outbound TCP 소켓은 가능한 경우 알려진 로컬 리스너와 연결해 표시하고, 그렇지 않으면
원격 IP 주소와 포트를 표시한다. 이는 설정 분석이 아니라 현재 소켓 상태에 대한 관찰이므로
유휴 상태이거나 짧게 끝난 요청은 나타나지 않을 수 있다.

## Privacy / 프라이버시

Everything runs locally. No network egress, no file contents are read, nothing is stored.
모든 분석은 로컬에서만 이뤄진다. 외부 전송 없음, 파일 내용을 읽지 않음, 저장도 없음.

## Docs / 문서

- Implementation design / 구현 설계: [`docs/DESIGN.md`](docs/DESIGN.md)
- IPC API / IPC API 문서: [`api.md`](api.md)
- Roadmap / 로드맵: v0.2 project mapping → v0.3 connections → v0.4 diagnostics
  (v0.2 프로젝트 매핑 → v0.3 연결 탐지 → v0.4 진단)

## Detection notes / 탐지 참고

The widget shows listening TCP processes, not only application servers. Local development tools
such as PostgreSQL, Redis, Python's `http.server`, and Java services may appear when they match
the process or port allowlist. The stop button terminates the selected process, so use it only
when you own that process.

위젯은 애플리케이션 서버만이 아니라 TCP 포트를 리슨하는 개발 관련 프로세스를 표시한다.
PostgreSQL, Redis, Python `http.server`, Java 서비스 등이 프로세스명 또는 포트 allowlist에
해당하면 나타날 수 있다. 중지 버튼은 선택한 프로세스를 종료하므로 직접 실행한 프로세스에만
사용한다.

## License

MIT
