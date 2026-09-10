<script lang="ts">
  // 러닝 서비스 1개를 표시하는 카드.
  // 1행: 프로젝트 이름(git 저장소명, 없으면 프로세스명) + 포트 + 중지 버튼
  // 2행: 프로세스명 · PID · 브랜치 + 명령줄 요약(말줄임, 풀 텍스트는 툴팁)
  // 중지 버튼은 2단계 확인: 첫 클릭에 확인 상태가 되고(3초 후 자동 해제),
  // 다시 클릭해야 SIGTERM이 전송된다. 상태 도트는 항상 초록 — 스냅샷에는
  // 리슨 중인 것만 담기므로 존재 자체가 RUNNING을 의미한다.
  import type { Service } from '$lib/types';
  import { t } from '$lib/i18n.svelte';
  import { stopService } from '$lib/api';

  interface Props {
    service: Service;
    /** 중지 신호 전송 성공 후 호출된다 — 페이지가 즉시 재스캔한다. */
    onstop?: (pid: number) => void;
  }

  let { service, onstop }: Props = $props();

  // 확인 상태 자동 해제 타이머. 카드가 사라지거나 단계가 바뀌면 정리한다.
  let confirming = $state(false);
  let stopping = $state(false);
  let failed = $state(false);
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;

  // 파생 값: ":5173 :8080" 형태의 포트 라벨.
  const portLabel = $derived(service.ports.map((port) => `:${port}`).join(' '));
  const title = $derived(service.project?.name ?? service.process);
  const branch = $derived(service.project?.branch);
  const metaParts = $derived(
    failed
      ? [t('stopFailed')]
      : stopping
        ? [t('stopping')]
        : [service.process, `#${service.pid}`, ...(branch ? [branch] : [])],
  );
  const ariaLabel = $derived(
    t('cardAria', {
      process: service.project?.name ?? service.process,
      ports: service.ports.join(', '),
    }),
  );

  function resetConfirm(): void {
    confirming = false;
    clearTimeout(confirmTimer);
  }

  function handleStopClick(): void {
    if (stopping) return;
    if (!confirming) {
      // 1차 클릭: 확인 단계 진입. 3초 내 재클릭이 없으면 자동 해제.
      confirming = true;
      failed = false;
      clearTimeout(confirmTimer);
      confirmTimer = setTimeout(resetConfirm, 3000);
      return;
    }
    // 2차 클릭: 실제 종료 신호 전송.
    resetConfirm();
    stopping = true;
    void stopService(service.pid)
      .then(() => {
        onstop?.(service.pid); // 성공: 즉시 재스캔 요청, 다음 스냅샷에서 카드가 사라진다.
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
    <button
      class="stop"
      class:confirming
      type="button"
      aria-label={confirming ? t('confirmStop') : t('stop')}
      onclick={handleStopClick}
    >
      {confirming ? t('confirmStop') : '■'}
    </button>
  </div>
  <div class="meta" title={service.command || undefined}>
    <span>{metaParts.join(' · ')}</span>
    {#if !failed && !stopping && service.command}
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

  .stop {
    flex: none;
    padding: 0 0.3rem;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: rgb(255 255 255 / 35%);
    font-size: 0.55rem;
    line-height: 1.5;
    cursor: pointer;
  }

  .stop:hover {
    background: rgb(255 99 99 / 20%);
    color: #ff8a80;
  }

  .stop.confirming {
    background: rgb(255 99 99 / 85%);
    color: #fff;
    font-size: 0.6rem;
    font-weight: 700;
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
