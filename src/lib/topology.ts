import type { Service } from './types';
import { serviceKey, type ManagedService } from './view-preferences';
import { serviceLabel } from './service-presentation';
import type { ManagedStatus } from './managed-status';

/** 구성도 노드 — 실행 중 서비스, 꺼진 관리 서비스, DB, 외부 엔드포인트. */
export interface TopologyNode {
  id: string;
  kind: 'service' | 'offline' | 'database' | 'external';
  label: string;
  sub: string;
  service?: Service;
  managed?: ManagedService;
  pids?: number[];
}

/** 구성도 엣지 — runtime은 관측된 연결(실선), candidate는 설정 기반 추론(점선). */
export interface TopologyEdge {
  id: string;
  source: string;
  target: string;
  observation: 'runtime' | 'candidate';
  label?: string;
}

export interface Topology {
  nodes: TopologyNode[];
  edges: TopologyEdge[];
}

export interface NodePosition {
  x: number;
  y: number;
}

export interface LaidOutNode extends TopologyNode {
  x: number;
  y: number;
  tier: number;
}

export const NODE_WIDTH = 138;
export const NODE_HEIGHT = 50;
const ROW_HEIGHT = 104;
const MARGIN = 14;

const LOOPBACK = ['localhost', '127.0.0.1', '::1'];

function externalNodeId(host: string, port: number): string {
  return `ext:${host}:${port}`;
}

function databaseNodeId(engine: string, database: string, port: number): string {
  return `db:${engine}:${database}:${port}`;
}

/** 로컬 리스너 해석: 출발지 자신을 제외하고 해당 포트를 리슨하는 서비스를 찾는다. */
function listenersOnPort(services: Service[], source: Service, port: number): Service[] {
  return services.filter(
    (candidate) => candidate.pid !== source.pid && candidate.ports.includes(port),
  );
}

/**
 * 스캔 결과와 관리 서비스 설정을 노드/엣지 그래프로 조립한다.
 *
 * 엣지 우선순위: 관측된 연결(runtime)이 설정 추론(candidate)과 같은 쌍이면 runtime 하나만 남긴다.
 * 실행 중인 관리 서비스는 서비스 노드로 이미 표현되고, 매칭에 실패한 설정만 꺼진 노드가 된다.
 */
export function buildTopology(services: Service[], managedStatuses: ManagedStatus[]): Topology {
  const nodes = new Map<string, TopologyNode>();
  const edges = new Map<string, TopologyEdge>();

  for (const service of services) {
    const id = serviceKey(service);
    nodes.set(id, {
      id,
      kind: 'service',
      label: service.project?.name ?? serviceLabel(service),
      sub: service.ports.map((port) => `:${port}`).join(' ') || service.process,
      service,
      pids: [service.pid],
    });
  }

  for (const status of managedStatuses) {
    if (status.running) continue;
    const id = `managed:${status.config.key}`;
    nodes.set(id, {
      id,
      kind: 'offline',
      label: status.config.name || status.config.cwd.split('/').filter(Boolean).pop() || status.config.key,
      sub: status.config.cwd,
      managed: status.config,
    });
  }

  for (const service of services) {
    const sourceId = serviceKey(service);
    for (const connection of service.connections ?? []) {
      if (connection.local) {
        const targets = listenersOnPort(services, service, connection.port);
        if (targets.length > 0) {
          for (const target of targets) {
            addEdge(edges, sourceId, serviceKey(target), 'runtime', `:${connection.port}`);
          }
          continue;
        }
        // 자기 자신 리슨 포트로의 연결은 정보량이 없으니 버린다.
        if (service.ports.includes(connection.port)) continue;
        // 로컬 포트인데 개발 관련 리스너가 없다(시스템 프로세스 등). 관측된 엔드포인트로 남긴다.
        const targetId = externalNodeId(connection.target, connection.port);
        nodes.set(targetId, {
          id: targetId,
          kind: 'external',
          label: connection.target,
          sub: `:${connection.port}`,
        });
        addEdge(edges, sourceId, targetId, 'runtime', `:${connection.port}`);
      } else {
        const targetId = externalNodeId(connection.target, connection.port);
        nodes.set(targetId, {
          id: targetId,
          kind: 'external',
          label: connection.target,
          sub: `:${connection.port}`,
        });
        addEdge(edges, sourceId, targetId, 'runtime', `:${connection.port}`);
      }
    }

    for (const target of service.apiTargets ?? []) {
      const matches = LOOPBACK.includes(target.host)
        ? listenersOnPort(services, service, target.port)
        : [];
      if (matches.length === 1) {
        addEdge(edges, sourceId, serviceKey(matches[0]), 'candidate', `:${target.port}`);
      } else {
        const targetId = externalNodeId(target.host, target.port);
        nodes.set(targetId, {
          id: targetId,
          kind: 'external',
          label: target.host,
          sub: `:${target.port}`,
        });
        addEdge(edges, sourceId, targetId, 'candidate', `:${target.port}`);
      }
    }

    for (const target of service.databaseTargets ?? []) {
      const targetId = databaseNodeId(target.engine, target.database, target.port);
      nodes.set(targetId, {
        id: targetId,
        kind: 'database',
        label: `${target.engine} ${target.database}`.trim(),
        sub: `:${target.port}`,
      });
      addEdge(edges, sourceId, targetId, 'candidate', `:${target.port}`);
    }
  }

  return { nodes: [...nodes.values()], edges: [...edges.values()] };
}

