import { describe, expect, it } from 'vitest';
import type { Service } from './types';
import {
  boardFromManagedServices,
  matchBoardNode,
  nodeFromDetectedService,
  normalizeBoards,
} from './boards';

function service(partial: Partial<Service> & Pick<Service, 'pid' | 'process' | 'ports'>): Service {
  return { ...partial };
}

describe('matchBoardNode', () => {
  it('matches by cwd when no port is declared', () => {
    const running = service({ pid: 1, process: 'java', ports: [8080], project: { name: 'api', path: '/w/api' } });
    expect(matchBoardNode({ id: 'n:1', name: 'api', kind: 'service', cwd: '/w/api', buildCommand: '', runCommand: '', envText: '', port: 0, endpoint: '' }, [running]))
      .toEqual({ running: true, pids: [1] });
  });

  it('requires the declared port to listen when set', () => {
    const running = service({ pid: 2, process: 'java', ports: [8081], project: { name: 'api', path: '/w/api' } });
    const node = { id: 'n:2', name: 'api', kind: 'service' as const, cwd: '/w/api', buildCommand: '', runCommand: '', envText: '', port: 8080, endpoint: '' };
    expect(matchBoardNode(node, [running]).running).toBe(false);
  });

  it('ignores non-service nodes and blank cwd', () => {
    const db = { id: 'n:3', name: 'pg', kind: 'database' as const, cwd: '', buildCommand: '', runCommand: '', envText: '', port: 5432, endpoint: 'localhost:5432' };
    const stray = { id: 'n:4', name: 'x', kind: 'service' as const, cwd: '  ', buildCommand: '', runCommand: '', envText: '', port: 0, endpoint: '' };
    expect(matchBoardNode(db, []).running).toBe(false);
    expect(matchBoardNode(stray, [service({ pid: 3, process: 'node', ports: [1] })]).running).toBe(false);
  });
});

describe('boardFromManagedServices', () => {
  it('converts legacy flat configs into service nodes without edges', () => {
    const board = boardFromManagedServices('b:1', 'Main', [
      { key: 'k1', name: 'web', cwd: '/w/web', buildCommand: '', runCommand: 'npm run dev', envText: 'A=1' },
    ]);
    expect(board.nodes).toHaveLength(1);
    expect(board.nodes[0]).toMatchObject({ name: 'web', kind: 'service', runCommand: 'npm run dev', envText: 'A=1' });
    expect(board.edges).toEqual([]);
  });
});

describe('nodeFromDetectedService', () => {
  it('prefills from a detected process for convenient editing', () => {
    const detected = service({
      pid: 9, process: 'node', ports: [5173],
      command: 'npm run dev',
      project: { name: 'web', path: '/w/web' },
    });
    expect(nodeFromDetectedService(detected)).toMatchObject({
      kind: 'service', name: 'web', cwd: '/w/web', port: 5173, runCommand: 'npm run dev',
    });
  });
});

describe('normalizeBoards', () => {
  it('drops malformed entries instead of throwing', () => {
    expect(normalizeBoards('junk')).toEqual([]);
    expect(normalizeBoards([
      { id: 'b:1', name: 'ok', nodes: [
        { id: 'n:1', name: 'a', kind: 'service', cwd: '', buildCommand: '', runCommand: '', envText: '', port: 0, endpoint: '' },
        { id: 'n:bad' },
      ], edges: [{ id: 'e:1', source: 'n:1', target: 'n:gone' }, { nope: true }] },
      { broken: true },
    ])).toEqual([
      { id: 'b:1', name: 'ok', nodes: [
        { id: 'n:1', name: 'a', kind: 'service', cwd: '', buildCommand: '', runCommand: '', envText: '', port: 0, endpoint: '' },
      ], edges: [{ id: 'e:1', source: 'n:1', target: 'n:gone' }] },
    ]);
  });
});
