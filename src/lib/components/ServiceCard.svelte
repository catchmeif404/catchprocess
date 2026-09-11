<script lang="ts">
  // 러닝 서비스 1개를 표시하는 카드 — site의 전시품(exhibit) 카드 언어를 따른다:
  // 종이 카드, 잉크 테두리, 위쪽 테이프, 스탬프 레드 포인트.
  // 1행: 프로젝트 이름(git 저장소명, 없으면 프로세스명) + 포트 + 중지 버튼
  // 2행: 프로세스명 · PID · 브랜치
  // 중지 버튼은 단일 클릭 즉시 중지: 확인 단계 없이 SIGTERM → (유예 후) SIGKILL.
  // 도트는 스탬프 레드 — 리슨 중인 것만 담긴다.
  import type { Service } from '$lib/types';
  import { t } from '$lib/i18n.svelte';
  import { stopService } from '$lib/api';
  import { serviceLabel } from '$lib/service-presentation';

  interface Props {
    service: Service;
    /** 중지 신호 전송 성공 후 호출된다 — 페이지가 즉시 재스캔한다. */
    onstop?: (pid: number) => void;
    /** 같은 프로젝트 그룹에서 해석한 API 대상 표시명. */
    apiTargetLabels?: Record<string, string>;
    /** 프로젝트명이 부모 그룹에 이미 표시될 때 카드 정보를 압축한다. */
    compact?: boolean;
    onhide?: () => void;
  }

  let { service, onstop, onhide, apiTargetLabels = {}, compact = false }: Props = $props();
  let confirming = $state(false);

  let stopping = $state(false);
  let failed = $state(false);

  // 파생 값: ":5173 :8080" 형태의 포트 라벨.
  const portLabel = $derived(service.ports.map((port) => `:${port}`).join(' '));
  const title = $derived(compact ? serviceLabel(service) : service.project?.name ?? service.process);
  const branch = $derived(service.project?.branch);
  const metaParts = $derived(
    failed
      ? [t('stopFailed')]
      : stopping
        ? [t('stopping')]
        : compact
          ? []
          : [service.process, `#${service.pid}`, ...(branch ? [branch] : [])],
  );
  const ariaLabel = $derived(
    t('cardAria', {
      process: service.project?.name ?? service.process,
      ports: service.ports.join(', '),
    }),
  );
  const connections = $derived(service.connections ?? []);
  const apiTargets = $derived(service.apiTargets ?? []);
  const databaseTargets = $derived(service.databaseTargets ?? []);

  function connectionTargetLabel(connection: NonNullable<Service['connections']>[number]): string {
    return connection.target;
  }

  function handleStopClick(): void {
    if (stopping) return;
    stopping = true;
    failed = false;
    void stopService(service.pid)
      .then(() => {
        onstop?.(service.pid); // 성공: 즉시 재스캔 요청, 스냅샷에서 카드가 사라진다.
      })
      .catch(() => {
        // OS 거부 등 실패: 카드 안에 실패 표시(다음 폴링/재클릭까지 유지).
        failed = true;
      })
      .finally(() => {
        stopping = false;
      });
  }
</script>

