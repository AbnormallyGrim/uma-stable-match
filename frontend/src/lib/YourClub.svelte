<!--
  @component
  The trainer's current club: where it would sit in the results, its median against their pace,
  and its stats side by side with the top pick's.
-->

<script lang="ts">
  import type { Club, YourClub } from './api';
  import {
    formatGap,
    formatPace,
    formatPercent,
    formatPlace,
    joinStyleLabel,
    rankTrend,
    spotsLabel,
  } from './format';
  import { t } from './i18n';
  import TierBadge from './TierBadge.svelte';

  interface Props {
    own: YourClub;
    topPick: Club | null;
    pace: number;
    count: number;
  }

  let { own, topPick, pace, count }: Props = $props();

  const verdict = $derived(
    own.result_rank === 1
      ? t('yourClub.best')
      : own.result_rank <= count
        ? t('yourClub.rank', { rank: own.result_rank })
        : t('yourClub.below', { n: count }),
  );
</script>

{#snippet panel(label: string, club: Club, testId: string)}
  {@const trend = rankTrend(club.monthly_rank, club.last_month_rank)}
  {@const joinStyle = joinStyleLabel(club.join_style)}
  {@const spots = spotsLabel(club.open_spots)}
  <div class="panel" data-testid={testId}>
    <p class="label">{label}</p>
    <div class="head">
      <TierBadge tier={club.tier} />
      <div class="info">
        <a class="name" href={club.url} target="_blank" rel="noopener noreferrer">{club.name}</a>
        <div class="line">
          {#if club.monthly_rank !== null}
            <span>
              {t('club.rank', { rank: club.monthly_rank })}
              {#if trend.label}
                <span class="trend {trend.kind}" title={trend.description}>
                  <span aria-hidden="true">{trend.label}</span>
                  <span class="visually-hidden">{trend.description}</span>
                </span>
              {/if}
            </span>
          {/if}
          {#if spots}<span>{spots}</span>{/if}
          {#if joinStyle}<span class="tag join {club.join_style}">{joinStyle}</span>{/if}
        </div>
      </div>
    </div>
    <dl class="stats">
      <div>
        <dt>{t('club.median')}</dt>
        <dd>{formatPace(club.median_per_day)}</dd>
      </div>
      <div>
        <dt>{t('yourClub.gap')}</dt>
        <dd>{formatGap(club.median_per_day - pace)}</dd>
      </div>
      <div>
        <dt>{t('club.youdBe')}</dt>
        <dd>{formatPlace(club.your_place, club.club_size)}</dd>
      </div>
      {#if club.balance}
        <div>
          <dt>{t('club.balance')}</dt>
          <dd>{club.balance.score}/100</dd>
        </div>
        <div>
          <dt>{t('club.passengers')}</dt>
          <dd>{formatPlace(club.balance.passengers, club.balance.counted)}</dd>
        </div>
        <div>
          <dt>{t('club.top3Share')}</dt>
          <dd>{formatPercent(club.balance.top3_share)}</dd>
        </div>
      {/if}
    </dl>
  </div>
{/snippet}

<section class="your-club" aria-labelledby="your-club-heading" data-testid="your-club">
  <h3 id="your-club-heading">{t('yourClub.heading')}</h3>
  <p class="verdict">
    {verdict}
    {t('yourClub.typical', { own: formatPace(own.median_per_day), pace: formatPace(pace) })}
  </p>
  <div class="panels">
    {@render panel(t('yourClub.heading'), own, 'your-club-own')}
    {#if topPick}{@render panel(t('yourClub.topPick'), topPick, 'your-club-top')}{/if}
  </div>
</section>

<style>
  .your-club {
    margin-top: 24px;
    padding: 12px 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  h3 {
    margin: 0 0 4px;
    font-size: 1rem;
    font-weight: 650;
  }

  .verdict {
    margin: 0 0 12px;
    color: var(--text-2);
    line-height: 1.5;
  }

  /* Two panels side by side on wide screens, stacked on narrow ones. */
  .panels {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .panel {
    padding: 10px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .label {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .head {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
  }

  .info {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    color: var(--text);
    font-weight: 650;
    text-decoration: none;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name:hover,
  .name:focus-visible {
    text-decoration: underline;
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    align-items: center;
    color: var(--text-2);
    font-size: 0.8125rem;
  }

  .trend.up {
    color: var(--up);
  }

  .trend.down {
    color: var(--down);
  }

  .trend.new {
    color: var(--muted);
    font-style: italic;
  }

  .tag {
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    font-size: 0.6875rem;
    font-weight: 500;
  }

  .join.open {
    background: color-mix(in srgb, var(--open) 15%, transparent);
    color: var(--open);
  }

  .join.approval {
    background: color-mix(in srgb, var(--approval) 15%, transparent);
    color: var(--approval);
  }

  .join.closed {
    background: color-mix(in srgb, var(--closed) 15%, transparent);
    color: var(--closed);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px 16px;
    margin: 12px 0 0;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .stats dt {
    color: var(--muted);
    font-size: 0.6875rem;
  }

  .stats dd {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  @media (max-width: 720px) {
    .panels {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
