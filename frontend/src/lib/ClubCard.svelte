<!--
  @component
  One recommended club: tier, link to uma.moe, rank trend, tags, how you'd compare,
  and balance details when balance scoring is on. The top three get medal colours.
-->

<script lang="ts">
  import type { Club } from './api';
  import {
    formatPercent,
    formatPerDay,
    formatPlace,
    joinStyleLabel,
    playstyleLabel,
    rankTrend,
    spotsLabel,
  } from './format';
  import { t } from './i18n';
  import TierBadge from './TierBadge.svelte';

  interface Props {
    club: Club;
    position: number;
  }

  let { club, position }: Props = $props();

  const trend = $derived(rankTrend(club.monthly_rank, club.last_month_rank));
  const joinStyle = $derived(joinStyleLabel(club.join_style));
  const playstyle = $derived(playstyleLabel(club.policy));
  const spots = $derived(spotsLabel(club.open_spots));
  const medal = $derived(position <= 3 ? `p${position}` : '');
</script>

<article class="card {medal}" aria-labelledby="club-{club.circle_id}">
  <div class="head">
    <div class="pick" aria-label={t('club.pick', { n: position })}>{position}</div>
    <TierBadge tier={club.tier} />
    <div class="info">
      <a
        id="club-{club.circle_id}"
        class="name"
        href={club.url}
        target="_blank"
        rel="noopener noreferrer">{club.name}</a
      >
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
        {#if playstyle}<span class="tag policy">{playstyle}</span>{/if}
        {#if club.has_discord}
          <span class="tag discord" title={t('club.discordTitle')}>{t('club.discord')}</span>
        {/if}
      </div>
    </div>
  </div>

  <dl class="nums">
    <div>
      <dt>{t('club.youdBe')}</dt>
      <dd>{formatPlace(club.your_place, club.club_size)}</dd>
    </div>
    <div>
      <dt>{t('club.median')}</dt>
      <dd>{formatPerDay(club.median_per_day)}</dd>
    </div>
  </dl>

  {#if club.comment || club.balance}
    <div class="details">
      {#if club.comment}
        <p class="comment">{club.comment}</p>
      {/if}

      {#if club.balance}
        <dl class="balance">
          <div class="bar-row">
            <dt>{t('club.balance')}</dt>
            <dd>
              <span class="bar" aria-hidden="true"><i style:width="{club.balance.score}%"></i></span
              >
              {club.balance.score}/100
            </dd>
          </div>
          <div>
            <dt>{t('club.passengers')}</dt>
            <dd>{formatPlace(club.balance.passengers, club.balance.counted)}</dd>
          </div>
          <div>
            <dt>{t('club.top3Share')}</dt>
            <dd>{formatPercent(club.balance.top3_share)}</dd>
          </div>
        </dl>
      {/if}
    </div>
  {/if}
</article>

<style>
  /* Pick column width and header gap, shared with the description so it lines up with the name. */
  .card {
    --pick: 32px;
    --gap: 12px;
    display: grid;
    grid-template-areas:
      'head nums'
      'details nums';
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 16px;
    padding: 11px 15px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .card.p1 {
    border-color: color-mix(in srgb, var(--gold) 40%, transparent);
  }

  .card.p2 {
    border-color: color-mix(in srgb, var(--silver) 40%, transparent);
  }

  .card.p3 {
    border-color: color-mix(in srgb, var(--bronze) 40%, transparent);
  }

  .head {
    grid-area: head;
    display: grid;
    grid-template-columns: var(--pick) 48px minmax(0, 1fr);
    gap: var(--gap);
    align-items: center;
  }

  .pick {
    font-size: 1.25rem;
    font-weight: 700;
    text-align: center;
  }

  .p1 .pick {
    color: var(--gold);
  }

  .p2 .pick {
    color: var(--silver);
  }

  .p3 .pick {
    color: var(--bronze);
  }

  .info {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    font-size: 1rem;
    font-weight: 600;
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

  .policy {
    background: color-mix(in srgb, var(--policy) 12%, transparent);
    color: var(--policy);
  }

  .discord {
    background: color-mix(in srgb, var(--discord) 14%, transparent);
    color: var(--discord);
  }

  /* The divider on its left spans 85% of the card's content height. Fixed column widths, wide
     enough for `30 of 30` and `<0.01M/day`, keep it in line from card to card. */
  .nums {
    position: relative;
    grid-area: nums;
    display: grid;
    grid-template-columns: 4.5rem 6rem;
    gap: 16px;
    align-content: center;
    margin: 0;
    padding-left: 16px;
    text-align: right;
  }

  .nums::before {
    position: absolute;
    top: 7.5%;
    bottom: 7.5%;
    left: 0;
    border-left: 1px solid var(--border);
    content: '';
  }

  .nums dt,
  .balance dt {
    color: var(--muted);
    font-size: 0.6875rem;
  }

  .nums dd {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* The description and balance details share a row on wide screens, and stack on narrow ones. */
  .details {
    grid-area: details;
    display: flex;
    gap: 8px 24px;
    align-items: center;
    justify-content: space-between;
    margin: 8px 0 0 calc(var(--pick) + 48px + 2 * var(--gap));
  }

  .comment {
    flex: 1 1 auto;
    max-width: 80ch;
    margin: 0;
    color: var(--muted);
    font-size: 0.8125rem;
    overflow-wrap: anywhere;
  }

  .balance {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin: 0;
    font-size: 0.8125rem;
  }

  .balance div {
    display: flex;
    gap: 6px;
    align-items: baseline;
  }

  .balance dd {
    display: flex;
    gap: 6px;
    align-items: center;
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .bar {
    display: inline-block;
    width: 90px;
    height: 6px;
    overflow: hidden;
    background: var(--surface);
    border-radius: 999px;
  }

  .bar i {
    display: block;
    height: 100%;
    background: var(--open);
    border-radius: inherit;
  }

  @media (max-width: 720px) {
    .details {
      display: block;
      margin-left: 0;
    }

    .comment + .balance {
      margin-top: 8px;
    }

    .card {
      --pick: 24px;
      --gap: 8px;
      grid-template-areas:
        'head'
        'nums'
        'details';
      grid-template-columns: minmax(0, 1fr);
    }

    .nums {
      grid-template-columns: none;
      grid-auto-flow: column;
      justify-content: start;
      margin-top: 8px;
      padding: 8px 0 0;
      border-top: 1px solid var(--border);
      text-align: left;
    }

    .nums::before {
      display: none;
    }
  }
</style>
