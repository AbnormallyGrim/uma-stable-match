<!--
  @component
  Shows the search's phase: loading, waiting for the first scan, rate limited or an error.
  `onretry` runs when the visitor chooses to retry.
-->

<script lang="ts">
  import { t } from './i18n';
  import type { SearchPhase } from './runner';

  interface Props {
    status: SearchPhase;
    onretry: () => void;
  }

  let { status, onretry }: Props = $props();

  const seconds = (n: number) => t('time.seconds', { n });
</script>

{#if status.phase === 'loading'}
  <p class="status" role="status">{t('status.loading')}</p>
{:else if status.phase === 'waiting-for-scan'}
  <div class="status notice" role="status">
    <p class="title">{t('status.waitingTitle')}</p>
    <p>{t('status.waitingBody', { wait: seconds(status.retryIn) })}</p>
    <button type="button" onclick={onretry}>{t('status.tryNow')}</button>
  </div>
{:else if status.phase === 'rate-limited'}
  <div class="status notice" role="status">
    <p class="title">{t('status.rateLimitedTitle')}</p>
    {#if status.retryIn > 0}
      <p>{t('status.rateLimitedWait', { wait: seconds(status.retryIn) })}</p>
    {:else}
      <p>{t('status.rateLimitedDone')}</p>
    {/if}
    <button type="button" onclick={onretry} disabled={status.retryIn > 0}
      >{t('status.tryAgain')}</button
    >
  </div>
{:else if status.phase === 'error'}
  <div class="status error" role="alert">
    <p class="title">{status.message.title}</p>
    <p>{status.message.detail}</p>
    {#if status.message.canRetry}
      <button type="button" onclick={onretry}>{t('status.tryAgain')}</button>
    {/if}
  </div>
{/if}

<style>
  .status {
    margin: 20px 0 0;
    padding: 12px 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-2);
  }

  p.status {
    color: var(--muted);
  }

  .status p {
    margin: 0 0 6px;
  }

  .title {
    color: var(--text);
    font-weight: 600;
  }

  .notice {
    border-color: color-mix(in srgb, var(--approval) 45%, transparent);
  }

  .error {
    border-color: color-mix(in srgb, var(--closed) 45%, transparent);
  }

  button {
    margin-top: 4px;
  }
</style>
