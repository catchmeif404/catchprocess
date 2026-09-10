// Tauri IPC 호출을 모아두는 API 클라이언트. 컴포넌트는 이 모듈만 호출한다.

import { invoke } from '@tauri-apps/api/core';
import type { Snapshot } from './types';

/** Rust 쪽 #[tauri::command] 이름. */
const COMMAND_GET_SERVICES = 'get_services';

/** 위젯 폴링 주기 (밀리초) — DESIGN.md의 3초 폴링. */
export const SCAN_INTERVAL_MS = 3000;

/** 현재 스냅샷 1회 조회. 실패 시 그대로 throw — 호출부(UI)가 재시도 폴링을 담당한다. */
export function fetchSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>(COMMAND_GET_SERVICES);
}
