<!--
  @component
  The results view: summary, any notices, the top ten picks as cards, notes on full clubs and tiers,
  the trainer's own club when found, then the returned clubs in a table.
-->

<script lang="ts">
  import type { Recommendations } from './api';
  import ClubCard from './ClubCard.svelte';
  import ResultsSummary from './ResultsSummary.svelte';
  import ResultsTable from './ResultsTable.svelte';
  import { emptyHints, resultNotices } from './states';
  import YourClub from './YourClub.svelte';

  const TOP_PICKS = 10;

  interface Props {
    result: Recommendations;
    stale?: boolean;
  }

  let { result, stale = false }: Props = $props();

  const picks = $derived(result.clubs.slice(0, TOP_PICKS));
  const balanceOn = $derived(result.options.balance_weight !== null);
  const hints = $derived(emptyHints(result.options));
  const notices = $derived(resultNotices(result));
  const count = $derived(result.clubs.length);
</script>

<section aria-labelledby="results-heading" class:stale aria-busy={stale}>
  <h2 id="results-heading" class="visually-hidden">Results</h2>
  <ResultsSummary {result} />
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
  {/if}

  <div class="about" data-testid="results-about">
    <p>
      Full clubs (30 of 30 members) aren't in the results, since there's no spot to join. Your
      current club isn't either:
      {result.your_club
        ? "it's compared with your top pick below."
        : 'search by trainer ID to see how it compares.'}
    </p>
    <details>
      <summary>Why not a higher-tier club?</summary>
      <p>
        A club's tier comes from its total fans, which its fastest members drive. Matches are based
        on a club's typical member instead, so you land in clubs where you'd sit around the middle.
        If you're well below a high-tier club's typical member, your best fits may be a tier or two
        lower.
      </p>
    </details>
  </div>

  {#if result.your_club}
    <YourClub
      own={result.your_club}
      topPick={picks[0] ?? null}
      pace={result.trainer.pace_per_day}
      {count}
      {balanceOn}
    />
  {/if}

  {#if picks.length > 0}
    <h3>Top {count} {count === 1 ? 'result' : 'results'}</h3>
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

  .about {
    margin: 16px 0 0;
    color: var(--muted);
    font-size: 0.875rem;
    line-height: 1.5;
  }

  .about p {
    margin: 0;
  }

  .about details {
    margin-top: 6px;
  }

  .about summary {
    color: var(--text-2);
    cursor: pointer;
  }

  .about details p {
    margin-top: 4px;
  }

  .cards {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
</style>
