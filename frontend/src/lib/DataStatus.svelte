<!--
  @component
  A one-line summary of the club data from `/api/v1/status`: how old it is, whether a refresh
  is running, and whether the last one failed. Polls while the tab is visible.
  `onscanready` runs whenever a newly finished scan is seen.
-->

<script lang="ts" module>
  /** How often to check `/api/v1/status` once club data is ready. */
  export const POLL_MS = 60_000;
  /** How often to check while a scan runs or before any data exists. */
  export const FAST_POLL_MS = 5_000;
</script>

<script lang="ts">
  import { onMount } from 'svelte';

  import { fetchStatus, type Status } from './api';
  import { formatUpdated } from './format';

  interface Props {
    load?: (signal: AbortSignal) => Promise<Status>;
    onscanready?: () => void;
  }

  let { load = (signal) => fetchStatus({ signal }), onscanready }: Props = $props();

  let status = $state<Status | null>(null);
  let checkedAt = $state(new Date());
  let lastFinished: string | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let controller: AbortController | null = null;

  function schedule(ms: number) {
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => void poll(), ms);
  }

  async function poll() {
    timer = null;
    if (document.hidden) {
      schedule(POLL_MS);
      return;
    }
    controller?.abort();
    const current = new AbortController();
    controller = current;
    try {
      const next = await load(current.signal);
      status = next;
      checkedAt = new Date();
      const finished = next.last_scan?.finished_at ?? null;
      if (finished !== null && finished !== lastFinished) {
        lastFinished = finished;
        onscanready?.();
      }
    } catch {
      if (current.signal.aborted) return;
    }
    const busy = status === null || status.scanning || status.last_scan === null;
    schedule(busy ? FAST_POLL_MS : POLL_MS);
  }

  function onvisibility() {
    if (!document.hidden) {
      void poll();
    }
  }

  onMount(() => {
    void poll();
    document.addEventListener('visibilitychange', onvisibility);
    return () => {
      document.removeEventListener('visibilitychange', onvisibility);
      if (timer !== null) clearTimeout(timer);
      controller?.abort();
    };
  });
</script>

{#if status}
  <p class="data-status" class:warn={status.last_error !== null} data-testid="data-status">
    {#if status.last_scan === null}
      {status.scanning ? 'Loading club data for the first time…' : "Club data isn't available yet."}
    {:else}
      Club data updated
      <time datetime={status.last_scan.finished_at} title={status.last_scan.finished_at}
        >{formatUpdated(status.last_scan.finished_at, checkedAt)}</time
      >{#if status.scanning}; refreshing now…{:else}.{/if}
    {/if}
    {#if status.last_error}
      The last refresh didn't finish ({status.last_error.message}).
    {/if}
  </p>
{/if}

<style>
  .data-status {
    margin: 0 0 16px;
    color: var(--muted);
    font-size: 0.8125rem;
  }

  .warn {
    color: var(--approval);
  }
</style>
