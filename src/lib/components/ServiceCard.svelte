<script lang="ts">
  // 러닝 서비스 1개를 표시하는 카드. 상태 도트는 항상 초록 — v0.1은
  // "리슨 중인 것만" 스냅샷에 담기므로 존재 자체가 RUNNING을 의미한다.
  import type { Service } from '$lib/types';
  import { t } from '$lib/i18n.svelte';

  interface Props {
    service: Service;
  }

  let { service }: Props = $props();

  // 파생 값: ":5173 :8080" 형태의 포트 라벨.
  const portLabel = $derived(service.ports.map((port) => `:${port}`).join(' '));

  const ariaLabel = $derived(
    t('cardAria', { process: service.process, ports: service.ports.join(', ') }),
  );
</script>

<li class="card" aria-label={ariaLabel} data-testid="service-card">
  <span class="dot" aria-hidden="true"></span>
  <span class="name">{service.process}</span>
  <span class="ports">{portLabel}</span>
  <span class="pid">#{service.pid}</span>
</li>

<style>
  .card {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    padding: 0.45rem 0.6rem;
    border-radius: 8px;
    background: rgb(255 255 255 / 6%);
    list-style: none;
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
    flex: 1;
    color: rgb(255 255 255 / 75%);
    font-variant-numeric: tabular-nums;
  }

  .pid {
    color: rgb(255 255 255 / 40%);
    font-size: 0.72rem;
  }
</style>
