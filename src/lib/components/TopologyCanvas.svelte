<script lang="ts">
  // 구성도 캔버스 — 설계 문서 18장의 topology graph. 종이 카드 노드가 층위로 떠 있고
  // 관측된 연결은 실선, 설정 기반 추론은 점선으로 이어진다. 노드는 끌어서 배치할 수 있고
  // 클릭하면 부모가 상세 스트립을 보여준다.
  import type { LaidOutNode } from '$lib/topology';
  import type { TopologyEdge } from '$lib/topology';
  import { NODE_HEIGHT, NODE_WIDTH } from '$lib/topology';

  interface Props {
    nodes: LaidOutNode[];
    edges: TopologyEdge[];
    width: number;
    height: number;
    selectedId: string | null;
    onselect: (id: string | null) => void;
    onnodemove: (id: string, x: number, y: number) => void;
  }

  let { nodes, edges, width, height, selectedId, onselect, onnodemove }: Props = $props();

  const PAD = 10;
  const CANVAS_WIDTH = $derived(width + PAD * 2);
  const CANVAS_HEIGHT = $derived(height + PAD * 2);
  const byId = $derived(new Map(nodes.map((node) => [node.id, node])));

  interface DragState { id: string; offsetX: number; offsetY: number; moved: boolean; }
  let drag: DragState | null = null;

  function nodeAt(id: string): LaidOutNode | undefined {
    return byId.get(id);
  }

  // 엣지 경로: 출발지 아래 중점에서 도착지 위 중점으로 부드러운 S자 곡선.
  function edgePath(edge: TopologyEdge): string | null {
    const from = nodeAt(edge.source);
    const to = nodeAt(edge.target);
    if (!from || !to) return null;
    const x1 = from.x + NODE_WIDTH / 2 + PAD;
    const y1 = from.y + NODE_HEIGHT + PAD;
    const x2 = to.x + NODE_WIDTH / 2 + PAD;
    const y2 = to.y + PAD;
    const mid = (y1 + y2) / 2;
    return `M ${x1} ${y1} C ${x1} ${mid}, ${x2} ${mid}, ${x2} ${y2}`;
  }

  function startDrag(event: PointerEvent, node: LaidOutNode): void {
    event.stopPropagation();
    drag = { id: node.id, offsetX: event.clientX - node.x, offsetY: event.clientY - node.y, moved: false };
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
  }

  function moveDrag(event: PointerEvent): void {
    if (!drag) return;
    // 캔버스가 scale 없이 1:1로 렌더되므로 client 좌표를 그대로 쓴다.
    const x = Math.max(0, Math.min(width - NODE_WIDTH, event.clientX - drag.offsetX));
    const y = Math.max(0, Math.min(height - NODE_HEIGHT, event.clientY - drag.offsetY));
    const node = nodeAt(drag.id);
    if (node) {
      node.x = x;
      node.y = y;
      drag.moved = true;
    }
  }

  function endDrag(): void {
    if (drag) {
      const node = nodeAt(drag.id);
      if (node && drag.moved) onnodemove(node.id, node.x, node.y);
      drag = null;
    }
  }

  function clickNode(node: LaidOutNode): void {
    onselect(selectedId === node.id ? null : node.id);
  }

  function keyNode(event: KeyboardEvent, node: LaidOutNode): void {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      clickNode(node);
    }
  }

  // 배경 클릭(노드 밖)은 선택 해제. 노드의 pointerdown은 stopPropagation이라 여기까지 안 온다.
  function backgroundPointerDown(event: PointerEvent): void {
    const target = event.target as Element;
    if (target.classList?.contains('canvas')) onselect(null);
  }
</script>

<svelte:window onpointermove={moveDrag} onpointerup={endDrag} onpointerdown={backgroundPointerDown} onkeydown={(event) => { if (event.key === 'Escape') onselect(null); }} />

<svg
  class="canvas"
  viewBox="0 0 {CANVAS_WIDTH} {CANVAS_HEIGHT}"
  {width}
  role="application"
  aria-label="topology map"
>
  <defs>
    <marker id="arrow-ink" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" class="arrow" />
    </marker>
    <marker id="arrow-muted" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" class="arrow muted" />
    </marker>
  </defs>

  {#each edges as edge (edge.id)}
    {@const path = edgePath(edge)}
    {@const highlighted = selectedId !== null && (edge.source === selectedId || edge.target === selectedId)}
    {@const dimmed = selectedId !== null && !highlighted}
    {#if path}
      <path
        d={path}
        class="edge {edge.observation}"
        class:highlighted
        class:dimmed
        marker-end="url(#arrow-{edge.observation === 'runtime' ? 'ink' : 'muted'})"
      />
    {/if}
  {/each}

  {#each nodes as node (node.id)}
    <g
      class="node {node.kind}"
      class:selected={selectedId === node.id}
      transform="translate({node.x + PAD}, {node.y + PAD})"
      role="button"
      tabindex="-1"
      aria-label="{node.label} {node.sub}"
      onpointerdown={(event) => startDrag(event, node)}
      onkeydown={(event) => keyNode(event, node)}
      onclick={(event) => { event.stopPropagation(); clickNode(node); }}
    >
      <rect width={NODE_WIDTH} height={NODE_HEIGHT} rx="6" />
      <text class="label" x="10" y="20">{node.label.length > 17 ? node.label.slice(0, 16) + '…' : node.label}</text>
      <text class="sub" x="10" y="38">{node.sub.length > 20 ? node.sub.slice(0, 19) + '…' : node.sub}</text>
      {#if node.kind === 'offline'}<circle class="dot" cx={NODE_WIDTH - 12} cy="12" r="4" />{/if}
    </g>
  {/each}
</svg>

<style>
  .canvas { display: block; touch-action: none; user-select: none; }
  .edge {
    fill: none;
    stroke: var(--ink);
    stroke-width: 1.4;
    opacity: .8;
  }
  .edge.candidate { stroke-dasharray: 5 4; stroke: var(--ink-muted); }
  .edge.highlighted { stroke: var(--stamp); stroke-width: 2; opacity: 1; }
  .edge.dimmed { opacity: .25; }
  .arrow { fill: var(--ink); }
  .arrow.muted { fill: var(--ink-muted); }
  .node { cursor: grab; }
  .node:active { cursor: grabbing; }
  .node rect {
    fill: var(--paper-card);
    stroke: var(--ink);
    stroke-width: 1.2;
    filter: drop-shadow(0 2px 2px rgba(36, 31, 26, .18));
  }
  .node.offline rect { stroke-dasharray: 4 3; stroke: var(--ink-muted); }
  .node.database rect, .node.external rect { fill: #241f1a08; }
  .node.selected rect { stroke: var(--stamp); stroke-width: 2; }
  .label { fill: var(--ink); font: 600 12px system-ui, sans-serif; }
  .sub { fill: var(--ink-muted); font: 10px ui-monospace, monospace; }
  .node.selected .label { fill: var(--stamp); }
  .dot { fill: var(--stamp); }
</style>
