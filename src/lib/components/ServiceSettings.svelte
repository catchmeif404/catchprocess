<script lang="ts">
  import { buildService, startService } from '$lib/api';
  import { t } from '$lib/i18n.svelte';
  import { parseEnvironment, type ManagedService } from '$lib/view-preferences';

  interface Props { config: ManagedService; onsave: (config: ManagedService) => void; onclose: () => void; }
  let { config, onsave, onclose }: Props = $props();
  let draft = $state<ManagedService>({ key: '', name: '', cwd: '', buildCommand: '', runCommand: '', envText: '' });
  $effect(() => { draft = { ...config }; });
  let busy = $state('');
  let message = $state('');

  async function runBuild() {
    busy = 'build'; message = '';
    try { message = await buildService({ command: draft.buildCommand, cwd: draft.cwd, env: parseEnvironment(draft.envText) }); }
    catch (error) { message = error instanceof Error ? error.message : String(error); }
    finally { busy = ''; }
  }
  async function runStart() {
    busy = 'start'; message = '';
    try { const pid = await startService({ command: draft.runCommand, cwd: draft.cwd, env: parseEnvironment(draft.envText) }); message = `started · PID ${pid}`; }
    catch (error) { message = error instanceof Error ? error.message : String(error); }
    finally { busy = ''; }
  }
</script>

<div class="backdrop" role="presentation" onclick={onclose}>
  <div class="panel" role="dialog" tabindex="-1" aria-modal="true" aria-label={t('serviceSettings')} onclick={(event) => event.stopPropagation()} onkeydown={(event) => event.stopPropagation()}>
    <div class="panel-head"><h2>{t('serviceSettings')}</h2><button onclick={onclose}>×</button></div>
    <label>{t('serviceName')}<input bind:value={draft.name} /></label>
    <label>{t('workingDirectory')}<input bind:value={draft.cwd} /></label>
    <label>{t('buildCommand')}<input placeholder="./gradlew build" bind:value={draft.buildCommand} /></label>
    <label>{t('runCommand')}<input placeholder="./gradlew bootRun" bind:value={draft.runCommand} /></label>
    <label>{t('environment')}<textarea placeholder="PORT=8080\nPROFILE=local" bind:value={draft.envText}></textarea></label>
    <p class="hint">{t('environmentNote')}</p>
    {#if message}<pre class:error={message.includes('failed')}>{message}</pre>{/if}
    <div class="actions"><button class="secondary" onclick={onclose}>{t('cancel')}</button><button onclick={() => onsave(draft)}>{t('save')}</button><button disabled={!!busy || !draft.buildCommand} onclick={runBuild}>{busy === 'build' ? t('working') : t('build')}</button><button disabled={!!busy || !draft.runCommand} onclick={runStart}>{busy === 'start' ? t('working') : t('start')}</button></div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: 10; display: grid; place-items: center; padding: 20px; background: rgba(36,31,26,.35); }
  .panel { width: min(100%, 520px); max-height: 92vh; overflow: auto; padding: 20px; border: 1px solid #cdbfa6; border-radius: 10px; background: var(--paper-card); color: var(--ink); box-shadow: 0 10px 30px rgba(0,0,0,.25); font: 13px/1.4 system-ui,sans-serif; }
  .panel-head, .actions { display: flex; align-items: center; gap: 8px; }
  .panel-head { justify-content: space-between; margin-bottom: 18px; }
  h2 { margin: 0; font-size: 17px; }
  label { display: grid; gap: 5px; margin: 12px 0; font-weight: 600; }
  input, textarea { width: 100%; padding: 9px; border: 1px solid #cdbfa6; border-radius: 5px; background: #fffaf0; color: var(--ink); font: inherit; }
  textarea { min-height: 90px; resize: vertical; font-family: ui-monospace, monospace; }
  button { padding: 7px 10px; border: 1px solid #cdbfa6; border-radius: 5px; background: transparent; color: inherit; cursor: pointer; font: inherit; }
  button:hover { background: #241f1a0d; } button:disabled { opacity: .45; cursor: default; }
  .actions { justify-content: flex-end; flex-wrap: wrap; margin-top: 18px; } .secondary { margin-right: auto; }
  .hint { color: var(--ink-muted); font-size: 12px; } pre { max-height: 120px; overflow: auto; padding: 10px; background: #241f1a0a; white-space: pre-wrap; } .error { color: var(--stamp); }
</style>
