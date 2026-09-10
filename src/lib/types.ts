// src-tauri/src/scanner (Rust)가 직렬화하는 Snapshot DTO와 1:1로 대응되는 타입.
// 위젯 화면은 이 형태를 그대로 렌더링한다 (v0.1은 가공 없는 뷰어이므로 별도 ViewModel 없음).

/** 프로세스 1개가 리슨 중인 서비스 카드 정보. */
export interface Service {
  pid: number;
  /** 소문자 실행 파일 이름 (Windows `.exe` 접미사 제거됨). */
  process: string;
  /** 이 pid가 리슨 중인 TCP 포트 전체, 오름차순 정렬. */
  ports: number[];
}

export interface HostInfo {
  /** `std::env::consts::OS` 값 — "macos" | "windows" | "linux". */
  os: string;
  hostname: string;
}

export interface Snapshot {
  services: Service[];
  /** Unix epoch 밀리초. UI가 상대 시간을 계산한다. */
  generatedAt: number;
  host: HostInfo;
}
