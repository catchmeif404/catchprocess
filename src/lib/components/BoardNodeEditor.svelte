<script lang="ts">
  // 보드 노드 추가/편집 다이얼로그. 감지된 실행 서비스에서 프리필할 수 있는 게 핵심 편의다:
  // 구성이 주도지만, 띄워 본 적 있는 서비스를 손으로 다시 타이핑하게 하지 않는다.
  import { t } from '$lib/i18n.svelte';
  import { newNode, nodeFromDetectedService, type BoardNode } from '$lib/boards';
  import { parseEnvironment } from '$lib/view-preferences';
  import type { Service } from '$lib/types';

  interface Props {
    initial?: BoardNode;
    detected: Service[];
    onsave: (node: BoardNode) => void;
    onclose: () => void;
  }

  let { initial, detected, onsave, onclose }: Props = $props();
  // initial은 마운트 시점 값만 쓴다 — 다이얼로그는 매번 새로 열리고, 편집 중에는 draft가 진실이다.
  const makeSeed = () => initial ?? newNode('service');
  let draft = $state<BoardNode>({ ...makeSeed() });
  let busy = $state(false);
  let message = $state('');

  const isService = $derived(draft.kind === 'service');

  function applyDetected(event: Event): void {
    const pid = Number((event.currentTarget as HTMLSelectElement).value);
    const source = detected.find((service) => service.pid === pid);
    if (!source) return;
    draft = { ...draft, ...nodeFromDetectedService(source) } as BoardNode;
  }

  async function testBuild(): Promise<void> {
    busy = true; message = '';
    try {
      const { buildService } = await import('$lib/api');
      message = await buildService({ command: draft.buildCommand, cwd: draft.cwd, env: parseEnvironment(draft.envText) });
    } catch (error) {
      message = error instanceof Error ? error.message : String(error);
    } finally { busy = false; }
  }
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div class="panel" role="dialog" tabindex="-1" aria-modal="true" aria-label={t('nodeEditor')} onclick={(event) => event.stopPropagation()} onkeydown={(event) => event.stopPropagation()}>
    <div class="panel-head"><h2>{initial ? t('configure') : t('addNode')}</h2><button onclick={onclose}>×</button></div>

    {#if detected.length > 0}
      <label>{t('importRunning')}
        <select onchange={applyDetected}>
          <option value="">—</option>
          {#each detected as service (service.pid)}
            <option value={service.pid}>{service.project?.name ?? service.process} · {service.process} :{service.ports.join(' :')}</option>
          {/each}
        </select>
      </label>
    {/if}

    <label>{t('nodeKind')}
      <select bind:value={draft.kind}>
        <option value="service">{t('kindService')}</option>
        <option value="database">{t('kindDatabase')}</option>
        <option value="external">{t('kindExternal')}</option>
      </select>
    </label>
    <label>{t('serviceName')}<input bind:value={draft.name} placeholder="web / api / pg" /></label>
    {#if isService}
      <label>{t('workingDirectory')}<input bind:value={draft.cwd} placeholder="~/dev/my-app" /></label>
      <label>{t('nodePort')}<input type="number" min="0" max="65535" bind:value={draft.port} /></label>
      <label>{t('buildCommand')}<input placeholder="./gradlew build" bind:value={draft.buildCommand} /></label>
      <label>{t('runCommand')}<input placeholder="npm run dev" bind:value={draft.runCommand} /></label>
      <label>{t('environment')}<textarea placeholder="PORT=8080&#10;PROFILE=local" bind:value={draft.envText}></textarea></label>
      <p class="hint">{t('environmentNote')}</p>
    {:else}
      <label>{t('endpoint')}<input bind:value={draft.endpoint} placeholder="localhost:5432" /></label>
    {/if}

    {#if message}<pre class:error={message.includes('failed')}>{message}</pre>{/if}
    <div class="actions">
      <button class="secondary" onclick={onclose}>{t('cancel')}</button>
      {#if isService && draft.buildCommand}
        <button disabled={busy} onclick={testBuild}>{busy ? t('working') : t('build')}</button>
      {/if}
      <button onclick={() => onsave(draft)}>{t('save')}</button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: 10; display: grid; place-items: center; padding: 20px; background: rgba(36,31,26,.35); }
  .panel { width: min(100%, 520px); max-height: 92vh; overflow: auto; padding: 20px; border: 1px solid #cdbfa6; border-radius: 10px; background: var(--paper-card); color: var(--ink); box-shadow: 0 10px 30px rgba(0,0,0,.25); font: 13px/1.4 system-ui,sans-serif; }
  .panel-head, .actions { display: flex; align-items: center; gap: 8px; }
  .panel-head { justify-content: space-between; margin-bottom: 18px; }
  h2 { margin: 0; font-size: 17px; }
  label { display: grid; gap: 5px; margin: 12px 0; font-weight: 600; }
  input, textarea, select { width: 100%; padding: 9px; border: 1px solid #cdbfa6; border-radius: 5px; background: #fffaf0; color: var(--ink); font: inherit; }
  select { font-weight: 400; }
  textarea { min-height: 90px; resize: vertical; font-family: ui-monospace, monospace; }
  button { padding: 7px 10px; border: 1px solid #cdbfa6; border-radius: 5px; background: transparent; color: inherit; cursor: pointer; font: inherit; }
  button:hover { background: #241f1a0d; } button:disabled { opacity: .45; cursor: default; }
  .actions { justify-content: flex-end; flex-wrap: wrap; margin-top: 18px; } .secondary { margin-right: auto; }
  .hint { color: var(--ink-muted); font-size: 12px; } pre { max-height: 120px; overflow: auto; padding: 10px; background: #241f1a0a; white-space: pre-wrap; } .error { color: var(--stamp); }
</style>
