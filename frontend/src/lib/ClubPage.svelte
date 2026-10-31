<!--
  @component
  The club page: a loaded club's details and current members, fastest first, with the trainer
  added as a highlighted row where they'd rank.
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
      </p>
      <div class="scroll">
        <table>
          <thead>
            <tr>
              <th scope="col">{t('clubPage.col.place')}</th>
              <th scope="col">{t('clubPage.col.pace')}</th>
              {#if trainer}<th scope="col">{t('clubPage.col.gap')}</th>{/if}
            </tr>
          </thead>
          <tbody>
            {#each rows as row, i (i)}
              <tr class:you={row.you}>
                <td>
                  {row.place ?? '–'}
                  {#if row.you}<span class="you-tag">{t('clubPage.you')}</span>{/if}
                </td>
                <td>
                  {row.pace === null ? t('clubPage.new') : formatPace(row.pace)}
                </td>
                {#if trainer}
                  <td>{row.gap === null ? '' : formatGap(row.gap)}</td>
                {/if}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
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

  .scroll {
    max-width: 560px;
    overflow-x: auto;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8125rem;
    font-variant-numeric: tabular-nums;
  }

  th,
  td {
    padding: 6px 16px;
    border-bottom: 1px solid var(--border);
    text-align: right;
    white-space: nowrap;
  }

  th {
    color: var(--text-2);
    font-size: 0.6875rem;
    font-weight: 600;
  }

  th:first-child,
  td:first-child {
    text-align: left;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  tr.you td {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    font-weight: 600;
  }

  .you-tag {
    margin-left: 6px;
    padding: 1px 6px;
    background: var(--accent);
    border-radius: var(--radius-sm);
    color: var(--card);
    font-size: 0.6875rem;
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
