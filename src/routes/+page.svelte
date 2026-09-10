<script lang="ts">
  // 위젯 메인 화면: 헤더(드래그 영역 + 개수 + 갱신 시각 + 닫기)와 서비스 카드 목록.
  // 데이터 페칭은 lib/api 클라이언트에 위임하고, 여기서는 3초 폴링과 상태 표시만 담당한다.
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { SCAN_INTERVAL_MS, fetchSnapshot } from '$lib/api';
  import ServiceCard from '$lib/components/ServiceCard.svelte';
  import { getLocale, t } from '$lib/i18n.svelte';
  import { formatClock } from '$lib/time';
  import type { Snapshot } from '$lib/types';

  let snapshot = $state<Snapshot | null>(null);
  let failed = $state(false);

  const serviceCount = $derived(snapshot?.services.length ?? 0);
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
    <span class="count" data-tauri-drag-region>{t('servicesUp', { count: serviceCount })}</span>
    <button
      class="close"
      type="button"
      aria-label={t('close')}
      onclick={() => void getCurrentWindow().close()}
    >
      ×
    </button>
  </header>

  {#if failed}
    <p class="status error" role="alert">{t('scanFailed')}</p>
  {:else if snapshot && snapshot.services.length > 0}
    <ul class="list">
      {#each snapshot.services as service (service.pid)}
        <ServiceCard {service} {onstop} />
      {/each}
    </ul>
    {#if updatedAt}
      <p class="status">{t('updatedAt', { time: updatedAt })}</p>
    {/if}
  {:else}
    <p class="status">{t('empty')}</p>
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

  .close:hover {
    background: rgba(163, 43, 43, 0.12);
    color: var(--stamp);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    /* 사이드 스크롤바 숨김 — 휠/트랙패드 스크롤은 동작한다 */
    scrollbar-width: none;
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
