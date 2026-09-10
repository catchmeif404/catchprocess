// 검색 필터: 프로젝트명/프로세스명/브랜치/포트/PID를 대소문자 무시하고 부분 일치한다.
// 순수 함수 — UI 없이 유닛 테스트 대상.

import type { Service } from './types';

export function filterServices(services: Service[], query: string): Service[] {
  const q = query.trim().toLowerCase();
  if (!q) return services;
  return services.filter(
    (service) =>
      service.process.toLowerCase().includes(q) ||
      (service.project?.name ?? '').toLowerCase().includes(q) ||
      (service.project?.branch ?? '').toLowerCase().includes(q) ||
      service.ports.some((port) => String(port).includes(q)) ||
      String(service.pid).includes(q),
  );
}
