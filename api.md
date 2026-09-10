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
    { "pid": 44121, "process": "node", "ports": [5173] },
    { "pid": 55231, "process": "java", "ports": [8080] },
    { "pid": 3332, "process": "postgres", "ports": [5432] }
  ],
  "generatedAt": 1760000000000,
  "host": { "os": "macos", "hostname": "dev-machine" }
}
```

- `services`: dev 관련성 필터(`src-tauri/src/scanner/registry.rs`의 allowlist)를 통과한
  프로세스만 포함. 포트 오름차순 정렬.
- `generatedAt`: Unix epoch 밀리초.
- `host.os`: `macos` | `windows` | `linux` (`std::env::consts::OS`).

- **Error Response**: invoke promise rejection (문자열 메시지). 소켓 테이블 조회 실패는
  오류가 아니라 빈 목록으로 degrade하므로 정상 응답으로 온다.
- **사용하는 화면/컴포넌트**: `src/routes/+page.svelte` (3초 폴링, `lib/api.ts` 경유).
- **비고**: 로컬 머신 밖으로 나가는 데이터 없음. 파일 내용을 읽지 않음.

## 변경 이력

- v0.1 (2026-09-10): `get_services` 최초 정의.
