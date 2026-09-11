<script lang="ts">
  import { onMount } from 'svelte';
  // 위젯 메인 화면: 헤더(드래그 영역 + 개수 + 갱신 시각 + 닫기)와 서비스 카드 목록.
  // 데이터 페칭은 lib/api 클라이언트에 위임하고, 여기서는 3초 폴링과 상태 표시만 담당한다.
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { loadManagedServices, persistManagedServices, SCAN_INTERVAL_MS, fetchSnapshot, startService, stopService } from '$lib/api';
  import { dragScroll } from '$lib/actions/dragScroll';
  import ServiceCard from '$lib/components/ServiceCard.svelte';
  import ServiceSettings from '$lib/components/ServiceSettings.svelte';
  import { apiLabel } from '$lib/service-presentation';
  import { getLocale, setLocale, t } from '$lib/i18n.svelte';
  import { filterServices } from '$lib/search';
  import { formatClock } from '$lib/time';
  import type { Service, Snapshot } from '$lib/types';
  import { readManagedServices, readPreferences, preferenceKey, serviceConfigKey, serviceKey, parseEnvironment, type ManagedService } from '$lib/view-preferences';
  import { matchManagedServices, type ManagedStatus } from '$lib/managed-status';
  let preferences = $state(readPreferences());
  let managedServices = $state<ManagedService[]>([]);
  let selectedConfig = $state<ManagedService | null>(null);
  onMount(async () => {
    try {
      const fileServices = await loadManagedServices();
      const legacyServices = readManagedServices();
      managedServices = fileServices.length > 0 ? fileServices : legacyServices;
      if (fileServices.length === 0 && legacyServices.length > 0) await persistManagedServices(legacyServices);
    } catch {
      managedServices = readManagedServices();
    }
  });
  let showHidden = $state(false);
  let showConfigured = $state(false);
  function saveView() {
    try { localStorage.setItem(preferenceKey, JSON.stringify(preferences)); } catch { /* Session state still works. */ }
  }
  function openSettings(service: Service) {
    const key = serviceKey(service);
    selectedConfig = managedServices.find(item => item.key === key) ?? {
      key,
      name: service.project?.name ? `${service.project.name} ${service.process}` : service.process,
      cwd: service.project?.path ?? '',
      buildCommand: '',
      runCommand: '',
      envText: '',
    };
  }
  function openNewSettings() {
    selectedConfig = {
      key: `manual:${Date.now()}`,
      name: '', cwd: '', buildCommand: '', runCommand: '', envText: '',
    };
  }
  function saveServiceConfig(config: ManagedService) {
    managedServices = [...managedServices.filter(item => item.key !== config.key), config];
    void persistManagedServices(managedServices);
    try { localStorage.setItem(serviceConfigKey, JSON.stringify(managedServices)); } catch { /* Legacy fallback only. */ }
    selectedConfig = null;
  }

  // 설정 패널의 빠른 시작/재시작. 종료(terminate)는 프로세스 소멸을 확인한 뒤에야 반환하므로
  // 재시작에서 이전 인스턴스가 포트를 붙잡은 채 새 인스턴스가 뜨는 race는 생기지 않는다.
  let busyKey = $state<string | null>(null);
  let panelError = $state('');
  async function startManaged(config: ManagedService) {
    busyKey = config.key; panelError = '';
    try {
      await startService({ command: config.runCommand, cwd: config.cwd, env: parseEnvironment(config.envText) });
      await load();
    } catch (error) {
      panelError = `${t('startFailed')}: ${error instanceof Error ? error.message : String(error)}`;
    } finally { busyKey = null; }
  }
  async function restartManaged(status: ManagedStatus) {
    busyKey = status.config.key; panelError = '';
    try {
      for (const pid of status.pids) await stopService(pid);
      await startService({ command: status.config.runCommand, cwd: status.config.cwd, env: parseEnvironment(status.config.envText) });
      await load();
    } catch (error) {
      panelError = `${t('startFailed')}: ${error instanceof Error ? error.message : String(error)}`;
    } finally { busyKey = null; }
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
  const managedStatuses = $derived(matchManagedServices(managedServices, services));
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
</script>

<svelte:head>
  <title>DevTopology</title>
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
    <button onclick={() => showHidden = !showHidden} aria-expanded={showHidden}>{t('hidden')} ({preferences.hidden.length})</button>
    <button onclick={() => showConfigured = !showConfigured} aria-expanded={showConfigured}>{t('configuredServices')} ({managedServices.length})</button>
  </div>
  {#if showConfigured}
    <div class="hidden-panel configured-panel">
      <button class="register" onclick={openNewSettings}>＋ {t('registerService')}</button>
      {#if panelError}<p class="panel-error" role="alert">{panelError}</p>{/if}
      {#each managedStatuses as status (status.config.key)}
        <div class="hidden-row">
          <span>{status.config.name}<small>{status.config.cwd}</small></span>
          <span class="run-state" class:down={!status.running}>{status.running ? t('statusUp', { pids: status.pids.join(', ') }) : t('statusDown')}</span>
          {#if status.running}
            <button disabled={busyKey !== null || !status.config.runCommand} onclick={() => void restartManaged(status)}>{busyKey === status.config.key ? t('working') : t('restart')}</button>
          {:else}
            <button disabled={busyKey !== null || !status.config.runCommand} onclick={() => void startManaged(status.config)}>{busyKey === status.config.key ? t('working') : t('start')}</button>
          {/if}
          <button onclick={() => selectedConfig = { ...status.config }}>{t('configure')}</button>
        </div>
      {/each}
    </div>
  {/if}
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
                onconfigure={() => openSettings(service)}
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

{#if selectedConfig}
  <ServiceSettings config={selectedConfig} onsave={saveServiceConfig} onclose={() => selectedConfig = null} />
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
  .hidden-panel { padding: 10px; max-height: 160px; overflow: auto; background: var(--paper-card); font: 12px system-ui; border-radius: 6px; }
  .hidden-row { display: flex; align-items: center; gap: 8px; }
  .hidden-row span { overflow-wrap: anywhere; flex: 1; }
  .hidden-row small { display: block; margin-top: 3px; color: var(--ink-muted); font: 11px/1.4 ui-monospace, monospace; overflow-wrap: anywhere; }
  .run-state { flex: none !important; padding: 2px 6px; border: 1px solid var(--stamp); border-radius: 3px; color: var(--stamp); font: 10px/1.4 ui-monospace, monospace; text-transform: uppercase; letter-spacing: .04em; }
  .run-state.down { border-color: var(--ink-muted); color: var(--ink-muted); }
  .panel-error { margin: 4px 0; color: var(--stamp); font: 11px/1.4 system-ui; }
  .configured-panel { max-height: 220px; }
  .status { margin-top: 0; flex-shrink: 0; background: transparent; border: 0; padding: 0; font: 11px system-ui; }
</style>
