import { describe, expect, it } from 'vitest';
import type { Service } from './types';
import { buildTopology, layoutTopology } from './topology';
import { serviceKey, type ManagedService } from './view-preferences';

function service(partial: Partial<Service> & Pick<Service, 'pid' | 'process' | 'ports'>): Service {
  return { ...partial };
}

function offlineConfig(partial: Partial<ManagedService> = {}): { config: ManagedService; running: false; pids: []; match: 'none' } {
  return {
    config: { key: 'manual:1', name: 'backend', cwd: '/work/proj', buildCommand: '', runCommand: './run', envText: '', ...partial },
    running: false,
    pids: [],
    match: 'none',
  };
}

describe('buildTopology', () => {
  it('creates service nodes and runtime edges for observed local connections', () => {
    const frontend = service({ pid: 1, process: 'node', ports: [5173], project: { name: 'web', path: '/w/web' } });
    const backend = service({ pid: 2, process: 'java', ports: [8080], project: { name: 'api', path: '/w/api' } });
    const calling = service({
      pid: 3, process: 'node', ports: [5174], project: { name: 'web2', path: '/w/web2' },
      connections: [{ target: '127.0.0.1', port: 8080, local: true }],
    });
    const topology = buildTopology([frontend, backend, calling], []);
    const edge = topology.edges.find((e) => e.source === serviceKey(calling) && e.target === serviceKey(backend));
    expect(edge).toMatchObject({ observation: 'runtime', label: ':8080' });
    expect(topology.nodes.map((n) => n.kind)).toEqual(['service', 'service', 'service']);
  });

  it('unresolved local ports and remote hosts become external nodes', () => {
    const caller = service({
      pid: 5, process: 'java', ports: [8080], project: { name: 'api', path: '/w/api' },
      connections: [
        { target: '10.0.0.8', port: 5432, local: false },
        { target: '127.0.0.1', port: 9999, local: true },
      ],
      databaseTargets: [{ engine: 'postgresql', database: 'app', port: 5432 }],
    });
    const topology = buildTopology([caller], []);
    const kinds = Object.fromEntries(topology.nodes.map((n) => [n.id, n.kind]));
    expect(kinds['ext:10.0.0.8:5432']).toBe('external');
    expect(kinds['ext:127.0.0.1:9999']).toBe('external');
    expect(kinds['db:postgresql:app:5432']).toBe('database');
  });

  it('unmatched managed services become offline nodes', () => {
    const topology = buildTopology([], [offlineConfig()]);
    expect(topology.nodes[0]).toMatchObject({ kind: 'offline', label: 'backend' });
  });

  it('runtime edges replace candidate edges for the same pair', () => {
    const source = service({
      pid: 7, process: 'node', ports: [3000], project: { name: 'web', path: '/w/web' },
      apiTargets: [{ host: 'localhost', port: 8080 }],
      connections: [{ target: '127.0.0.1', port: 8080, local: true }],
    });
    const backend = service({ pid: 8, process: 'java', ports: [8080], project: { name: 'api', path: '/w/api' } });
    const topology = buildTopology([source, backend], []);
    const pairEdges = topology.edges.filter((e) => e.source === serviceKey(source) && e.target === serviceKey(backend));
    expect(pairEdges).toHaveLength(1);
    expect(pairEdges[0].observation).toBe('runtime');
  });

  it('self edges from a service probing its own port are dropped', () => {
    const lonely = service({
      pid: 9, process: 'java', ports: [8080], project: { name: 'api', path: '/w/api' },
      connections: [{ target: '127.0.0.1', port: 8080, local: true }],
    });
    const topology = buildTopology([lonely], []);
    expect(topology.edges).toHaveLength(0);
  });
});

describe('layoutTopology', () => {
  it('places roots above their downstream services', () => {
    const frontend = service({ pid: 11, process: 'node', ports: [5173], project: { name: 'web', path: '/w/web' } });
    const backend = service({ pid: 12, process: 'java', ports: [8080], project: { name: 'api', path: '/w/api' } });
    const caller = service({
      pid: 13, process: 'node', ports: [5174], project: { name: 'web2', path: '/w/web2' },
      connections: [{ target: '127.0.0.1', port: 8080, local: true }],
    });
    const topology = buildTopology([frontend, backend, caller], []);
    const { nodes, height } = layoutTopology(topology, 576);
    const byId = Object.fromEntries(nodes.map((node) => [node.id, node]));
    expect(byId[serviceKey(caller)].tier).toBe(0);
    expect(byId[serviceKey(backend)].tier).toBe(1);
    expect(byId[serviceKey(backend)].y).toBeGreaterThan(byId[serviceKey(caller)].y);
    expect(height).toBeGreaterThan(0);
  });

  it('saved positions override the computed layout', () => {
    const topology = buildTopology([service({ pid: 14, process: 'node', ports: [3000], project: { name: 'web', path: '/w/web' } })], []);
    const id = topology.nodes[0].id;
    const { nodes } = layoutTopology(topology, 576, { [id]: { x: 40, y: 90 } });
    expect(nodes[0]).toMatchObject({ x: 40, y: 90 });
  });
});
