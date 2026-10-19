<!--
  @component
  The results view: summary, the top ten picks as cards, then every club in a table.
-->

<script lang="ts">
  import type { Recommendations } from './api';
  import ClubCard from './ClubCard.svelte';
  import ResultsSummary from './ResultsSummary.svelte';
  import ResultsTable from './ResultsTable.svelte';

  const TOP_PICKS = 10;

  interface Props {
    result: Recommendations;
    now?: Date;
  }

  let { result, now }: Props = $props();

  const picks = $derived(result.clubs.slice(0, TOP_PICKS));
  const balanceOn = $derived(result.options.balance_weight !== null);
</script>

<section aria-labelledby="results-heading">
  <h2 id="results-heading" class="visually-hidden">Results</h2>
  <ResultsSummary {result} {now} />

  <h3>Top picks</h3>
  {#if picks.length === 0}
    <p>No clubs matched your search.</p>
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
