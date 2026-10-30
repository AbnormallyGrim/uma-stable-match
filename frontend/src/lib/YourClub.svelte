<!--
  @component
  The trainer's current club: where it would sit in the results, its median against their pace,
  and its median, place and fit side by side with the top pick.
-->

<script lang="ts">
  import type { Club, YourClub } from './api';
  import { formatPace, formatPlace, formatScore, joinStyleLabel, spotsLabel } from './format';
  import { t } from './i18n';
  import TierBadge from './TierBadge.svelte';

  interface Props {
    own: YourClub;
    topPick: Club | null;
    pace: number;
    count: number;
    balanceOn: boolean;
  }

  let { own, topPick, pace, count, balanceOn }: Props = $props();

  const joinStyle = $derived(joinStyleLabel(own.join_style));
  const spots = $derived(spotsLabel(own.open_spots));
  const verdict = $derived(
    own.result_rank === 1
      ? t('yourClub.best')
      : own.result_rank <= count
        ? t('yourClub.rank', { rank: own.result_rank })
        : t('yourClub.below', { n: count }),
  );
  const scoreLabel = $derived(balanceOn ? t('club.score') : t('club.fit'));
  const rows = $derived([
    {
      label: t('club.median'),
      own: formatPace(own.median_per_day),
      top: topPick && formatPace(topPick.median_per_day),
    },
    {
      label: t('club.youdBe'),
      own: formatPlace(own.your_place, own.club_size),
      top: topPick && formatPlace(topPick.your_place, topPick.club_size),
    },
    {
      label: scoreLabel,
      own: formatScore(balanceOn ? own.score : own.fit),
      top: topPick && formatScore(balanceOn ? topPick.score : topPick.fit),
    },
  ]);
</script>

<section class="your-club" aria-labelledby="your-club-heading" data-testid="your-club">
  <h3 id="your-club-heading">{t('yourClub.heading')}</h3>
  <div class="head">
    <TierBadge tier={own.tier} />
    <div class="info">
      <a class="name" href={own.url} target="_blank" rel="noopener noreferrer">{own.name}</a>
      <div class="line">
        {#if own.monthly_rank !== null}<span>{t('club.rank', { rank: own.monthly_rank })}</span
          >{/if}
        {#if spots}<span>{spots}</span>{/if}
        {#if joinStyle}<span class="tag join {own.join_style}">{joinStyle}</span>{/if}
      </div>
    </div>
  </div>

  <p class="verdict">
    {verdict}
    {t('yourClub.typical', { own: formatPace(own.median_per_day), pace: formatPace(pace) })}
  </p>

  <table>
    <thead>
      <tr>
        <td></td>
        <th scope="col">{t('yourClub.heading')}</th>
        {#if topPick}<th scope="col">{t('yourClub.topPick')}</th>{/if}
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.label)}
        <tr>
          <th scope="row">{row.label}</th>
          <td>{row.own}</td>
          {#if topPick}<td>{row.top}</td>{/if}
        </tr>
      {/each}
    </tbody>
  </table>
  {#if topPick}
    <p class="note">
      {t('yourClub.note', {
        name: topPick.name,
        score: t('yourClub.scoreWord', { balanced: balanceOn }),
      })}
    </p>
  {/if}
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
    margin: 0 0 10px;
    font-size: 1rem;
    font-weight: 650;
  }

  .head {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr);
    gap: 12px;
    align-items: center;
  }

  .info {
    min-width: 0;
  }

  .name {
    color: var(--text);
    font-weight: 650;
    text-decoration: none;
  }

  .name:hover {
    text-decoration: underline;
  }

  .line {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    margin-top: 2px;
    color: var(--text-2);
    font-size: 0.8125rem;
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

  .verdict {
    margin: 12px 0 8px;
    color: var(--text-2);
    line-height: 1.5;
  }

  table {
    width: 100%;
    max-width: 420px;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
  }

  th,
  td {
    padding: 4px 8px 4px 0;
    text-align: left;
  }

  thead th {
    color: var(--text-2);
    font-size: 0.75rem;
    font-weight: 600;
  }

  tbody th {
    color: var(--text-2);
    font-size: 0.8125rem;
    font-weight: 500;
  }

  tbody td {
    font-weight: 650;
  }

  .note {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 0.8125rem;
  }
</style>
