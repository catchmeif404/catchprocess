import type { Service } from './types';
import type { ManagedService } from './view-preferences';
import { serviceKey } from './view-preferences';

/** 등록된 서비스 하나의 현재 실행 상태 판정 결과. */
export interface ManagedStatus {
  config: ManagedService;
  running: boolean;
  pids: number[];
  /** 판정 근거 — 자동 등록 키 정확 일치, 프로젝트 경로 일치, 미발견. */
  match: 'service-key' | 'project-path' | 'none';
}

/**
 * 등록된 서비스 설정을 현재 스캔 결과와 대응시킨다.
 *
 * 1차: 카드에서 자동 등록된 설정(키가 [프로젝트 경로, 포트 목록] JSON)은 그 키로 정확 매칭.
 * 2차: 수동 등록(키 manual:*)은 작업 폴더가 같은 프로젝트의 실행 서비스에 붙는다 —
 * 포트가 바뀌어 재시작된 경우에도 프로젝트 단위로 "실행 중"을 유지하기 위해서다.
 * 실행 파일 이름은 러너에 따라 바뀌므로(e.g. node vs next-server) 매칭에 쓰지 않는다.
 */
export function matchManagedServices(configs: ManagedService[], services: Service[]): ManagedStatus[] {
  return configs.map(config => {
    const keyMatches = services.filter(service => serviceKey(service) === config.key);
    if (keyMatches.length > 0) {
      return { config, running: true, pids: keyMatches.map(service => service.pid), match: 'service-key' as const };
    }
    const path = config.cwd.trim();
    const pathMatches = path
      ? services.filter(service => service.project?.path === path)
      : [];
    if (pathMatches.length > 0) {
      return { config, running: true, pids: pathMatches.map(service => service.pid), match: 'project-path' as const };
    }
    return { config, running: false, pids: [], match: 'none' as const };
  });
}
