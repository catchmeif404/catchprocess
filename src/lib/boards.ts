import type { Service } from './types';
import type { ManagedService } from './view-preferences';

/** 구성도(보드) 노드 — service는 켜고 끌 수 있고, database/external은 엔드포인트 표시다. */
export interface BoardNode {
  id: string;
  name: string;
  kind: 'service' | 'database' | 'external';
  cwd: string;
  buildCommand: string;
  runCommand: string;
  envText: string;
  /** 실행 상태 매칭에 쓰는 리슨 포트. 0이면 cwd만으로 매칭한다. */
  port: number;
  /** database/external 노드의 표시용 엔드포인트 (예: localhost:5432). */
  endpoint: string;
}

export interface BoardEdge {
  id: string;
  source: string;
  target: string;
}

/** 프로젝트별 구성도 판. 위젯의 진실(source of truth)이다. */
export interface Board {
  id: string;
  name: string;
  nodes: BoardNode[];
  edges: BoardEdge[];
}

export function newNode(kind: BoardNode['kind'], overrides: Partial<BoardNode> = {}): BoardNode {
  return {
    id: `n:${crypto.randomUUID().slice(0, 8)}`,
    name: '',
    kind,
    cwd: '',
    buildCommand: '',
    runCommand: '',
    envText: '',
    port: 0,
    endpoint: '',
    ...overrides,
  };
}

/** 옛 플랫 설정(services.json)을 기본 보드 하나로 옮긴다. */
export function boardFromManagedServices(id: string, name: string, managed: ManagedService[]): Board {
  return {
    id,
    name,
    nodes: managed.map((item) => ({
      id: `n:${crypto.randomUUID().slice(0, 8)}`,
      name: item.name,
      kind: 'service' as const,
      cwd: item.cwd,
      buildCommand: item.buildCommand,
      runCommand: item.runCommand,
      envText: item.envText,
      port: 0,
      endpoint: '',
    })),
    edges: [],
  };
}

/** 보드 노드 하나의 현재 실행 판정. 포트를 알면 cwd+포트로, 모르면 cwd로만 맞춘다. */
export function matchBoardNode(node: BoardNode, services: Service[]): { running: boolean; pids: number[] } {
  if (node.kind !== 'service' || !node.cwd.trim()) return { running: false, pids: [] };
  const sameCwd = services.filter((service) => service.project?.path === node.cwd.trim());
  const matched = node.port > 0 ? sameCwd.filter((service) => service.ports.includes(node.port)) : sameCwd;
  return { running: matched.length > 0, pids: matched.map((service) => service.pid) };
}

/** 보드 전체 노드의 실행 판정. */
export function matchBoard(board: Board, services: Service[]): Map<string, { running: boolean; pids: number[] }> {
  const statuses = new Map<string, { running: boolean; pids: number[] }>();
  for (const node of board.nodes) statuses.set(node.id, matchBoardNode(node, services));
  return statuses;
}

/** 감지된 실행 서비스를 보드 노드로 채워주는 프리필 — 설정을 돕는 조수 역할. */
export function nodeFromDetectedService(service: Service): Partial<BoardNode> & { kind: BoardNode['kind'] } {
  return {
    kind: 'service',
    name: service.project?.name ? `${service.project.name}` : service.process,
    cwd: service.project?.path ?? '',
    port: service.ports[0] ?? 0,
    // 감지된 전체 커맨드 라인은 시작 명령의 출발점으로 좋다 — 편집해서 쓴다.
    runCommand: service.command ?? '',
  };
}

/** 보드 JSON의 방어적 정규화 — 파일이 손상돼도 위젯이 죽지 않게 한다. */
export function normalizeBoards(value: unknown): Board[] {
  if (!Array.isArray(value)) return [];
  return value.filter((board): board is Board =>
    board && typeof board.id === 'string' && typeof board.name === 'string'
      && Array.isArray(board.nodes) && Array.isArray(board.edges),
  ).map((board) => ({
    ...board,
    nodes: board.nodes.filter((node): node is BoardNode =>
      node && typeof node.id === 'string' && (node.kind === 'service' || node.kind === 'database' || node.kind === 'external')),
    edges: board.edges.filter((edge): edge is BoardEdge =>
      edge && typeof edge.id === 'string' && typeof edge.source === 'string' && typeof edge.target === 'string'),
  }));
}
