<script lang="ts">
  import { onMount } from 'svelte';
  // 위젯 메인 화면 — 구성 주도 모델. 기본 뷰는 보드(프로젝트별 구성도 판) 위에서
  // 노드를 켜고/끄는 구성도 맵이고, 카드 뷰는 현재 감지된 프로세스의 증거 뷰다.
  // 감지(스캐너)는 보드를 채울 때의 조수이자 카드 뷰의 내용일 뿐, 맵의 진실은 보드다.
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { loadBoards, loadManagedServices, persistBoards, SCAN_INTERVAL_MS, fetchSnapshot, startService, stopService } from '$lib/api';
  import { dragScroll } from '$lib/actions/dragScroll';
  import ServiceCard from '$lib/components/ServiceCard.svelte';
  import BoardNodeEditor from '$lib/components/BoardNodeEditor.svelte';
  import TopologyCanvas from '$lib/components/TopologyCanvas.svelte';
  import { apiLabel } from '$lib/service-presentation';
  import { getLocale, setLocale, t } from '$lib/i18n.svelte';
  import { filterServices } from '$lib/search';
  import { formatClock } from '$lib/time';
  import type { Service, Snapshot } from '$lib/types';
  import { readPreferences, preferenceKey, serviceKey, parseEnvironment } from '$lib/view-preferences';
  import { boardFromManagedServices, newNode, nodeFromDetectedService, normalizeBoards, type Board, type BoardNode } from '$lib/boards';
  import { buildBoardTopology, layoutTopology } from '$lib/topology';

  let preferences = $state(readPreferences());
  let boards = $state<Board[]>([]);
  let activeBoardId = $state<string | null>(null);
  const activeBoard = $derived(boards.find((board) => board.id === activeBoardId) ?? null);

  function newBoardId(): string {
    return `b:${crypto.randomUUID().slice(0, 8)}`;
  }

  async function persistAll(): Promise<void> {
    try { await persistBoards(boards); } catch { /* 다음 변경 때 다시 시도한다. */ }
  }

  function updateBoard(boardId: string, update: (board: Board) => Board): void {
    boards = boards.map((board) => (board.id === boardId ? update(board) : board));
    void persistAll();
  }

  onMount(async () => {
    let stored: Board[] = [];
    try { stored = normalizeBoards(await loadBoards()); } catch { stored = []; }
    if (stored.length > 0) {
      boards = stored;
    } else {
      // 마이그레이션: 옛 플랫 설정(services.json)이 있으면 기본 보드로 옮긴다.
      try {
        const legacy = await loadManagedServices();
        if (legacy.length > 0) {
          boards = [boardFromManagedServices(newBoardId(), t('defaultBoardName'), legacy)];
          await persistBoards(boards);
        }
      } catch { /* legacy 파일이 없으면 그냥 빈 상태다. */ }
    }
    activeBoardId = (preferences.activeBoardId && boards.some((board) => board.id === preferences.activeBoardId))
      ? preferences.activeBoardId
      : boards[0]?.id ?? null;
  });

  function createBoard(): void {
    const board: Board = { id: newBoardId(), name: `${t('defaultBoardName')} ${boards.length + 1}`, nodes: [], edges: [] };
    boards = [...boards, board];
    activeBoardId = board.id;
    void persistAll();
  }

  function deleteBoard(): void {
    if (!activeBoard || !window.confirm(t('confirmDeleteBoard'))) return;
    boards = boards.filter((board) => board.id !== activeBoard.id);
    activeBoardId = boards[0]?.id ?? null;
    void persistAll();
  }

  let showHidden = $state(false);
  function saveView() {
    try { localStorage.setItem(preferenceKey, JSON.stringify(preferences)); } catch { /* Session state still works. */ }
  }
  function hideService(service: Service) {
    preferences.hidden = [...new Set([...preferences.hidden, serviceKey(service)])];
    saveView();
  }
  function restore(key: string) {
    preferences.hidden = preferences.hidden.filter(item => item !== key);
    saveView();
  }
  function toggleProject(key: string) {
    preferences.collapsed = preferences.collapsed.includes(key)
      ? preferences.collapsed.filter(item => item !== key) : [...preferences.collapsed, key];
    saveView();
  }
  function moveProject(index: number, delta: number) {
    const keys = projectGroups.map(group => group.key);
    const destination = index + delta;
    if (destination < 0 || destination >= keys.length) return;
    [keys[index], keys[destination]] = [keys[destination], keys[index]];
    preferences.order = [...keys, ...preferences.order.filter(key => !keys.includes(key))];
    saveView();
  }

  interface ProjectGroup {
    key: string;
    name: string;
    branch?: string;
    services: Service[];
  }

  function groupServices(items: Service[]): ProjectGroup[] {
    const groups = new Map<string, ProjectGroup>();
    for (const service of items) {
      const project = service.project;
      const key = project?.path ?? `unassigned:${service.pid}`;
      const group = groups.get(key) ?? {
        key,
        name: project?.name ?? 'unassigned',
        branch: project?.branch,
        services: [],
      };
      group.services.push(service);
      groups.set(key, group);
    }
    return [...groups.values()];
  }

  function apiTargetLabels(service: Service): Record<string, string> {
    const labels: Record<string, string> = {};
    for (const target of service.apiTargets ?? []) {
      labels[`${target.host}:${target.port}`] = apiLabel(target, service, services);
    }
    return labels;
  }

  let snapshot = $state<Snapshot | null>(null);
  let failed = $state(false);
  let query = $state('');

  const services = $derived(snapshot?.services ?? []);
  const visibleServices = $derived(filterServices(services.filter(s => !preferences.hidden.includes(serviceKey(s))), query));
  const projectGroups = $derived(groupServices(visibleServices).sort((a,b) => {
    const rank = (key: string) => { const index = preferences.order.indexOf(key); return index < 0 ? Number.MAX_SAFE_INTEGER : index; };
    return rank(a.key) - rank(b.key) || a.name.localeCompare(b.name);
  }));
  const serviceCount = $derived(services.length);
  const updatedAt = $derived(
    snapshot ? formatClock(snapshot.generatedAt, getLocale()) : null,
  );

  // 폴링 요청 겹침(느린 스캔 + 짧은 주기) 시 이전 요청의 늦은 응답이 최신 상태를 덮어쓰는
  // race condition을 시퀀스 번호로 차단한다 — 마지막 요청의 응답만 상태에 반영.
  let requestSeq = 0;

  async function load(): Promise<void> {
    const seq = ++requestSeq;
    try {
      const next = await fetchSnapshot();
      if (seq === requestSeq) {
        snapshot = next;
        failed = false;
      }
    } catch {
      // 실패해도 폴링은 계속한다. 조용히 삼키지 않고 UI에 실패 상태를 표시.
      if (seq === requestSeq) {
        failed = true;
      }
    }
  }

  // 카드의 중지 버튼이 성공하면 3초 폴링을 기다리지 않고 즉시 재스캔한다.
  const onstop = (): void => {
    void load();
  };

  void load();
  $effect(() => {
    const timer = setInterval(() => void load(), SCAN_INTERVAL_MS);
    return () => clearInterval(timer);
  });

  // ---- 구성도(보드 맵) 뷰 ----
  const CANVAS_WIDTH = 552;
  const boardTopology = $derived(activeBoard ? buildBoardTopology(activeBoard, visibleServices) : { nodes: [], edges: [] });
  const laidOut = $derived(layoutTopology(boardTopology, CANVAS_WIDTH, preferences.positions));
  let selectedNodeId = $state<string | null>(null);
  let editingNode = $state<BoardNode | null>(null);
  let connectMode = $state(false);
  let connectSource = $state<string | null>(null);
  let nodeBusy = $state<string | null>(null);
  let nodeError = $state('');

  function saveNodePosition(id: string, x: number, y: number): void {
    preferences.positions = { ...preferences.positions, [id]: { x, y } };
    saveView();
  }

  function saveBoardNode(node: BoardNode): void {
    if (!activeBoard) return;
    updateBoard(activeBoard.id, (board) => ({
      ...board,
      nodes: board.nodes.some((existing) => existing.id === node.id)
        ? board.nodes.map((existing) => (existing.id === node.id ? node : existing))
        : [...board.nodes, node],
    }));
    editingNode = null;
  }

  function removeNode(nodeId: string): void {
    if (!activeBoard) return;
    updateBoard(activeBoard.id, (board) => ({
      ...board,
      nodes: board.nodes.filter((node) => node.id !== nodeId),
      edges: board.edges.filter((edge) => edge.source !== nodeId && edge.target !== nodeId),
    }));
    if (selectedNodeId === nodeId) selectedNodeId = null;
  }

  // 배선 모드: 첫 번째 노드(출발)를 고르고 다음 노드(도착)를 클릭하면 연결된다.
  function handleCanvasSelect(nodeId: string | null): void {
    if (nodeId === null) { selectedNodeId = null; return; }
    if (connectMode) {
      if (!connectSource) {
        connectSource = nodeId;
        return;
      }
      if (connectSource !== nodeId && activeBoard) {
        const id = `e:${crypto.randomUUID().slice(0, 8)}`;
        updateBoard(activeBoard.id, (board) => ({
          ...board,
          edges: board.edges.some((edge) => edge.source === connectSource && edge.target === nodeId)
            ? board.edges
            : [...board.edges, { id, source: connectSource!, target: nodeId }],
        }));
      }
      connectMode = false;
      connectSource = null;
      return;
    }
    selectedNodeId = selectedNodeId === nodeId ? null : nodeId;
  }

  function removeEdge(edgeId: string): void {
    if (!activeBoard) return;
    updateBoard(activeBoard.id, (board) => ({ ...board, edges: board.edges.filter((edge) => edge.id !== edgeId) }));
  }

  function parseEnv(text: string): Record<string, string> {
    return parseEnvironment(text);
  }

  async function startBoardNode(node: BoardNode): Promise<void> {
    if (!node.runCommand.trim()) return;
    nodeBusy = node.id; nodeError = '';
    try {
      await startService({ command: node.runCommand, cwd: node.cwd, env: parseEnv(node.envText) });
      await load();
    } catch (error) {
      nodeError = error instanceof Error ? error.message : String(error);
    } finally { nodeBusy = null; }
  }

  async function stopPids(pids: number[]): Promise<void> {
    for (const pid of pids) await stopService(pid);
    await load();
  }

  const selectedNode = $derived(laidOut.nodes.find((node) => node.id === selectedNodeId) ?? null);
  const selectedBoardNode = $derived(
    selectedNode && activeBoard ? activeBoard.nodes.find((node) => node.id === selectedNode.id) ?? null : null,
  );

  async function stopSelected(): Promise<void> {
    if (!selectedNode?.pids?.length) return;
    nodeBusy = selectedNode.id; nodeError = '';
    try { await stopPids(selectedNode.pids); }
    catch (error) { nodeError = error instanceof Error ? error.message : String(error); }
    finally { nodeBusy = null; }
  }

  async function startAll(): Promise<void> {
    if (!activeBoard) return;
    nodeError = '';
    for (const node of activeBoard.nodes) {
      if (node.kind !== 'service' || !node.runCommand.trim()) continue;
      const graph = boardTopology.nodes.find((candidate) => candidate.id === node.id);
      if (graph?.kind === 'service') continue;
      nodeBusy = node.id;
      try { await startService({ command: node.runCommand, cwd: node.cwd, env: parseEnv(node.envText) }); }
      catch (error) { nodeError = `${node.name || node.id}: ${error instanceof Error ? error.message : String(error)}`; }
      finally { nodeBusy = null; }
    }
    await load();
  }

  async function stopAll(): Promise<void> {
    if (!activeBoard) return;
    nodeError = '';
    const pids = boardTopology.nodes.flatMap((node) => (node.kind === 'service' ? node.pids ?? [] : []));
    nodeBusy = 'all';
    try { await stopPids(pids); }
    catch (error) { nodeError = error instanceof Error ? error.message : String(error); }
    finally { nodeBusy = null; }
  }

  function setView(view: 'map' | 'cards'): void {
    preferences.view = view;
    saveView();
  }
