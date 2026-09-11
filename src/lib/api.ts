// Tauri IPC 호출을 모아두는 API 클라이언트. 컴포넌트는 이 모듈만 호출한다.

import { invoke } from '@tauri-apps/api/core';
import type { Snapshot } from './types';
import type { ManagedService } from './view-preferences';

/** Rust 쪽 #[tauri::command] 이름. */
const COMMAND_GET_SERVICES = 'get_services';
const COMMAND_STOP_SERVICE = 'stop_service';
const COMMAND_START_SERVICE = 'start_service';
const COMMAND_BUILD_SERVICE = 'build_service';

/** 위젯 폴링 주기 (밀리초) — DESIGN.md의 3초 폴링. */
export const SCAN_INTERVAL_MS = 3000;

/** 현재 스냅샷 1회 조회. 실패 시 그대로 throw — 호출부(UI)가 재시도 폴링을 담당한다. */
export function fetchSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>(COMMAND_GET_SERVICES);
}

/** 서비스 pid에 정상 종료(SIGTERM) 신호를 보낸다. OS가 거부하면 reject된다. */
export function stopService(pid: number): Promise<void> {
  return invoke<void>(COMMAND_STOP_SERVICE, { pid });
}

export interface ProcessCommandRequest {
  command: string;
  cwd: string;
  env: Record<string, string>;
}

export function startService(request: ProcessCommandRequest): Promise<number> {
  return invoke<number>(COMMAND_START_SERVICE, { request });
}

export function buildService(request: ProcessCommandRequest): Promise<string> {
  return invoke<string>(COMMAND_BUILD_SERVICE, { request });
}

export function loadManagedServices(): Promise<ManagedService[]> {
  return invoke<ManagedService[]>('get_managed_services');
}

export function persistManagedServices(services: ManagedService[]): Promise<void> {
  return invoke<void>('save_managed_services', { services });
}
