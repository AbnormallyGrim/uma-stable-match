<!--
  @component
  A club's rank and trend, open spots, join style, playstyle and Discord tag, on one wrapping line.
-->

<script lang="ts">
  import type { ClubInfo } from './api';
  import { joinStyleLabel, playstyleLabel, rankTrend, spotsLabel } from './format';
  import { t } from './i18n';

  interface Props {
    club: ClubInfo;
  }

  let { club }: Props = $props();

  const trend = $derived(rankTrend(club.monthly_rank, club.last_month_rank));
  const joinStyle = $derived(joinStyleLabel(club.join_style));
  const playstyle = $derived(playstyleLabel(club.policy));
  const spots = $derived(spotsLabel(club.open_spots));
</script>

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

<style>
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
</style>
