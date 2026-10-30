<!--
  @component
  The results view: summary, any notices, the top ten picks as cards, notes on full clubs and tiers,
  the trainer's own club when found, then the returned clubs in a table.
-->

<script lang="ts">
  import type { Recommendations } from './api';
  import ClubCard from './ClubCard.svelte';
  import { t } from './i18n';
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

  const picks = $derived(result.clubs.filter((club) => club.open_spots !== 0).slice(0, TOP_PICKS));
  const balanceOn = $derived(result.options.balance_weight !== null);
  const hints = $derived(emptyHints(result.options));
  const notices = $derived(resultNotices(result));
  const count = $derived(result.clubs.length);
</script>

<section aria-labelledby="results-heading" class:stale aria-busy={stale}>
  <h2 id="results-heading" class="visually-hidden">{t('results.heading')}</h2>
  <ResultsSummary {result} />
  {#each notices as notice (notice)}
    <p class="notice" data-testid="result-notice">{notice}</p>
  {/each}

  <h3>{t('results.topPicks')}</h3>
  {#if picks.length === 0 && count > 0}
    <p>{t('results.allFull')}</p>
  {:else if picks.length === 0}
    <p>{t('results.none')}</p>
    {#if hints.length > 0}
      <p>{t('results.tryThese')}</p>
      <ul class="hints">
        {#each hints as hint (hint)}
          <li>{hint}</li>
        {/each}
      </ul>
    {/if}
  {:else}
    <ol class="cards">
      {#each picks as club, i (club.circle_id)}
        <li><ClubCard {club} position={i + 1} /></li>
      {/each}
    </ol>
  {/if}

  <div class="about" data-testid="results-about">
    <p>
      {t(result.options.include_full ? 'results.fullIncluded' : 'results.fullExcluded')}
      {t(result.your_club ? 'results.ownCompared' : 'results.ownSearch')}
    </p>
    <details>
      <summary>{t('results.whyTier')}</summary>
      <p>{t('results.whyTierBody')}</p>
    </details>
  </div>

  {#if result.your_club}
    <YourClub
      own={result.your_club}
      topPick={picks[0] ?? null}
      pace={result.trainer.pace_per_day}
      {count}
    />
  {/if}

  {#if count > 0}
    <h3>{t('results.topN', { n: count })}</h3>
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
