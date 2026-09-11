import type { ApiTarget, Service } from './types';

/** Display framework evidence without inferring application roles. */
export function serviceLabel(service: Service): string {
  const command = service.command ?? '';
  if (service.framework === 'nextjs' || /next-server|next\/dist|\bnext dev\b/.test(command)) return 'Next.js';
  if (service.framework === 'spring' || /org\.springframework\.boot|spring-boot/.test(command)) return 'Spring Boot';
  if (service.process === 'postgres') return 'PostgreSQL';
  if (service.process === 'redis-server') return 'Redis';
  return service.process;
}

export function apiLabel(target: ApiTarget, source: Service, services: Service[]): string {
  const loopback = ['localhost', '127.0.0.1', '::1'].includes(target.host);
  const matches = loopback ? services.filter(s => s.pid !== source.pid && s.ports.includes(target.port)) : [];
  return matches.length === 1 ? serviceLabel(matches[0]) : target.host;
}
