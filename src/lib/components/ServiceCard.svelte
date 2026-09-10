<script lang="ts">
  // 러닝 서비스 1개를 표시하는 카드 — site의 전시품(exhibit) 카드 언어를 따른다:
  // 종이 카드, 잉크 테두리, 위쪽 테이프, 스탬프 레드 포인트.
  // 1행: 프로젝트 이름(git 저장소명, 없으면 프로세스명) + 포트 + 중지 버튼
  // 2행: 프로세스명 · PID · 브랜치
  // 중지 버튼은 2단계 확인: 첫 클릭에 확인 상태가 되고(3초 후 자동 해제),
  // 다시 클릭해야 SIGTERM이 전송된다. 도트는 스탬프 레드 — 리슨 중인 것만 담긴다.
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
  <span class="tape" aria-hidden="true"></span>
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
  <div class="meta">
    <span>{metaParts.join(' · ')}</span>
  </div>
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
  .tape {
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

  .stop.confirming {
    background: var(--stamp);
    border-color: var(--stamp);
    color: var(--paper);
    font-size: 0.6rem;
    font-weight: 700;
  }

  .meta {
    padding-left: 1.15rem;
    color: var(--ink-muted);
    font-size: 0.68rem;
  }
</style>
