import { describe, expect, it } from 'vitest';
import type { Service } from './types';
import { buildBoardTopology, layoutTopology } from './topology';
import type { Board } from './boards';

function service(partial: Partial<Service> & Pick<Service, 'pid' | 'process' | 'ports'>): Service {
  return { ...partial };
}

function board(partial: Partial<Board>): Board {
  return { id: 'b:1', name: 'test', nodes: [], edges: [], ...partial };
}

describe('buildBoardTopology', () => {
  it('renders declared service nodes offline when nothing runs', () => {
    const topology = buildBoardTopology(board({
      nodes: [{ id: 'n:web', name: 'web', kind: 'service', cwd: '/w/web', buildCommand: '', runCommand: 'npm run dev', envText: '', port: 5173, endpoint: '' }],
      edges: [],
    }), []);
    expect(topology.nodes[0]).toMatchObject({ kind: 'offline', label: 'web', sub: ':5173' });
    expect(topology.nodes[0].managed?.runCommand).toBe('npm run dev');
  });

  it('flips to a running service node when cwd+port match a live process', () => {
    const live = service({ pid: 21, process: 'node', ports: [5173], project: { name: 'web', path: '/w/web' } });
    const topology = buildBoardTopology(board({
      nodes: [{ id: 'n:web', name: 'web', kind: 'service', cwd: '/w/web', buildCommand: '', runCommand: '', envText: '', port: 5173, endpoint: '' }],
      edges: [],
    }), [live]);
    expect(topology.nodes[0]).toMatchObject({ kind: 'service', pids: [21] });
  });

  it('wires declared edges between declared nodes with the target port label', () => {
    const topology = buildBoardTopology(board({
      nodes: [
        { id: 'n:web', name: 'web', kind: 'service', cwd: '/w/web', buildCommand: '', runCommand: '', envText: '', port: 5173, endpoint: '' },
        { id: 'n:api', name: 'api', kind: 'service', cwd: '/w/api', buildCommand: '', runCommand: '', envText: '', port: 8080, endpoint: '' },
        { id: 'n:pg', name: 'pg', kind: 'database', cwd: '', buildCommand: '', runCommand: '', envText: '', port: 0, endpoint: 'localhost:5432' },
      ],
      edges: [
        { id: 'e:1', source: 'n:web', target: 'n:api' },
        { id: 'e:2', source: 'n:api', target: 'n:pg' },
        { id: 'e:3', source: 'n:web', target: 'n:ghost' },
      ],
    }), []);
    expect(topology.edges).toHaveLength(2);
    const toApi = topology.edges.find((edge) => edge.target === 'n:api');
    expect(toApi).toMatchObject({ observation: 'runtime', label: ':8080' });
    const toPg = topology.edges.find((edge) => edge.target === 'n:pg');
    expect(toPg?.label).toBeUndefined();
  });

  it('keeps the board stable while the same scan changes pids', () => {
    const makeBoard = () => board({
      nodes: [{ id: 'n:web', name: 'web', kind: 'service', cwd: '/w/web', buildCommand: '', runCommand: '', envText: '', port: 3000, endpoint: '' }],
      edges: [],
    });
    const first = buildBoardTopology(makeBoard(), [service({ pid: 30, process: 'node', ports: [3000], project: { name: 'web', path: '/w/web' } })]);
    const second = buildBoardTopology(makeBoard(), [service({ pid: 31, process: 'node', ports: [3000], project: { name: 'web', path: '/w/web' } })]);
    expect(first.nodes[0].id).toBe(second.nodes[0].id);
    expect(second.nodes[0].pids).toEqual([31]);
  });
});

describe('board layout', () => {
  it('lays declared graph top to bottom', () => {
    const topology = buildBoardTopology(board({
      nodes: [
        { id: 'n:web', name: 'web', kind: 'service', cwd: '/w/web', buildCommand: '', runCommand: '', envText: '', port: 5173, endpoint: '' },
        { id: 'n:api', name: 'api', kind: 'service', cwd: '/w/api', buildCommand: '', runCommand: '', envText: '', port: 8080, endpoint: '' },
        { id: 'n:pg', name: 'pg', kind: 'database', cwd: '', buildCommand: '', runCommand: '', envText: '', port: 0, endpoint: 'localhost:5432' },
      ],
      edges: [
        { id: 'e:1', source: 'n:web', target: 'n:api' },
        { id: 'e:2', source: 'n:api', target: 'n:pg' },
      ],
    }), []);
    const { nodes } = layoutTopology(topology, 552);
    const byId = Object.fromEntries(nodes.map((node) => [node.id, node]));
    expect(byId['n:web'].tier).toBe(0);
    expect(byId['n:api'].tier).toBe(1);
    expect(byId['n:pg'].tier).toBe(2);
  });
});
