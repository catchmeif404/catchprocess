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

  $effect(() => {
    // 폴링 요청 겹침(느린 스캔 + 짧은 주기) 시 이전 요청의 늦은 응답이 최신 상태를 덮어쓰는
    // race condition을 시퀀스 번호로 차단한다 — 마지막 요청의 응답만 상태에 반영.
    let requestSeq = 0;

    const poll = async (): Promise<void> => {
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
    };

    void poll();
    const timer = setInterval(() => void poll(), SCAN_INTERVAL_MS);
    return () => clearInterval(timer);
  });
</script>

<svelte:head>
  <title>DevTopology</title>
</svelte:head>

<div class="widget">
  <!-- 프레임리스 창 드래그 영역: 헤더 전체 -->
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
        <ServiceCard {service} />
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
  .widget {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    height: 100vh;
    padding: 0.6rem;
    box-sizing: border-box;
    border-radius: 12px;
    background: rgb(18 20 24 / 88%);
    color: #f2f4f8;
    font-family:
      'SF Mono', ui-monospace, Menlo, Consolas, monospace;
    font-size: 0.8rem;
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.15rem 0.2rem 0.45rem;
    border-bottom: 1px solid rgb(255 255 255 / 12%);
    cursor: grab;
    user-select: none;
  }

  .brand {
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .count {
    flex: 1;
    color: rgb(255 255 255 / 55%);
  }

  .close {
    flex: none;
    padding: 0 0.35rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: rgb(255 255 255 / 50%);
    font-size: 0.9rem;
    line-height: 1.4;
    cursor: pointer;
  }

  .close:hover {
    background: rgb(255 255 255 / 12%);
    color: #fff;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin: 0;
    padding: 0;
    overflow-y: auto;
  }

  .status {
    margin: auto 0 0;
    padding: 0.3rem 0.2rem 0;
    color: rgb(255 255 255 / 45%);
    font-size: 0.7rem;
  }

  .status.error {
    color: #ff8a80;
  }
</style>
