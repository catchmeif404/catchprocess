<script lang="ts">
  // 위젯 메인 화면: 헤더(드래그 영역 + 개수 + 갱신 시각 + 닫기)와 서비스 카드 목록.
  // 데이터 페칭은 lib/api 클라이언트에 위임하고, 여기서는 3초 폴링과 상태 표시만 담당한다.
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { SCAN_INTERVAL_MS, fetchSnapshot } from '$lib/api';
  import { dragScroll } from '$lib/actions/dragScroll';
  import ServiceCard from '$lib/components/ServiceCard.svelte';
  import { getLocale, setLocale, t } from '$lib/i18n.svelte';
  import { filterServices } from '$lib/search';
  import { formatClock } from '$lib/time';
  import type { ApiTarget, Service, Snapshot } from '$lib/types';

  interface ProjectGroup {
    key: string;
    name: string;
    branch?: string;
    apiTargets: ApiTarget[];
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
        apiTargets: project?.apiTargets ?? [],
        services: [],
      };
      group.services.push(service);
      groups.set(key, group);
    }
    return [...groups.values()];
  }

  let snapshot = $state<Snapshot | null>(null);
  let failed = $state(false);
  let query = $state('');

  const services = $derived(snapshot?.services ?? []);
  const visibleServices = $derived(filterServices(services, query));
  const projectGroups = $derived(groupServices(visibleServices));
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

  {#if failed}
    <p class="status error" role="alert">{t('scanFailed')}</p>
  {:else if serviceCount === 0}
    <p class="status">{t('empty')}</p>
  {:else if visibleServices.length === 0}
    <p class="status">{t('noMatches')}</p>
  {:else}
    <div class="groups">
      {#each projectGroups as group (group.key)}
        <section class="project-group" aria-label={group.name}>
          <div class="project-header">
            <span class="project-name">{group.name}</span>
            {#if group.branch}<span class="branch">· {group.branch}</span>{/if}
          </div>
          {#if group.apiTargets.length > 0}
            <div class="api-targets">
              {#each group.apiTargets as target}
                <span>{t('apiTarget', { host: target.host, port: target.port })}</span>
              {/each}
            </div>
          {/if}
          <ul class="list" use:dragScroll>
            {#each group.services as service (service.pid)}
              <ServiceCard {service} {onstop} />
            {/each}
          </ul>
        </section>
      {/each}
    </div>
    {#if updatedAt}
      <p class="status">{t('updatedAt', { time: updatedAt })}</p>
    {/if}
  {/if}
</div>

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
    transform: rotate(-0.6deg);
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

  .api-targets {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.45rem;
    padding: 0.2rem 0.45rem;
    border: 1px solid rgba(36, 31, 26, 0.2);
    background: var(--paper-card);
    color: var(--stamp);
    font-size: 0.64rem;
  }

  .list::-webkit-scrollbar {
    display: none;
  }

  /* 서류 더미 느낌: 카드마다 미세한 기울임 (site .exhibit의 --rot 변주) */
  .list :global(li:nth-child(2n)) {
    transform: rotate(0.4deg);
  }

  .list :global(li:nth-child(2n + 1)) {
    transform: rotate(-0.3deg);
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
    transform: rotate(-0.4deg);
  }

  .status.error {
    color: var(--stamp);
    font-style: normal;
  }
</style>