function addEdge(
  edges: Map<string, TopologyEdge>,
  source: string,
  target: string,
  observation: 'runtime' | 'candidate',
  label?: string,
): void {
  if (source === target) return;
  const id = `${source}->${target}`;
  const existing = edges.get(id);
  // runtime이 candidate를 대체하고, 같은 관측 유형이면 처음 것을 유지한다.
  if (existing && existing.observation === 'runtime') return;
  if (existing && observation === 'candidate') return;
  edges.set(id, { id, source, target, observation, label });
}

/**
 * 층위 기반 결정론적 레이아웃. 서비스 간 엣지로 층위를 계산하고(입장 노드가 0층),
 * DB·외부 노드는 자신으로 들어오는 엣지 출발지보다 한 층 아래에 놓는다.
 * 같은 층에서는 라벨 순으로 정렬해 널찍이 배치한다. 사용자가 끌어놓은 위치(positions)가 있으면 그것이 이긴다.
 */
export function layoutTopology(
  topology: Topology,
  width: number,
  positions: Record<string, NodePosition> = {},
): { nodes: LaidOutNode[]; height: number } {
  const nodeById = new Map(topology.nodes.map((node) => [node.id, node]));
  const inbound = new Map<string, string[]>();
  const outbound = new Map<string, string[]>();
  for (const edge of topology.edges) {
    inbound.set(edge.target, [...(inbound.get(edge.target) ?? []), edge.source]);
    outbound.set(edge.source, [...(outbound.get(edge.source) ?? []), edge.target]);
  }

  const tiers = new Map<string, number>();
  function tierOf(id: string, seen: Set<string> = new Set()): number {
    const known = tiers.get(id);
    if (known !== undefined) return known;
    if (seen.has(id)) return 0;
    seen.add(id);
    const parents = (inbound.get(id) ?? []).filter((parent) => nodeById.get(parent)?.kind === 'service');
    const tier = parents.length === 0 ? 0 : Math.max(...parents.map((parent) => tierOf(parent, seen) + 1));
    tiers.set(id, tier);
    return tier;
  }
  for (const node of topology.nodes) tierOf(node.id);

  const byTier = new Map<number, TopologyNode[]>();
  for (const node of topology.nodes) {
    const tier = tiers.get(node.id) ?? 0;
    byTier.set(tier, [...(byTier.get(tier) ?? []), node]);
  }

  const laidOut: LaidOutNode[] = [];
  const maxTier = Math.max(-1, ...[...byTier.keys()]);
  for (const [tier, members] of [...byTier].sort((a, b) => a[0] - b[0])) {
    members.sort((a, b) => a.label.localeCompare(b.label) || a.id.localeCompare(b.id));
    members.forEach((node, index) => {
      const saved = positions[node.id];
      laidOut.push({
        ...node,
        tier,
        x: saved?.x ?? ((index + 1) / (members.length + 1)) * width - NODE_WIDTH / 2,
        y: saved?.y ?? MARGIN + tier * ROW_HEIGHT,
      });
    });
  }

  const height = MARGIN * 2 + Math.max(2, maxTier + 1) * ROW_HEIGHT;
  return { nodes: laidOut, height };
}
