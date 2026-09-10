<script lang="ts">
  // 러닝 서비스 1개를 표시하는 카드.
  // 1행: 프로젝트 이름(git 저장소명, 없으면 프로세스명) + 포트
  // 2행: 프로세스명 · PID · 브랜치 + 명령줄 요약(말줄임, 풀 텍스트는 툴팁)
  // 상태 도트는 항상 초록 — v0.1은 "리슨 중인 것만" 스냅샷에 담기므로 존재 자체가 RUNNING.
  import type { Service } from '$lib/types';
  import { t } from '$lib/i18n.svelte';

  interface Props {
    service: Service;
  }

  let { service }: Props = $props();

  // 파생 값: ":5173 :8080" 형태의 포트 라벨.
  const portLabel = $derived(service.ports.map((port) => `:${port}`).join(' '));
  const title = $derived(service.project?.name ?? service.process);
  const branch = $derived(service.project?.branch);
  const metaParts = $derived([
    service.process,
    `#${service.pid}`,
    ...(branch ? [branch] : []),
  ]);
  const ariaLabel = $derived(
    t('cardAria', {
      process: service.project?.name ?? service.process,
      ports: service.ports.join(', '),
    }),
  );
</script>

<li class="card" aria-label={ariaLabel} data-testid="service-card">
  <div class="row">
    <span class="dot" aria-hidden="true"></span>
    <span class="name">{title}</span>
    <span class="ports">{portLabel}</span>
  </div>
  <div class="meta" title={service.command || undefined}>
    <span>{metaParts.join(' · ')}</span>
    {#if service.command}
      <span class="cmd">{service.command}</span>
    {/if}
  </div>
</li>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.4rem 0.6rem;
    border-radius: 8px;
    background: rgb(255 255 255 / 6%);
    list-style: none;
  }

  .row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    min-width: 0;
  }

  .dot {
    flex: none;
    align-self: center;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #34d269;
    box-shadow: 0 0 6px rgb(52 210 105 / 70%);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .ports {
    flex: none;
    margin-left: auto;
    color: rgb(255 255 255 / 75%);
    font-variant-numeric: tabular-nums;
  }

  .meta {
    display: flex;
    gap: 0.5rem;
    min-width: 0;
    padding-left: 1.3rem;
    color: rgb(255 255 255 / 40%);
    font-size: 0.68rem;
  }

  .meta > span:first-child {
    flex: none;
  }

  .cmd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