</script>


<svelte:head>
  <title>catchprocess</title>
</svelte:head>

<div class="widget">
  <!-- 프레임리스 창 드래그 영역: 헤더 전체. 시각적 "바" 없이 텍스트만 떠 있다. -->
  <header data-tauri-drag-region>
    <span class="brand" data-tauri-drag-region>{t('title')}</span>
    <span class="count" data-tauri-drag-region>
      {t('servicesUp', { count: query.trim() ? visibleServices.length : serviceCount })}
    </span>
    <div class="language" aria-label={t('language')}>
      <button
        class:active={getLocale() === 'en'}
        type="button"
        aria-label={t('switchToEnglish')}
        aria-pressed={getLocale() === 'en'}
        onclick={() => setLocale('en')}
      >EN</button>
      <span aria-hidden="true">/</span>
      <button
        class:active={getLocale() === 'ko'}
        type="button"
        aria-label={t('switchToKorean')}
        aria-pressed={getLocale() === 'ko'}
        onclick={() => setLocale('ko')}
      >KO</button>
    </div>
    <button
      class="window-hide"
      type="button"
      aria-label={t('hideWindow')}
      title={t('hideWindow')}
      onclick={() => void getCurrentWindow().hide()}
    >
      —
    </button>
    <button
      class="close"
      type="button"
      aria-label={t('close')}
      onclick={() => void getCurrentWindow().close()}
    >
      ×
    </button>
  </header>

  <div class="searchrow">
    <input
      class="search"
      type="search"
      placeholder={t('searchPlaceholder')}
      bind:value={query}
      aria-label={t('searchPlaceholder')}
    />
  </div>

  <div class="view-tools">
    <span class="view-toggle" role="group" aria-label="view mode">
      <button class:active={preferences.view === 'map'} onclick={() => setView('map')}>{t('viewMap')}</button>
      <button class:active={preferences.view === 'cards'} onclick={() => setView('cards')}>{t('viewCards')}</button>
    </span>
    <button onclick={() => showHidden = !showHidden} aria-expanded={showHidden}>{t('hidden')} ({preferences.hidden.length})</button>
  </div>
  {#if showHidden}
    <div class="hidden-panel">
      <p>{t('hiddenNote')}</p>
      {#each preferences.hidden as key}
        <div class="hidden-row"><span>{services.find(s => serviceKey(s) === key)?.project?.name ?? key}</span><button onclick={() => restore(key)}>{t('restore')}</button></div>
      {/each}
    </div>
  {/if}

  {#if failed}
    <p class="status error" role="alert">{t('scanFailed')}</p>
  {:else if preferences.view === 'map'}
    <div class="board-bar">
      {#if boards.length > 0}
        <select
          class="board-select"
          bind:value={activeBoardId}
          aria-label="board"
          onchange={() => { selectedNodeId = null; connectMode = false; connectSource = null; nodeError = ''; preferences.activeBoardId = activeBoardId; saveView(); }}
        >
          {#each boards as board (board.id)}
            <option value={board.id}>{board.name}</option>
          {/each}
        </select>
      {/if}
      <button onclick={createBoard}>{t('newBoard')}</button>
      {#if activeBoard}
        <span class="spacer"></span>
        <button disabled={nodeBusy !== null} onclick={() => void startAll()}>{nodeBusy === 'all' ? t('working') : t('startAll')}</button>
        <button disabled={nodeBusy !== null} onclick={() => void stopAll()}>{nodeBusy === 'all' ? t('working') : t('stopAll')}</button>
        <button class:wiring={connectMode} onclick={() => { connectMode = !connectMode; connectSource = null; selectedNodeId = null; }}>{connectMode ? (connectSource ? t('to') : t('from')) : t('connect')}</button>
        <button onclick={() => editingNode = newNode('service')}>{t('addNode')}</button>
        <button onclick={deleteBoard}>{t('deleteBoard')}</button>
      {/if}
    </div>
    {#if nodeError}<p class="panel-error" role="alert">{nodeError}</p>{/if}
    {#if !activeBoard || activeBoard.nodes.length === 0}
      <p class="status">{t('emptyBoard')}</p>
    {:else}
      <div class="map-wrap">
        <TopologyCanvas
          nodes={laidOut.nodes}
          edges={boardTopology.edges}
          width={CANVAS_WIDTH}
          height={laidOut.height}
          selectedId={connectMode ? connectSource : selectedNodeId}
          onselect={handleCanvasSelect}
          onnodemove={saveNodePosition}
        />
      </div>
      {#if connectMode}
        <p class="status">{connectSource ? t('to') : t('from')}</p>
      {/if}
      {#if selectedNode && selectedBoardNode}
        <div class="node-strip" role="region" aria-label={selectedNode.label}>
          <div class="strip-head">
            <strong>{selectedNode.label}</strong>
            <span class="strip-sub">{selectedNode.sub}{selectedNode.pids?.length ? ` · pid ${selectedNode.pids.join(', ')}` : ''}</span>
            <span class="run-state" class:down={selectedNode.kind === 'offline'}>{selectedNode.kind === 'offline' ? t('statusDown') : t('statusUp', { pids: selectedNode.pids?.join(', ') ?? '' })}</span>
            <button class="strip-close" onclick={() => selectedNodeId = null}>×</button>
          </div>
          <div class="strip-actions">
            {#if selectedNode.kind === 'offline'}
              <button disabled={nodeBusy !== null || !selectedBoardNode.runCommand} onclick={() => void startBoardNode(selectedBoardNode)}>{nodeBusy === selectedBoardNode.id ? t('working') : t('start')}</button>
            {:else if selectedNode.pids?.length}
              <button disabled={nodeBusy !== null} onclick={() => void stopSelected()}>{nodeBusy === selectedBoardNode.id ? t('stopping') : t('stop')}</button>
            {/if}
            <button onclick={() => editingNode = { ...selectedBoardNode }}>{t('configure')}</button>
            <button onclick={() => removeNode(selectedBoardNode.id)}>{t('removeNode')}</button>
          </div>
        </div>
      {/if}
      {#if activeBoard.edges.length > 0}
        <details class="wires">
          <summary>{t('connect')} ({activeBoard.edges.length})</summary>
          {#each activeBoard.edges as edge (edge.id)}
            {@const sourceName = activeBoard.nodes.find((node) => node.id === edge.source)?.name ?? edge.source}
            {@const targetName = activeBoard.nodes.find((node) => node.id === edge.target)?.name ?? edge.target}
            <div class="hidden-row"><span>{sourceName} → {targetName}</span><button onclick={() => removeEdge(edge.id)}>×</button></div>
          {/each}
        </details>
      {/if}
    {/if}
    {#if updatedAt}
      <p class="status">{t('updatedAt', { time: updatedAt })}</p>
    {/if}
  {:else if serviceCount === 0}
    <p class="status">{t('empty')}</p>
  {:else if visibleServices.length === 0}
    <p class="status">{t('noMatches')}</p>
  {:else}
    <div class="groups" use:dragScroll>
      {#each projectGroups as group, index (group.key)}
        <section class="project-group" aria-label={group.name}>
          <div class="project-header">
            <button class="project-name" aria-expanded={!preferences.collapsed.includes(group.key)} onclick={() => toggleProject(group.key)}>{preferences.collapsed.includes(group.key) ? '▸' : '▾'} {group.name}</button>
            {#if group.branch}<span class="branch">· {group.branch}</span>{/if}
            <span class="project-actions"><button title={t('moveUp')} aria-label={t('moveUp')} disabled={index === 0} onclick={() => moveProject(index, -1)}>↑</button><button title={t('moveDown')} aria-label={t('moveDown')} disabled={index === projectGroups.length - 1} onclick={() => moveProject(index, 1)}>↓</button></span>
          </div>
          {#if !preferences.collapsed.includes(group.key) || query.trim()}
          <ul class="list">
            {#each group.services as service (service.pid)}
              <ServiceCard
                {service}
                {onstop}
                onhide={() => hideService(service)}
                onconfigure={() => editingNode = newNode('service', nodeFromDetectedService(service))}
                compact
                apiTargetLabels={apiTargetLabels(service)}
              />
            {/each}
          </ul>
          {/if}
        </section>
      {/each}
    </div>
    {#if updatedAt}
      <p class="status">{t('updatedAt', { time: updatedAt })}</p>
    {/if}
  {/if}
</div>

{#if editingNode}
  <BoardNodeEditor initial={editingNode} detected={services} onsave={saveBoardNode} onclose={() => editingNode = null} />
{/if}

<style>
  /* 외곽 박스 없음: 창 자체가 투명하고, 종이 카드들과 작은 종이 조각만 떠 있다.
     토큰은 app.css의 케이스 파일 팔레트(site와 동일 값)를 쓴다. */
  .widget {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    height: 100vh;
    padding: 0.45rem;
    box-sizing: border-box;
    color: var(--ink);
    font-family: var(--mono);
    font-size: 0.8rem;
    overflow: hidden;
  }

  /* 헤더: site의 .upcoming-item처럼 작은 종이 조각(점선 테두리), 살짝 기울여 띄움 */
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.6rem;
    border: 1px dashed rgba(36, 31, 26, 0.35);
    background: var(--paper);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    user-select: none;
  }

  .brand {
    font-family: var(--serif-stamp);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .count {
    flex: 1;
    color: var(--ink-muted);
    letter-spacing: 0.03em;
  }

  .window-hide,
  .close {
    flex: none;
    padding: 0 0.35rem;
    border: none;
    border-radius: 2px;
    background: transparent;
    color: var(--ink-muted);
    font-size: 0.9rem;
    line-height: 1.4;
    cursor: pointer;
  }

  .language {
    display: flex;
    align-items: center;
    gap: 0.15rem;
    color: var(--ink-muted);
    font-size: 0.62rem;
    letter-spacing: 0.04em;
  }

  .language button {
    padding: 0.1rem 0.15rem;
    border: none;
    background: transparent;
    color: var(--ink-muted);
    font: inherit;
    cursor: pointer;
  }

  .language button.active {
    color: var(--stamp);
    font-weight: 700;
    text-decoration: underline;
    text-underline-offset: 0.15rem;
  }

  .language button:hover {
    color: var(--stamp);
  }

  .window-hide:hover,
  .close:hover {
    background: rgba(163, 43, 43, 0.12);
    color: var(--stamp);
  }

  /* 검색 칸: site 팁 폼 입력 스타일(종이 위 잉크 테두리)을 위젯 크기로 축소 */
  .searchrow {
    display: flex;
  }

  .search {
    flex: 1;
    min-width: 0;
    padding: 0.25rem 0.5rem;
    border: 1px solid rgba(36, 31, 26, 0.4);
    border-radius: 2px;
    background: var(--paper-card);
    color: var(--ink);
    font: inherit;
    font-size: 0.72rem;
  }

  .search:focus-visible {
    outline: 2px solid var(--stamp);
    outline-offset: 1px;
  }

  .search::placeholder {
    color: var(--ink-muted);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    /* 사이드 스크롤바 숨김 — 휠/트랙패드 스크롤과 드래그 스크롤이 동작한다 */
    scrollbar-width: none;
    /* 드래그 스크롤 중 텍스트 선택 방지 */
    user-select: none;
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }

  .groups::-webkit-scrollbar {
    display: none;
  }

  .project-group {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .project-header {
    display: flex;
    align-items: baseline;
    gap: 0.35rem;
    padding: 0.2rem 0.45rem;
    border-bottom: 1px dashed rgba(36, 31, 26, 0.35);
    background: var(--paper);
    color: var(--ink);
    font-family: var(--serif-stamp);
    font-size: 0.78rem;
    font-weight: 700;
    text-transform: uppercase;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.14);
  }

  .branch {
    color: var(--ink-muted);
    font-family: var(--mono);
    font-size: 0.64rem;
    font-weight: 400;
    text-transform: none;
  }

  .list::-webkit-scrollbar {
    display: none;
  }

  /* 서류 더미 느낌: 카드마다 미세한 기울임 (site .exhibit의 --rot 변주) */
  .list :global(li:nth-child(2n)) {
    transform: none;
  }

  .list :global(li:nth-child(2n + 1)) {
    transform: none;
  }

  .status {
    margin: auto 0 0;
    padding: 0.2rem 0.5rem;
    border: 1px dashed rgba(36, 31, 26, 0.3);
    background: rgba(241, 233, 210, 0.85);
    color: var(--ink-muted);
    font-size: 0.7rem;
    font-style: italic;
    letter-spacing: 0.03em;
  }

  .status.error {
    color: var(--stamp);
    font-style: normal;
  }

  .widget { padding: 14px; gap: 12px; background: #e8dcc3; border: 1px solid #cdbfa6; border-radius: 12px; }
  header { padding: 8px 2px 12px; border: 0; border-bottom: 1px solid #cdbfa6; box-shadow: none; flex-shrink: 0; }
  .brand { font-size: 17px; }
  .search { padding: 10px 12px; border-radius: 6px; font: 13px system-ui; }
  .groups { flex: 1; gap: 14px; scrollbar-width: thin; }
  .project-group { flex-shrink: 0; gap: 0; background: var(--paper-card); border: 1px solid #cdbfa6; border-radius: 8px; overflow: hidden; }
  .project-header { padding: 10px 12px; gap: 8px; box-shadow: none; text-transform: none; }
  .project-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; text-align: left; }
  .project-actions { display: flex; margin-left: auto; }
  button { cursor: pointer; font: 12px system-ui; border: 0; background: transparent; color: var(--ink); padding: 5px 7px; border-radius: 4px; }
  button:hover { background: #241f1a0d; }
  button:disabled { opacity: .3; cursor: default; }
  button:focus-visible { outline: 2px solid var(--stamp); outline-offset: 2px; }
  .list { gap: 0; overflow: visible; }
  .view-tools { display: flex; justify-content: flex-end; }
  .view-toggle { display: flex; margin-right: auto; border: 1px solid #241f1a1a; border-radius: 5px; overflow: hidden; }
  .view-toggle button { border-radius: 0; }
  .view-toggle button.active { background: var(--ink); color: var(--paper); }
  .map-wrap { overflow: auto; border-radius: 6px; }
  .board-bar { display: flex; flex-wrap: wrap; align-items: center; gap: 4px; margin: 4px 0; }
  .board-select { flex: none; max-width: 200px; padding: 5px 7px; border: 1px solid #241f1a33; border-radius: 4px; background: var(--paper-card); color: var(--ink); font: 12px system-ui; }
  .board-bar .spacer { flex: 1; }
  .board-bar .wiring { border-color: var(--stamp); color: var(--stamp); }
  .wires { margin-top: 6px; font: 12px system-ui; color: var(--ink-muted); }
  .wires summary { cursor: pointer; }
  .node-strip { position: sticky; bottom: 0; display: grid; gap: 6px; margin-top: 6px; padding: 10px 12px; background: var(--paper-card); border: 1px solid var(--ink); border-radius: 6px; box-shadow: 0 4px 14px rgba(36,31,26,.22); font: 12px system-ui; }
  .strip-head { display: flex; align-items: center; gap: 8px; }
  .strip-head strong { overflow-wrap: anywhere; }
  .strip-sub { color: var(--ink-muted); font: 10px/1.4 ui-monospace, monospace; overflow-wrap: anywhere; }
  .strip-close { margin-left: auto; }
  .strip-actions { display: flex; gap: 6px; }
  .strip-actions button { border: 1px solid #241f1a33; }
  .hidden-panel { padding: 10px; max-height: 160px; overflow: auto; background: var(--paper-card); font: 12px system-ui; border-radius: 6px; }
  .hidden-row { display: flex; align-items: center; gap: 8px; }
  .hidden-row span { overflow-wrap: anywhere; flex: 1; }
  .run-state { flex: none !important; padding: 2px 6px; border: 1px solid var(--stamp); border-radius: 3px; color: var(--stamp); font: 10px/1.4 ui-monospace, monospace; text-transform: uppercase; letter-spacing: .04em; }
  .run-state.down { border-color: var(--ink-muted); color: var(--ink-muted); }
  .panel-error { margin: 4px 0; color: var(--stamp); font: 11px/1.4 system-ui; }
  .status { margin-top: 0; flex-shrink: 0; background: transparent; border: 0; padding: 0; font: 11px system-ui; }
</style>
