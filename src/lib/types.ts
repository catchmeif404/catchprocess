// src-tauri/src/scanner (Rust)가 직렬화하는 Snapshot DTO와 1:1로 대응되는 타입.
// 위젯 화면은 이 형태를 그대로 렌더링한다 (v0.1은 가공 없는 뷰어이므로 별도 ViewModel 없음).

/** cwd에서 도출한 프로젝트 정보. */
export interface ProjectInfo {
  /** git 저장소 폴더명 또는 cwd 폴더명. */
  name: string;
  /** 판단 근거가 된 전체 경로 (저장소 루트 또는 cwd). */
  path: string;
  /** git 저장소가 아닐 경우 필드 자체가 없다. */
  branch?: string;
}

export interface ApiTarget {
  host: string;
  port: number;
}

/** 프로세스 1개가 리슨 중인 서비스 카드 정보. */
export interface Service {
  pid: number;
  /** 소문자 실행 파일 이름 (Windows `.exe` 접미사 제거됨). */
  process: string;
  /** 이 pid가 리슨 중인 TCP 포트 전체, 오름차순 정렬. */
  ports: number[];
  /** 실행 명령줄 요약 (인자 포함, 최대 200자). 없을 경우 필드 자체가 없다. */
  command?: string;
  /** cwd 기반 프로젝트 정보. cwd를 못 읽으면 필드 자체가 없다. */
  project?: ProjectInfo;
  /** 현재 관찰된 outbound TCP 연결. */
  connections?: Connection[];
  /** source-discovered API endpoints for this process's working directory. */
  apiTargets?: ApiTarget[];
  /** source-discovered database targets for this process's working directory. */
  databaseTargets?: DatabaseTarget[];
}

export interface Connection {
  target: string;
  port: number;
  local: boolean;
}

export interface DatabaseTarget {
  engine: string;
  database: string;
  port: number;
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
