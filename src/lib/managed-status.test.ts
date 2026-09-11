import { describe, expect, it } from 'vitest';
import type { Service } from './types';
import { matchManagedServices, type ManagedStatus } from './managed-status';
import { serviceKey, type ManagedService } from './view-preferences';

function service(partial: Partial<Service> & Pick<Service, 'pid' | 'process' | 'ports'>): Service {
  return { ...partial };
}

function manualConfig(partial: Partial<ManagedService> = {}): ManagedService {
  return { key: 'manual:1', name: 'backend', cwd: '/work/proj', buildCommand: '', runCommand: './run', envText: '', ...partial };
}

describe('matchManagedServices', () => {
  it('matches auto-registered configs by their service key', () => {
    const running = service({ pid: 10, process: 'node', ports: [3000], project: { name: 'proj', path: '/work/proj' } });
    const config = manualConfig({ key: serviceKey(running) });
    const statuses = matchManagedServices([config], [running]);
    expect(statuses[0]).toMatchObject({ running: true, pids: [10], match: 'service-key' });
  });

  it('falls back to project path for manual configs when ports changed', () => {
    const running = service({ pid: 11, process: 'java', ports: [8081], project: { name: 'proj', path: '/work/proj' } });
    const config = manualConfig({ key: 'manual:2', cwd: '/work/proj' });
    const statuses = matchManagedServices([config], [running]);
    expect(statuses[0]).toMatchObject({ running: true, pids: [11], match: 'project-path' });
  });

  it('reports stopped when nothing matches', () => {
    const running = service({ pid: 12, process: 'node', ports: [3000], project: { name: 'other', path: '/work/other' } });
    const statuses = matchManagedServices([manualConfig()], [running]);
    expect(statuses[0]).toMatchObject({ running: false, pids: [], match: 'none' });
  });

  it('collects every pid under the same project path', () => {
    const frontend = service({ pid: 21, process: 'node', ports: [3000], project: { name: 'proj', path: '/work/proj' } });
    const backend = service({ pid: 22, process: 'java', ports: [8080], project: { name: 'proj', path: '/work/proj' } });
    const statuses = matchManagedServices([manualConfig()], [frontend, backend]);
    expect(statuses[0]).toMatchObject({ running: true, pids: [21, 22], match: 'project-path' });
  });

  it('ignores empty cwd instead of matching every project-less service', () => {
    const stray = service({ pid: 30, process: 'python', ports: [5001] });
    const statuses = matchManagedServices([manualConfig({ cwd: '  ' })], [stray]);
    expect(statuses[0].running).toBe(false);
  });

  it('never lets a key match a service with the same ports in another project', () => {
    const elsewhere = service({ pid: 40, process: 'node', ports: [3000], project: { name: 'elsewhere', path: '/work/elsewhere' } });
    const config = manualConfig({ key: serviceKey(service({ pid: 1, process: 'node', ports: [3000], project: { name: 'proj', path: '/work/proj' } })) });
    const statuses: ManagedStatus[] = matchManagedServices([config], [elsewhere]);
    expect(statuses[0].match).toBe('none');
    expect(statuses[0].running).toBe(false);
  });
});
