<script lang="ts">
  import type { SearchPhase } from './runner';

  interface Props {
    status: SearchPhase;
    onretry: () => void;
  }

  let { status, onretry }: Props = $props();

  const seconds = (n: number) => `${n} ${n === 1 ? 'second' : 'seconds'}`;
</script>

{#if status.phase === 'loading'}
  <p class="status" role="status">Finding clubs…</p>
{:else if status.phase === 'waiting-for-scan'}
  <div class="status notice" role="status">
    <p class="title">Club data is still loading</p>
    <p>
      The server is scanning clubs for the first time, which takes about a minute. Trying again in
      {seconds(status.retryIn)}.
    </p>
    <button type="button" onclick={onretry}>Try now</button>
  </div>
{:else if status.phase === 'rate-limited'}
  <div class="status notice" role="status">
    <p class="title">Too many searches</p>
    {#if status.retryIn > 0}
      <p>Please wait {seconds(status.retryIn)} before searching again.</p>
    {:else}
      <p>You can search again now.</p>
    {/if}
    <button type="button" onclick={onretry} disabled={status.retryIn > 0}>Try again</button>
  </div>
{:else if status.phase === 'error'}
  <div class="status error" role="alert">
    <p class="title">{status.message.title}</p>
    <p>{status.message.detail}</p>
    {#if status.message.canRetry}
      <button type="button" onclick={onretry}>Try again</button>
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
