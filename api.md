# api.md — DevTopology IPC API

DevTopology는 웹 서버 없이 Tauri IPC만 사용한다. 프론트엔드(Svelte)가 호출하는
Rust 커맨드를 아래에 문서화한다.

## `get_services`

- **Method**: Tauri IPC invoke (HTTP 아님)
- **URL**: 없음 (커맨드 이름 `get_services`)
- **Request Headers**: 없음
- **Parameters**: 없음
- **Request Body**: 없음
- **Success Response**: `Snapshot`

```json
{
  "services": [
    {
      "pid": 44121,
      "process": "node",
      "ports": [5173],
      "command": "node vite --port 5173",
      "project": {
        "name": "my-app",
        "path": "/Users/k/dev/my-app",
        "branch": "feature/login"
      }
    },
    { "pid": 3332, "process": "postgres", "ports": [5432] }
  ],
  "generatedAt": 1760000000000,
  "host": { "os": "macos", "hostname": "dev-machine" }
}
```

- `services`: dev 관련성 필터(`src-tauri/src/scanner/registry.rs`의 allowlist)를 통과한
  프로세스만 포함. 포트 오름차순 정렬.
- `services[].command`: 실행 명령줄 요약(최대 200자). 읽지 못하면 필드 생략.
- `services[].project`: cwd에서 도출. cwd가 git 저장소면 저장소 루트의 폴더명과 브랜치,
  아니면 cwd 폴더명(브랜치 없음). cwd를 못 읽으면 필드 생략.
- `generatedAt`: Unix epoch 밀리초.
- `host.os`: `macos` | `windows` | `linux` (`std::env::consts::OS`).

- **Error Response**: invoke promise rejection (문자열 메시지). 소켓 테이블 조회 실패는
  오류가 아니라 빈 목록으로 degrade하므로 정상 응답으로 온다.
- **사용하는 화면/컴포넌트**: `src/routes/+page.svelte` (3초 폴링, `lib/api.ts` 경유).
- **비고**: 로컬 머신 밖으로 나가는 데이터 없음. 파일 내용을 읽지 않음.

## `stop_service`

- **Method**: Tauri IPC invoke (HTTP 아님)
- **URL**: 없음 (커맨드 이름 `stop_service`)
- **Request Headers**: 없음
- **Parameters**: `pid` (number) — 중지할 서비스의 프로세스 ID
- **Request Body**: 없음
- **Success Response**: 없음 (`void`)
- **Error Response**: invoke promise rejection (문자열). pid가 없거나 OS가 거부한 경우
  (예: 권한 없는 프로세스) 실패 메시지와 함께 reject.
- **사용하는 화면/컴포넌트**: `src/lib/components/ServiceCard.svelte`의 중지 버튼
  (2단계 확인 후 호출, `lib/api.ts` 경유).
- **비고**: unix는 `kill -TERM`(정상 종료 신호)만 보낸다 — SIGKILL은 제공하지 않는다.
  Windows는 우아한 종료 신호가 없어 `taskkill /PID <pid> /F`(강제)로 대체.
  성공해도 프로세스가 즉시 사라진다는 보장은 없으며, 위젯은 재스캔 후 사라짐을 반영한다.

## 변경 이력

- v0.1 (2026-09-10): `get_services` 최초 정의.
- v0.1.1 (2026-09-10): `get_services` 응답에 `command`, `project` 추가.
- v0.1.2 (2026-09-10): `stop_service` 최초 정의.
