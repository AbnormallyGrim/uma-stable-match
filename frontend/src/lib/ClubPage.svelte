<!--
  @component
  The club page: a loaded club's details, a chart of everyone's pace this month, and the current
  members as numbered cards, fastest first, with the trainer added as a highlighted card where
  they'd rank. Selecting a member's card highlights their line on the chart.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { fetchClub, type ClubDetails } from './api';
  import ClubTags from './ClubTags.svelte';
  import { formatGap, formatPace, formatPlace, formatUpdated, monthLabel } from './format';
  import { HumanCheck } from './human';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import PaceChart from './PaceChart.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { rosterRows } from './roster';
  import type { ClubRoute } from './route';
  import SiteFooter from './SiteFooter.svelte';
  import { describeClubError, type ErrorMessage } from './states';
  import TierBadge from './TierBadge.svelte';

  interface Props {
    route: ClubRoute;
    /** Replaceable in tests. */
    load?: typeof fetchClub;
  }

  let { route, load = fetchClub }: Props = $props();

  type PageState =
    | { phase: 'loading' }
    | { phase: 'error'; message: ErrorMessage }
    | { phase: 'done'; details: ClubDetails };

  let page = $state<PageState>({ phase: 'loading' });

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  const human = new HumanCheck();
  let controller: AbortController | null = null;

  async function run() {
    controller?.abort();
    const current = new AbortController();
    controller = current;
    page = { phase: 'loading' };
    try {
      const details = await load(route.circleId, route.trainerId, {
        signal: current.signal,
        human,
      });
      page = { phase: 'done', details };
    } catch (error) {
      if (!current.signal.aborted) {
        page = { phase: 'error', message: describeClubError(error) };
      }
    }
  }

  // Back to the search the visitor came from, when there is one.
  function back(event: MouseEvent) {
    if (document.referrer.startsWith(`${window.location.origin}/`) && history.length > 1) {
      event.preventDefault();
      history.back();
    }
  }

  const rows = $derived(page.phase === 'done' ? rosterRows(page.details) : []);

  // The member whose line the chart highlights, by index in `members`.
  let selected = $state<number | null>(null);
  let chart = $state<HTMLElement>();

  function select(member: number) {
    selected = selected === member ? null : member;
    if (selected === null || !chart) return;
    // Bring the chart into view, so the highlighted line is seen.
    const box = chart.getBoundingClientRect();
    if (box.top < 0 || box.bottom > window.innerHeight) {
      const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
      chart.scrollIntoView?.({ block: 'start', behavior: still ? 'auto' : 'smooth' });
    }
  }

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title =
      page.phase === 'done' ? t('clubPage.title', { name: page.details.club.name }) : t('app.name');
  });

  onMount(() => {
    run();
    return () => controller?.abort();
  });
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main>
  <a class="back" href="/" onclick={back}>← {t('clubPage.back')}</a>

  {#if page.phase === 'loading'}
    <p class="status" role="status">{t('clubPage.loading')}</p>
  {:else if page.phase === 'error'}
    <div class="status error" role="alert">
      <p class="title">{page.message.title}</p>
      <p>{page.message.detail}</p>
      {#if page.message.canRetry}
        <button type="button" onclick={run}>{t('status.tryAgain')}</button>
      {/if}
    </div>
  {:else}
    {@const { club, trainer, median_per_day: median } = page.details}
    <header class="club">
      <TierBadge tier={club.tier} />
      <div class="info">
        <h1>
          <a href={club.url} target="_blank" rel="noopener noreferrer">{club.name}</a>
        </h1>
        <ClubTags {club} />
      </div>
      <dl class="nums">
        {#if median !== null}
          <div>
            <dt>{t('clubPage.median')}</dt>
            <dd>{formatPace(median)}</dd>
          </div>
        {/if}
        {#if trainer}
          <div>
            <dt>{t('clubPage.yourPace')}</dt>
            <dd>{formatPace(trainer.pace_per_day)}</dd>
          </div>
          {#if trainer.your_place !== null && trainer.club_size !== null}
            <div>
              <dt>{t('clubPage.youdBe')}</dt>
              <dd>{formatPlace(trainer.your_place, trainer.club_size)}</dd>
            </div>
          {/if}
        {/if}
      </dl>
      {#if club.comment}
        <p class="comment">{club.comment}</p>
      {/if}
    </header>

    <div class="chart-box" bind:this={chart}>
      <PaceChart details={page.details} highlight={selected} />
    </div>

    {#if trainer?.in_club}
      <p class="note">{t('clubPage.inClub')}</p>
    {/if}

    <section aria-labelledby="members-heading">
      <h2 id="members-heading">{t('clubPage.members')}</h2>
      <p class="meta">
        {t('clubPage.membersNote', { n: page.details.members.length })}
        {#if page.details.month && page.details.updated_at}
          {t('clubPage.data', {
            month: monthLabel(page.details.month),
            updated: formatUpdated(page.details.updated_at),
          })}
        {/if}
        {t('clubPage.pickHint')}
      </p>
      <ol class="roster">
        {#each rows as row, i (i)}
          <li>
            {#if row.member === null}
              <div class="card you">
                <span class="place">{row.place ?? '–'}</span>
                <span class="who">
                  <span class="name">{t('clubPage.you')}</span>
                  <span class="pace">{row.pace === null ? '' : formatPace(row.pace)}</span>
                </span>
              </div>
            {:else}
              {@const member = row.member}
              <button
                type="button"
                class="card"
                class:chosen={selected === member}
                aria-pressed={selected === member}
                onclick={() => select(member)}
              >
                <span class="place">{row.place ?? '–'}</span>
                <span class="who">
                  <span class="name">{row.name ?? t('clubPage.unknown')}</span>
                  <span class="stats">
                    <span class="pace">
                      {row.pace === null ? t('clubPage.new') : formatPace(row.pace)}
                    </span>
                    {#if row.gap !== null}
                      <span class="gap">{t('clubPage.gap', { gap: formatGap(row.gap) })}</span>
                    {/if}
                  </span>
                </span>
              </button>
            {/if}
          </li>
        {/each}
      </ol>
    </section>
  {/if}
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  .back {
    display: inline-block;
    margin-bottom: 16px;
    font-size: 0.875rem;
    text-decoration: none;
  }

  .back:hover,
  .back:focus-visible {
    text-decoration: underline;
  }

  .status {
    margin: 0;
    padding: 12px 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-2);
  }

  .status p {
    margin: 0 0 6px;
  }

  .error {
    border-color: color-mix(in srgb, var(--closed) 45%, transparent);
  }

  .title {
    color: var(--text);
    font-weight: 600;
  }

  /* Clears the sticky navbar when a selection scrolls the chart into view. */
  .chart-box {
    scroll-margin-top: 72px;
  }

  .club {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr) auto;
    gap: 8px 12px;
    align-items: center;
    padding: 14px 16px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .info {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  h1 {
    margin: 0;
    overflow: hidden;
    font-size: 1.25rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  h1 a {
    color: var(--text);
    text-decoration: none;
  }

  h1 a:hover,
  h1 a:focus-visible {
    text-decoration: underline;
  }

  .nums {
    display: flex;
    gap: 20px;
    margin: 0;
    text-align: right;
  }

  .nums dt {
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

  .comment {
    grid-column: 2 / -1;
    margin: 0;
    color: var(--muted);
    font-size: 0.8125rem;
    overflow-wrap: anywhere;
  }

  .note {
    margin: 12px 0 0;
    color: var(--text-2);
    font-size: 0.875rem;
  }

  h2 {
    margin: 24px 0 2px;
    font-size: 1rem;
  }

  .meta {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 0.8125rem;
  }

  .roster {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .card {
    display: grid;
    grid-template-columns: 1.75rem minmax(0, 1fr);
    gap: 10px;
    align-items: center;
    width: 100%;
    height: 100%;
    padding: 8px 12px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font: inherit;
    text-align: left;
  }

  button.card {
    cursor: pointer;
  }

  button.card:hover {
    border-color: var(--border-strong);
  }

  button.card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .card.chosen {
    background: color-mix(in srgb, var(--highlight) 14%, var(--card));
    border-color: var(--highlight);
    box-shadow: inset 3px 0 var(--highlight);
  }

  .card.you {
    background: color-mix(in srgb, var(--accent) 14%, var(--card));
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }

  .place {
    color: var(--muted);
    font-size: 0.875rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .who {
    display: grid;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    font-size: 0.875rem;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 0 10px;
  }

  .pace,
  .gap {
    color: var(--text-2);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .gap {
    color: var(--muted);
  }

  @media (max-width: 720px) {
    .club {
      grid-template-columns: 48px minmax(0, 1fr);
    }

    .nums {
      grid-column: 1 / -1;
      justify-content: start;
      padding-top: 8px;
      border-top: 1px solid var(--border);
      text-align: left;
    }

    .comment {
      grid-column: 1 / -1;
    }
  }
</style>