<li class="card" aria-label={ariaLabel} data-testid="service-card">
  <div class="row">
    <span class="dot" aria-hidden="true"></span>
    <span class="name">{title}</span>
    <span class="ports">{portLabel}</span>
    {#if onhide}<button class="hide" title={t('hide')} aria-label={t('hide')} onclick={onhide}>−</button>{/if}
    <button
      class="stop"
      class:stopping
      type="button"
      aria-label={t('stop')}
      onclick={() => confirming = !confirming}
    >
      {t('stop')}
    </button>
  </div>
  {#if confirming}
    <div class="confirmation" role="group" aria-label={t('confirmStop')}>
      <p>{t('stopNote')}</p>
      <button disabled={stopping} onclick={handleStopClick}>{t('confirmStop')}</button>
      <button onclick={() => confirming = false}>{t('cancel')}</button>
    </div>
  {/if}
  {#if metaParts.length}<div class="meta">
    <span>{metaParts.join(' · ')}</span>
  </div>{/if}
  {#if connections.length > 0}
    <div class="connections">
      {#each connections as connection}
        <span class:local={connection.local} class="connection">
          {t('connectedTo', { target: connectionTargetLabel(connection), port: connection.port })}
          <span class="connection-scope">{connection.local ? t('localConnection') : t('externalConnection')} · {t('connections')}</span>
        </span>
      {/each}
    </div>
  {/if}
  {#if apiTargets.length > 0}
    <div class="connections api-targets">
      {#each apiTargets as target}
        <span class="connection" title={`${target.host}:${target.port} — ${t('configNote')}`}>→ {apiTargetLabels[`${target.host}:${target.port}`] ?? target.host}:{target.port}<span class="connection-scope">{t('configBadge')}</span></span>
      {/each}
    </div>
  {/if}
  {#each databaseTargets as database}
    <div class="connections"><span class="connection" title={t('configNote')}>DB · {database.database}<span class="connection-scope">{database.engine} :{database.port} · {t('configBadge')}</span></span></div>
  {/each}
  <details class="meta"><summary>{t('details')}</summary><p>{service.process} · PID {service.pid}</p>{#if apiTargets.length || databaseTargets.length}<p>{t('configNote')}</p>{/if}</details>
</li>

<style>
  /* 종이 조각 카드 — site의 .exhibit 토큰 그대로 */
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.45rem 0.6rem;
    border: 1px solid rgba(36, 31, 26, 0.35);
    border-radius: 1px;
    background: var(--paper-card);
    color: var(--ink);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    list-style: none;
    font-family: var(--mono);
  }

  /* 전시품 카드 위쪽 테이프 (site .exhibit::before 동일 형태) */
  .card::before {
    position: absolute;
    top: -0.32rem;
    left: 0.9rem;
    width: 1.2rem;
    height: 0.55rem;
    background: rgba(36, 31, 26, 0.45);
    clip-path: polygon(12% 0, 88% 0, 100% 100%, 0 100%);
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
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--stamp);
    box-shadow: 0 0 0 1.5px rgba(163, 43, 43, 0.25);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 700;
  }

  .ports {
    flex: none;
    margin-left: auto;
    color: var(--ink);
    font-variant-numeric: tabular-nums;
  }

  .stop {
    flex: none;
    padding: 0 0.3rem;
    border: 1px solid transparent;
    border-radius: 2px;
    background: transparent;
    color: rgba(36, 31, 26, 0.45);
    font-size: 0.55rem;
    font-family: inherit;
    line-height: 1.5;
    cursor: pointer;
  }

  .stop:hover {
    border-color: var(--stamp);
    color: var(--stamp);
  }

  .stop.stopping {
    color: var(--stamp);
    cursor: wait;
  }

  .meta {
    padding-left: 1.15rem;
    color: var(--ink-muted);
    font-size: 0.68rem;
  }

  .connections {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.45rem;
    padding-left: 1.15rem;
    color: var(--ink-muted);
    font-size: 0.64rem;
  }

  .api-targets {
    padding: 0.2rem 0.45rem;
    border: 1px solid rgba(36, 31, 26, 0.2);
    background: var(--paper-card);
    color: var(--stamp);
  }

  .connection-scope {
    color: var(--ink-muted);
    font-size: 0.58rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .connection.local .connection-scope {
    color: var(--stamp);
  }

  .card { border: 0; border-bottom: 1px solid #d6cab4; padding: 16px; gap: 8px; box-shadow: none; }
  .row { align-items: center; gap: 10px; }
  .name { font-family: system-ui, sans-serif; font-size: 15px; }
  .ports { font-size: 13px; color: var(--ink-muted); }
  .stop, .hide { min-height: 28px; font: 12px system-ui; padding: 4px 8px; cursor: pointer; border: 1px solid #d6cab4; border-radius: 4px; background: transparent; color: var(--ink-muted); }
  .stop:hover { color: var(--stamp); background: #a32b2b0d; }
  .connections { display: flex; flex-direction: column; gap: 6px; padding-left: 17px; font: 13px/1.5 system-ui, sans-serif; }
  .connection { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; overflow-wrap: anywhere; }
  .connection-scope { flex-shrink: 0; font: 11px/1.5 system-ui; letter-spacing: 0; text-transform: none; }
  .api-targets { border: 0; padding: 0 0 0 17px; color: var(--ink); }
  .meta { font: 12px/1.6 system-ui; padding-left: 17px; }
  summary { cursor: pointer; }
  button:focus-visible, summary:focus-visible { outline: 2px solid var(--stamp); outline-offset: 3px; }
  .confirmation { padding: 10px; background: #a32b2b0d; font: 12px system-ui; }
  .confirmation button { padding: 6px 10px; margin-right: 8px; cursor: pointer; }
</style>
