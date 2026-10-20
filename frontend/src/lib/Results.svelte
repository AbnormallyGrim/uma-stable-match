<!--
  @component
  The results view: summary, the top ten picks as cards, then every club in a table.
-->

<script lang="ts">
  import type { Recommendations } from './api';
  import ClubCard from './ClubCard.svelte';
  import ResultsSummary from './ResultsSummary.svelte';
  import ResultsTable from './ResultsTable.svelte';
  import { emptyHints, resultNotices } from './states';

  const TOP_PICKS = 10;

  interface Props {
    result: Recommendations;
    now?: Date;
    stale?: boolean;
  }

  let { result, now, stale = false }: Props = $props();

  const picks = $derived(result.clubs.slice(0, TOP_PICKS));
  const balanceOn = $derived(result.options.balance_weight !== null);
  const hints = $derived(emptyHints(result.options));
  const notices = $derived(resultNotices(result));
</script>

<section aria-labelledby="results-heading" class:stale aria-busy={stale}>
  <h2 id="results-heading" class="visually-hidden">Results</h2>
  <ResultsSummary {result} {now} />
  {#each notices as notice (notice)}
    <p class="notice" data-testid="result-notice">{notice}</p>
  {/each}

  <h3>Top picks</h3>
  {#if picks.length === 0}
    <p>No clubs matched your search.</p>
    {#if hints.length > 0}
      <p>You could try:</p>
      <ul class="hints">
        {#each hints as hint (hint)}
          <li>{hint}</li>
        {/each}
      </ul>
    {/if}
  {:else}
    <ol class="cards">
      {#each picks as club, i (club.circle_id)}
        <li><ClubCard {club} position={i + 1} {balanceOn} /></li>
      {/each}
    </ol>

    <h3>All results</h3>
    <ResultsTable clubs={result.clubs} {balanceOn} includeClosed={result.options.include_closed} />
  {/if}
</section>

<style>
  .notice {
    margin: 10px 0 0;
    padding: 12px 14px;
    background: var(--card);
    border: 1px solid color-mix(in srgb, var(--approval) 45%, transparent);
    border-radius: var(--radius);
    color: var(--text-2);
  }

  .stale {
    opacity: 0.55;
    transition: opacity 0.2s;
  }

  @media (prefers-reduced-motion: reduce) {
    .stale {
      transition: none;
    }
  }

  h3 {
    margin: 32px 0 12px;
    font-size: 1rem;
    font-weight: 650;
  }

  .cards {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
