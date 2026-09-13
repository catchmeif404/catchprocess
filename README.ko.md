<div align="center">

# catchprocess

**전시품 C. 당신의 머신에서 돌아가는 모든 것에 대한 실시간 사건 기록.**

**한국어** · [English](README.md)

`catchmeif404`

</div>

---

catchprocess는 로컬에서 무엇이 실행 중인지 놓쳐버린 개발자를 위한 작은 데스크톱 위젯이다.
열려 있는 TCP 포트를 감시하고, 프로세스를 프로젝트별로 묶고, 확인한 증거를 직접 살펴보고
제어할 수 있는 보드로 보여준다.

브라우저 탭도, 별도 서버도, 텔레메트리도 없다. 질문은 하나뿐이다. **지금 무엇이 실행 중인가?**

## 기능

- **실시간 서비스 탐지** — 3초마다 TCP 리슨 포트를 검사하고 프로세스, 포트, PID, 프로젝트,
  Git 브랜치를 표시한다.
- **프로젝트 보드** — 서비스를 이름 있는 보드로 정리하고 토폴로지 맵으로 배치한다.
- **연결 증거** — 실제 관찰된 로컬·외부 TCP 연결을 소스 코드의 API 대상이나 DB 설정 후보와
  구분해서 보여준다.
- **시작·중지 제어** — 설정한 서비스를 실행하고, 보드의 꺼진 서비스를 일괄 시작하거나,
  보드에서 실행 중인 서비스를 일괄 중지한다.
- **두 가지 보기** — 토폴로지 맵과 프로젝트별 서비스 카드 사이를 전환한다.
- **메뉴바 모드** — 위젯을 시스템 트레이/메뉴바로 숨겼다가 필요할 때 다시 연다.
- **한국어·영어 UI** — 위젯 안에서 언어를 전환한다.

## 설치

[최신 릴리즈](https://github.com/catchmeif404/catchprocess/releases/latest)에서 운영체제에 맞는
설치 파일을 받는다.

현재 macOS 빌드는 Apple 서명·공증이 되어 있지 않으므로, 첫 실행 시 Finder에서 앱을 열고
Gatekeeper 확인을 진행해야 할 수 있다.

## 소스에서 실행

Node.js 20 이상과 Rust 툴체인이 필요하다.

```bash
npm ci
npm run tauri dev
```

로컬 설치 파일을 만들려면:

```bash
npm run tauri build
```

생성된 `.dmg`와 `.app`은 `src-tauri/target/release/bundle/` 아래에 저장된다.

## 배포

프로덕션 빌드는 버전 태그를 기준으로 GitHub Actions에서 만든다. 태그를 push하면 macOS Apple
Silicon, macOS Intel, Windows 설치 파일이 첨부된 Draft Release가 생성된다.

```bash
git tag v0.1.0
git push origin v0.1.0
```

생성된 파일을 확인한 뒤 Draft Release를 Publish하면 된다. workflow는
`.github/workflows/release.yml`에 있다.

## 동작 방식

Svelte UI가 3초마다 Tauri 커맨드 `get_services`를 호출한다. Rust 스캐너는 프로세스 목록에
`sysinfo`, TCP 소켓에 `netstat2`를 사용한다. 개발 프로세스와 포트만 남기도록 중앙 registry에서
시스템 노이즈를 걸러낸다.

프로세스의 작업 디렉터리와 Git 메타데이터에서 프로젝트 정보를 가져온다. 환경 파일이나
프로세스 환경변수 값은 읽지 않는다. 연결 정보는 현재 소켓에서 관찰한 증거이며, 설정된 모든
의존성이 실제로 살아 있다는 뜻은 아니다.

## 기술 스택

| 영역 | 기술 |
|---|---|
| 데스크톱 셸 | Tauri 2 |
| UI | Svelte 5, TypeScript, Vite |
| 스캐너 | Rust, `sysinfo`, `netstat2` |
| 저장 | 로컬 JSON 파일 및 창 상태 저장 |
| 플랫폼 | macOS, Windows |

## 개발 검사

```bash
npm run check
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

## 개인정보 보호

모든 스캔은 로컬에서 수행된다. 프로세스, 경로, 포트, 연결 정보는 원격 서비스로 전송되지
않는다. API 대상과 DB 후보를 찾기 위해 소스·설정 파일을 로컬에서 읽을 수 있지만, 환경변수
값은 의도적으로 제외한다.

## 로드맵

- HTTP 서비스 상태 확인과 포트 충돌 진단
- 프론트엔드가 꺼진 백엔드를 기대할 때 원인 표시
- 로그인 시 자동 시작
- 코딩 에이전트용 CLI/MCP 출력

## 문서

- [`api.md`](api.md) — Tauri IPC API
- [`docs/DESIGN.md`](docs/DESIGN.md) — 구현 설계와 로드맵

## 라이선스

MIT — [`LICENSE`](LICENSE) 참고.

<div align="center">

`catchmeif404`가 만들었다 — 아무도 부탁하지 않은 것들을 만든다.

</div>
