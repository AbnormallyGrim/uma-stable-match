<!--
  @component
  One recommended club: tier, link to uma.moe, rank trend, tags, buttons to the club page and, when
  applications are on, to quick apply, and stats on how you'd compare and, when balance scoring is on, on balance. The top
  three get medal colours.
-->

<script lang="ts">
  import type { Club } from './api';
  import { applyUrl } from './applications';
  import ClubTags from './ClubTags.svelte';
  import DiscordIcon from './DiscordIcon.svelte';
  import MembersLink from './MembersLink.svelte';
  import { flags } from './flags.svelte';
  import { formatPercent, formatPerDay, formatPlace } from './format';
  import { t } from './i18n';
  import TierBadge from './TierBadge.svelte';

  interface Props {
    club: Club;
    position: number;
    /** The trainer searched for, for the club page link, or `null` for a pace search. */
    trainerId?: string | null;
  }

  let { club, position, trainerId = null }: Props = $props();

  const medal = $derived(position <= 3 ? `p${position}` : '');
  const offId = $derived(`quick-apply-off-${club.circle_id}`);
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
      <ClubTags {club} />
    </div>
  </div>

  <div class="side">
    <MembersLink circleId={club.circle_id} name={club.name} {trainerId} />
    {#if !flags.applications}
      <!-- Quick apply is off. -->
    {:else if club.applications === 'discord'}
      <a class="quick-apply" href={applyUrl(club.circle_id)}>
        <span class="icon"><DiscordIcon /></span>{t('club.quickApply')}
      </a>
    {:else}
      <span class="quick-apply-wrap" data-tip={t('club.quickApplyOff')}>
        <button type="button" class="quick-apply" aria-disabled="true" aria-describedby={offId}>
          <span class="icon"><DiscordIcon /></span>{t('club.quickApply')}
        </button>
        <span id={offId} class="visually-hidden">{t('club.quickApplyOff')}</span>
      </span>
    {/if}
  </div>

  <div class="details">
    {#if club.comment}
      <p class="comment">{club.comment}</p>
    {/if}

    <dl class="stats">
      {#if club.balance}
        <div class="bar-row">
          <dt>{t('club.balance')}</dt>
          <dd>
            <span class="bar" aria-hidden="true"><i style:width="{club.balance.score}%"></i></span>
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
      {/if}
      <div>
        <dt>{t('club.youdBe')}</dt>
        <dd>{formatPlace(club.your_place, club.club_size)}</dd>
      </div>
      <div>
        <dt>{t('club.median')}</dt>
        <dd>{formatPerDay(club.median_per_day)}</dd>
      </div>
    </dl>
  </div>
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

  /* The club page and quick apply buttons. The divider on its left spans 85% of the card's content
     height. */
  .side {
    position: relative;
    grid-area: nums;
    display: flex;
    flex-direction: column;
    gap: 8px;
    justify-content: center;
    padding-left: 16px;
  }

  .side::before {
    position: absolute;
    top: 7.5%;
    bottom: 7.5%;
    left: 0;
    border-left: 1px solid var(--border);
    content: '';
  }

  /* A tooltip with a short, set delay: the browser's own `title` one can't be configured. The same
     text is the button's description for screen readers. */
  .quick-apply-wrap {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .quick-apply-wrap::after {
    position: absolute;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 10;
    width: max-content;
    max-width: 240px;
    padding: 4px 8px;
    background: var(--text);
    border-radius: var(--radius-sm);
    color: var(--card);
    font-size: 0.75rem;
    opacity: 0;
    pointer-events: none;
    content: attr(data-tip);
    transition: opacity 0.1s ease;
  }

  .quick-apply-wrap:hover::after,
  .quick-apply-wrap:focus-within::after {
    opacity: 1;
    transition-delay: 0.5s;
  }

  .quick-apply {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    justify-content: center;
    padding: 6px 12px;
    background: #5865f2;
    border: 1px solid #5865f2;
    border-radius: var(--radius-sm);
    color: #ffffff;
    font-size: 0.8125rem;
    font-weight: 600;
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
  }

  .quick-apply:hover {
    background: #4752c4;
    border-color: #4752c4;
  }

  .quick-apply:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .quick-apply[aria-disabled='true'] {
    background: var(--surface);
    border-color: var(--border);
    color: var(--muted);
    cursor: not-allowed;
  }

  .icon,
  .icon :global(svg) {
    display: block;
    width: 16px;
    height: 16px;
  }

  .stats dt {
    color: var(--muted);
    font-size: 0.6875rem;
  }

  /* The description and stats share a row on wide screens, and stack on narrow ones. */
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

  .stats {
    display: flex;
    flex: none;
    flex-wrap: wrap;
    gap: 6px 18px;
    margin: 0;
    font-size: 0.8125rem;
  }

  .stats div {
    display: flex;
    gap: 6px;
    align-items: baseline;
  }

  .stats dd {
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

    .comment + .stats {
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

    .side {
      align-items: stretch;
      margin-top: 8px;
      padding: 8px 0 0;
      border-top: 1px solid var(--border);
    }

    .side::before {
      display: none;
    }
  }
</style>
