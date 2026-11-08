<!--
  @component
  The board of ads: trainers who are looking for a club, in a table that can be filtered and paged.
  A button opens a dialog where a trainer posts or manages their own ad, and a club's leader gets
  an Invite button on each row.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import {
    adDay,
    fetchBoard,
    fetchMyAd,
    inviteTrainer,
    LANGUAGE_CODES,
    NO_FILTERS,
    profileUrl,
    RANK_CODES,
    rankLabel,
    rankRange,
    type BoardAd,
    type BoardFilters,
    type BoardPage,
    type MyAd,
  } from './ads';
  import AdPanel from './AdPanel.svelte';
  import { ApiError } from './api';
  import { fetchMyClub, loginUrl } from './applications';
  import Dropdown from './Dropdown.svelte';
  import { applyFeatures } from './features';
  import { formatMillions, formatPerDay } from './format';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import Modal from './Modal.svelte';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { adsPath } from './route';
  import SiteFooter from './SiteFooter.svelte';

  interface Props {
    /** Whether the dialog for the trainer's own ad starts open. */
    post?: boolean;
  }

  let { post = false }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  type Load = 'loading' | 'ready' | 'error';
  let load = $state<Load>('loading');
  /** The page of ads the server sent for the filters. */
  let board = $state<BoardPage>({ ads: [], page: 1, pages: 1, total: 0 });
  /** Whether the person and their club are known, so the board can be read. */
  let started = $state(false);
  /** Bumped to read the board again with the same filters. */
  let version = $state(0);
  /** Whether reading a later page or filter failed, with the earlier ads still shown. */
  let pageFailed = $state(false);
  /** The signed-in trainer's own ad. */
  let mine = $state<MyAd | null>(null);
  /** The club the signed-in person leads, who can invite from the board. */
  let ledClub = $state<string | null>(null);
  // svelte-ignore state_referenced_locally
  let posting = $state(post);
  let filters = $state<BoardFilters>({ ...NO_FILTERS });
  let page = $state(1);
  /** The ad open in its dialog, if any. */
  let viewing = $state<BoardAd | null>(null);
  /** The height of the sticky navbar, which the table's sticky headers sit under. */
  let navHeight = $state(0);
  /** The ad being invited from, and why the last invite failed, by ad ID. */
  let inviting = $state<string | null>(null);
  let failures = $state<Record<string, string>>({});

  const filtered = $derived(
    filters.rankMin !== NO_FILTERS.rankMin ||
      filters.rankMax !== NO_FILTERS.rankMax ||
      filters.hasDiscord !== NO_FILTERS.hasDiscord ||
      filters.language !== NO_FILTERS.language,
  );

  // Each filter starts unset ("Any"), which filters nothing.
  const rankOptions = $derived<{ value: number | null; label: string }[]>([
    { value: null, label: t('ads.filter.any') },
    ...RANK_CODES.map((code) => ({ value: code, label: rankLabel(code) })),
  ]);
  const discordOptions = $derived<{ value: boolean | null; label: string }[]>([
    { value: null, label: t('ads.filter.any') },
    { value: true, label: t('ads.filter.discord.yes') },
    { value: false, label: t('ads.filter.discord.no') },
  ]);
  // Any language first, then the rest in the order an ad offers them.
  const languageOptions = $derived(
    [
      ...LANGUAGE_CODES.filter((code) => code === 'any'),
      ...LANGUAGE_CODES.filter((code) => code !== 'any'),
    ].map((code) => ({
      value: code,
      label: code === 'any' ? t('ads.filter.any') : t(`ad.language.${code}`),
    })),
  );

  // A new filter starts again from the first page.
  $effect(() => {
    void [filters.rankMin, filters.rankMax, filters.hasDiscord, filters.language];
    page = 1;
  });

  // The server filters and pages the board, so it's read again whenever either changes.
  $effect(() => {
    if (!started) {
      return;
    }
    const asked = { ...filters };
    const wanted = page;
    const club = ledClub;
    void version;
    const reading = new AbortController();
    fetchBoard(club, asked, wanted, { signal: reading.signal }).then(
      (found) => {
        board = found;
        pageFailed = false;
        load = 'ready';
        // The server keeps the page in range, for example after the ads it was on expired.
        if (found.page !== wanted) {
          page = found.page;
        }
      },
      (reason) => {
        if (reading.signal.aborted || signedOut(reason)) {
          return;
        }
        if (load === 'loading') {
          load = 'error';
        } else {
          pageFailed = true;
        }
      },
    );
    return () => reading.abort();
  });

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('ads.title');
  });

  function signedOut(reason: unknown): boolean {
    if (reason instanceof ApiError && reason.status === 401) {
      window.location.assign(loginUrl(adsPath()));
      return true;
    }
    return false;
  }

  // The navbar wraps on narrow screens, so its height is measured rather than assumed.
  onMount(() => {
    const navbar = document.querySelector<HTMLElement>('.navbar');
    if (!navbar || typeof ResizeObserver === 'undefined') {
      return;
    }
    const watch = new ResizeObserver(() => (navHeight = navbar.offsetHeight));
    watch.observe(navbar);
    return () => watch.disconnect();
  });

  /** Opens the ad's dialog from a click on its row, but not from its links and buttons. */
  function openFromRow(event: MouseEvent, ad: BoardAd) {
    if ((event.target as HTMLElement).closest('a, button')) {
      return;
    }
    viewing = ad;
  }

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    const { signal } = features;
    void (async () => {
      try {
        const [own, club] = await Promise.all([
          fetchMyAd({ signal }),
          // Anyone can look at the board; only a leader's club can invite.
          fetchMyClub({ signal }).catch((reason) => {
            if (signedOut(reason)) {
              throw reason;
            }
            return null;
          }),
        ]);
        mine = own;
        ledClub = club?.club?.leader ? club.club.id : null;
        started = true;
      } catch (reason) {
        if (!signal.aborted && !signedOut(reason)) {
          load = 'error';
        }
      }
    })();
    return () => features.abort();
  });

  /** Reads the board again after the trainer's ad changed. */
  function reload() {
    version += 1;
  }

  async function invite(ad: BoardAd) {
    if (ledClub === null) {
      return;
    }
    inviting = ad.id;
    failures = Object.fromEntries(Object.entries(failures).filter(([id]) => id !== ad.id));
    try {
      await inviteTrainer(ledClub, ad.id);
      board = {
        ...board,
        ads: board.ads.map((found) => (found.id === ad.id ? { ...found, invited: true } : found)),
      };
      if (viewing?.id === ad.id) {
        viewing = { ...viewing, invited: true };
      }
    } catch (reason) {
      const status = reason instanceof ApiError ? reason.status : null;
      failures = {
        ...failures,
        [ad.id]:
          status === 404
            ? t('ads.inviteGone')
            : status === 409
              ? t('ads.inviteRefused')
              : t('ads.inviteError'),
      };
    } finally {
      inviting = null;
    }
  }
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main>
  <header class="intro">
    <p class="eyebrow">{t('ads.eyebrow')}</p>
    <h1>{t('ads.heading')}</h1>
    <p class="lede">{t('ads.lede')}</p>
  </header>

  {#if load === 'ready'}
    <div class="post">
      <button type="button" class="primary" onclick={() => (posting = true)}>
        {mine === null ? t('ads.post') : t('ads.myAd')}
      </button>
    </div>
  {/if}

  {#if load === 'loading'}
    <p class="card muted" role="status">{t('ads.loading')}</p>
  {:else if load === 'error'}
    <p class="card" role="alert">{t('ads.loadError')}</p>
  {:else}
    <section class="card" aria-labelledby="board-heading">
      <h2 id="board-heading" class="visually-hidden">{t('ads.heading')}</h2>
      <form class="filters" role="search" aria-label={t('ads.filter.label')}>
        <div class="field">
          <span class="label" id="ads-rank-min-label">{t('ads.filter.rankMin')}</span>
          <Dropdown
            id="ads-rank-min"
            labelledby="ads-rank-min-label"
            options={rankOptions}
            bind:value={filters.rankMin}
          />
        </div>
        <div class="field">
          <span class="label" id="ads-rank-max-label">{t('ads.filter.rankMax')}</span>
          <Dropdown
            id="ads-rank-max"
            labelledby="ads-rank-max-label"
            options={rankOptions}
            bind:value={filters.rankMax}
          />
        </div>
        <div class="field">
          <span class="label" id="ads-discord-label">{t('ads.filter.discord')}</span>
          <Dropdown
            id="ads-discord"
            labelledby="ads-discord-label"
            options={discordOptions}
            bind:value={filters.hasDiscord}
          />
        </div>
        <div class="field">
          <span class="label" id="ads-language-label">{t('ads.filter.language')}</span>
          <Dropdown
            id="ads-language"
            labelledby="ads-language-label"
            options={languageOptions}
            bind:value={filters.language}
          />
        </div>
        <div class="clear">
          <button type="button" onclick={() => (filters = { ...NO_FILTERS })} disabled={!filtered}>
            {t('ads.filter.clear')}
          </button>
        </div>
      </form>

      <p class="count" role="status">{t('ads.count', { n: board.total })}</p>

      {#if pageFailed}
        <p class="problem" role="alert">{t('ads.loadError')}</p>
      {/if}

      {#if board.total === 0 && !filtered}
        <div class="empty">
          <strong>{t('ads.empty')}</strong>
          <span>{t('ads.emptyHint')}</span>
        </div>
      {:else if board.total === 0}
        <div class="empty"><strong>{t('ads.noMatch')}</strong></div>
      {:else}
        <table style:--sticky-top="{navHeight}px">
          <thead>
            <tr>
              <th scope="col">{t('ads.table.trainer')}</th>
              <th scope="col">{t('ads.table.pace')}</th>
              <th scope="col">{t('ads.table.fans')}</th>
              <th scope="col">{t('ads.table.wants')}</th>
              <th scope="col">{t('ads.table.posted')}</th>
              <th scope="col"><span class="visually-hidden">{t('ads.table.actions')}</span></th>
            </tr>
          </thead>
          <tbody>
            {#each board.ads as ad (ad.id)}
              <!-- The View button is the keyboard way in; a click anywhere on the row is a pointer shortcut. -->
              <tr class="ad" onclick={(event) => openFromRow(event, ad)}>
                <td class="trainer" data-label={t('ads.table.trainer')}>
                  <a
                    class="name"
                    href={profileUrl(ad.trainerId)}
                    target="_blank"
                    rel="noopener noreferrer">{ad.trainerName}</a
                  >
                  <span class="muted">{ad.trainerId}</span>
                  <span class="muted">{ad.currentClub ?? t('ads.table.noClub')}</span>
                </td>
                <td class="num" data-label={t('ads.table.pace')}>
                  {ad.pace === null ? t('ads.table.unknown') : formatPerDay(ad.pace)}
                </td>
                <td class="num" data-label={t('ads.table.fans')}>
                  {ad.fansThisMonth === null
                    ? t('ads.table.unknown')
                    : formatMillions(ad.fansThisMonth)}
                </td>
                <td class="wants" data-label={t('ads.table.wants')}>
                  <div class="chips">
                    <span>{rankRange(ad.rankMin, ad.rankMax)}</span>
                    {#if ad.discord !== 'any'}
                      <span class="tag">{t(`ads.table.discord.${ad.discord}`)}</span>
                    {/if}
                    {#if ad.language !== 'any'}
                      <span class="tag">{t(`ad.language.${ad.language}`)}</span>
                    {/if}
                  </div>
                </td>
                <td data-label={t('ads.table.posted')}>{adDay(ad.postedAt)}</td>
                <td class="act">
                  <div class="buttons">
                    <button
                      type="button"
                      onclick={() => (viewing = ad)}
                      aria-label={t('ads.viewAd', { name: ad.trainerName })}
                    >
                      {t('ads.view')}
                    </button>
                    {#if ledClub !== null}
                      {@render inviteButton(ad)}
                    {/if}
                  </div>
                  {#if failures[ad.id]}
                    <span class="problem" role="alert">{failures[ad.id]}</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        {#if board.pages > 1}
          <nav class="pages" aria-label={t('ads.page.label')}>
            <button
              type="button"
              onclick={() => (page = board.page - 1)}
              disabled={board.page <= 1}
            >
              {t('ads.page.prev')}
            </button>
            <span role="status"
              >{t('ads.page.status', { page: board.page, pages: board.pages })}</span
            >
            <button
              type="button"
              onclick={() => (page = board.page + 1)}
              disabled={board.page >= board.pages}
            >
              {t('ads.page.next')}
            </button>
          </nav>
        {/if}
      {/if}
    </section>
  {/if}
</main>

<SiteFooter />

{#snippet inviteButton(ad: BoardAd)}
  {#if ad.invited}
    <span class="tag invited">{t('ads.invited')}</span>
  {:else}
    <button
      type="button"
      class="primary"
      onclick={() => invite(ad)}
      disabled={inviting !== null}
      aria-label={t('ads.inviteTrainer', { name: ad.trainerName })}
    >
      {inviting === ad.id ? t('ads.inviting') : t('ads.invite')}
    </button>
  {/if}
{/snippet}

{#if viewing}
  {@const ad = viewing}
  <Modal id="board-ad" title={ad.trainerName} onclose={() => (viewing = null)}>
    <div class="details">
      <p>
        <a href={profileUrl(ad.trainerId)} target="_blank" rel="noopener noreferrer">
          {t('ads.details.profile')}
        </a>
      </p>
      <dl>
        <dt>{t('ads.details.trainerId')}</dt>
        <dd>{ad.trainerId}</dd>
        <dt>{t('ads.table.club')}</dt>
        <dd>{ad.currentClub ?? t('ads.table.noClub')}</dd>
        <dt>{t('ads.table.pace')}</dt>
        <dd>{ad.pace === null ? t('ad.numbers.unknown') : formatPerDay(ad.pace)}</dd>
        <dt>{t('ads.table.fans')}</dt>
        <dd>
          {ad.fansThisMonth === null ? t('ad.numbers.unknown') : formatMillions(ad.fansThisMonth)}
        </dd>
        <dt>{t('ad.wants.ranks')}</dt>
        <dd>{rankRange(ad.rankMin, ad.rankMax)}</dd>
        <dt>{t('ad.wants.discord')}</dt>
        <dd>{t(`ad.discord.${ad.discord}`)}</dd>
        <dt>{t('ad.wants.language')}</dt>
        <dd>{t(`ad.language.${ad.language}`)}</dd>
        <dt>{t('ads.table.posted')}</dt>
        <dd>{adDay(ad.postedAt)}</dd>
        <dt>{t('ads.details.until')}</dt>
        <dd>{adDay(ad.visibleUntil)}</dd>
      </dl>
      {#if ledClub !== null}
        <div class="buttons">{@render inviteButton(ad)}</div>
        {#if failures[ad.id]}
          <p class="problem" role="alert">{failures[ad.id]}</p>
        {/if}
      {/if}
    </div>
  </Modal>
{/if}

{#if posting}
  <Modal id="ad" title={t('ad.heading')} onclose={() => (posting = false)}>
    <AdPanel bind:ad={mine} onchange={reload} />
  </Modal>
{/if}

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  main {
    display: grid;
    gap: 20px;
    max-width: 1100px;
  }

  .eyebrow {
    margin: 0;
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  /* Between the description and the table, on the right. */
  .post {
    display: flex;
    justify-content: flex-end;
  }

  h1 {
    margin: 2px 0;
    font-size: 1.6rem;
  }

  .lede,
  .muted {
    margin: 0;
    color: var(--text-2);
  }

  .card {
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .empty {
    display: grid;
    gap: 4px;
    padding: 32px 16px;
    color: var(--muted);
    text-align: center;
  }

  .empty strong {
    color: var(--text);
  }

  /* One row of equal fields on a wide screen, like the club finder's options. */
  .filters {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 220px)) auto;
    gap: 12px 16px;
  }

  .field {
    display: grid;
    align-content: start;
    gap: 6px;
    min-width: 0;
  }

  .label {
    overflow: hidden;
    color: var(--text-2);
    font-size: 0.8125rem;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    margin: 16px 0 6px;
    color: var(--muted);
    font-size: 0.85rem;
  }

  @media (width < 960px) {
    .filters {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (width < 560px) {
    .filters {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  /* The headers stay in view under the navbar while the table scrolls. */
  th {
    position: sticky;
    z-index: 1;
    top: var(--sticky-top, 0);
    background: var(--card);
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.75rem;
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }

  td {
    padding: 10px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .trainer span,
  .act span {
    display: block;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 6px;
    align-items: center;
  }

  .tag {
    padding: 2px 8px;
    background: var(--surface);
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .invited {
    color: var(--open);
  }

  .act {
    text-align: right;
  }

  .buttons {
    display: flex;
    flex-wrap: nowrap;
    gap: 6px;
    align-items: center;
    justify-content: flex-end;
  }

  tr.ad {
    cursor: pointer;
  }

  tr.ad:hover td {
    background: var(--surface);
  }

  .name {
    font-weight: 700;
  }

  .clear {
    display: flex;
    align-items: flex-end;
  }

  .details {
    display: grid;
    gap: 14px;
  }

  .details p {
    margin: 0;
  }

  .details dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 16px;
    margin: 0;
  }

  .details dt {
    color: var(--muted);
    font-size: 0.85rem;
  }

  .details dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .details .buttons {
    justify-content: flex-end;
  }

  .problem {
    margin-top: 4px;
    color: var(--closed);
    font-size: 0.8rem;
    text-align: left;
  }

  .pages {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: center;
    margin-top: 14px;
    color: var(--text-2);
    font-size: 0.85rem;
  }

  /* Each row becomes a card, with the column names beside the values. */
  @media (width < 720px) {
    table,
    tbody,
    tr,
    td {
      display: block;
    }

    thead {
      display: none;
    }

    tr {
      padding: 12px 0;
      border-bottom: 1px solid var(--border);
    }

    tbody tr:last-child {
      border-bottom: 0;
    }

    td {
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 12px;
      padding: 3px 0;
      border: 0;
      text-align: right;
    }

    td::before {
      flex: none;
      content: attr(data-label);
      color: var(--muted);
      font-size: 0.8rem;
      text-align: left;
    }

    /* The trainer heads the card, one detail per line. */
    .trainer {
      display: grid;
      gap: 2px;
      padding-bottom: 6px;
      text-align: left;
    }

    .trainer::before {
      display: none;
    }

    .chips {
      justify-content: flex-end;
    }

    .act {
      justify-content: flex-end;
    }
  }
</style>
