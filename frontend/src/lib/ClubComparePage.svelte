<!--
  @component
  The leader's applicant comparison: the club's chart and current members with up to 5 trainers
  placed among them, each on their own, as if they were the only one joining. It opens with the
  trainers in the address and lets the leader add and remove them. Only the club's leader can use it.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { ApiError, fetchClubCompare, type ClubCompare } from './api';
  import {
    activeApplications,
    fetchClubApplications,
    fetchMyClub,
    loginUrl,
    type Application,
  } from './applications';
  import { applyFeatures } from './features';
  import ClubTags from './ClubTags.svelte';
  import { formatGap, formatPace, formatUpdated, monthLabel } from './format';
  import { HumanCheck } from './human';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import PaceChart from './PaceChart.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import RunnerProgress from './RunnerProgress.svelte';
  import { compareRows } from './roster';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import {
    CLUB_APPLICATIONS_PATH,
    MAX_COMPARED,
    clubApplicationsPath,
    clubComparePath,
  } from './route';
  import {
    INTERVALS,
    perDay,
    rememberedRequirement,
    rememberRequirement,
    type Interval,
    type Requirement,
  } from './requirement';
  import { isTrainerId, normaliseTrainerId, parsePaceMillions } from './search';
  import SiteFooter from './SiteFooter.svelte';
  import { describeClubError, type ErrorMessage } from './states';
  import TierBadge from './TierBadge.svelte';
  import type { ChartLine } from './chart';

  interface Props {
    clubId: string;
    /** The trainers the address names, in order. */
    trainerIds?: string[];
    /** The fan requirement the address names, if any. */
    requirement?: Requirement | null;
    /** Replaceable in tests. */
    load?: typeof fetchClubCompare;
  }

  let { clubId, trainerIds = [], requirement = null, load = fetchClubCompare }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  // One colour per trainer, in the order they were added.
  const COLOURS = [
    'var(--accent)',
    'var(--candidate-2)',
    'var(--candidate-3)',
    'var(--candidate-4)',
    'var(--candidate-5)',
  ];

  /** Whether the person leads this club: only then does the comparison load. */
  type Access = 'loading' | 'ready' | 'signed-out' | 'forbidden' | 'error';
  let access = $state<Access>('loading');

  // svelte-ignore state_referenced_locally
  let ids = $state<string[]>(trainerIds.slice(0, MAX_COMPARED));
  let applications = $state<Application[]>([]);

  type PageState =
    | { phase: 'loading' }
    | { phase: 'error'; message: ErrorMessage }
    | { phase: 'done'; details: ClubCompare };
  let page = $state<PageState>({ phase: 'loading' });

  const human = new HumanCheck();
  let controller: AbortController | null = null;

  async function run(wanted: string[]) {
    controller?.abort();
    if (wanted.length === 0) {
      page = { phase: 'loading' };
      return;
    }
    const current = new AbortController();
    controller = current;
    page = { phase: 'loading' };
    try {
      const details = await load(clubId, wanted, { signal: current.signal, human });
      page = { phase: 'done', details };
    } catch (error) {
      if (!current.signal.aborted) {
        page = { phase: 'error', message: describeClubError(error) };
      }
    }
  }

  $effect(() => {
    if (access === 'ready') {
      void run([...ids]);
    }
  });

  function failed(reason: unknown): Access {
    const code = reason instanceof ApiError ? reason.status : null;
    return code === 401 ? 'signed-out' : code === 403 ? 'forbidden' : 'error';
  }

  async function open(signal: AbortSignal) {
    try {
      const mine = await fetchMyClub({ signal });
      if (!mine.club?.leader || mine.club.id !== clubId) {
        access = 'forbidden';
        return;
      }
      access = 'ready';
      // The page works without the list: trainers can still be added by ID.
      applications = await fetchClubApplications(clubId, { signal }).catch(() => []);
    } catch (reason) {
      if (!signal.aborted) {
        access = failed(reason);
      }
    }
  }

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    if (need !== null) updateAddress();
    const loading = new AbortController();
    void open(loading.signal);
    return () => {
      features.abort();
      loading.abort();
      controller?.abort();
    };
  });

  // The address follows the trainers and the requirement, so a reload or a shared link keeps both.
  function updateAddress() {
    history.replaceState(history.state, '', clubComparePath(clubId, ids, need));
  }

  function setIds(next: string[]) {
    ids = next;
    updateAddress();
  }

  // The requirement on the chart, and what the form holds.
  // The address wins; otherwise the one this browser remembers for the club.
  // svelte-ignore state_referenced_locally
  const startNeed = requirement ?? rememberedRequirement(storage, clubId);
  let need = $state<Requirement | null>(startNeed);
  let needAmount = $state(startNeed ? String(startNeed.fans / 1_000_000) : '');
  let needPer = $state<Interval>(startNeed?.per ?? 'week');
  let needInvalid = $state(false);

  function setRequirement(event: SubmitEvent) {
    event.preventDefault();
    const fans = parsePaceMillions(needAmount);
    needInvalid = fans === null;
    if (fans !== null) {
      need = { fans, per: needPer };
      rememberRequirement(storage, clubId, need);
      updateAddress();
    }
  }

  function clearRequirement() {
    need = null;
    needAmount = '';
    needInvalid = false;
    rememberRequirement(storage, clubId, null);
    updateAddress();
  }

  const names = $derived(new Map(applications.map((a) => [a.trainerId, a.trainerName])));
  // A trainer's name from their application, else their ID. A name two trainers share gets the ID.
  const labels = $derived(
    ids.map((id) => {
      const name = names.get(id) ?? t('compare.unnamed', { id });
      const shared = ids.some((other) => other !== id && (names.get(other) ?? other) === name);
      return shared ? `${name} (${id})` : name;
    }),
  );
  const unique = (list: Application[]) => [...new Map(list.map((a) => [a.trainerId, a])).values()];
  const options = $derived(
    unique(activeApplications(applications)).filter((a) => !ids.includes(a.trainerId)),
  );
  const full = $derived(ids.length >= MAX_COMPARED);

  let pick = $state('');
  let typed = $state('');
  let invalid = $state(false);

  function add(id: string) {
    if (!ids.includes(id) && ids.length < MAX_COMPARED) {
      setIds([...ids, id]);
    }
  }

  function addPicked() {
    if (pick !== '') {
      add(pick);
      pick = '';
    }
  }

  function addTyped(event: SubmitEvent) {
    event.preventDefault();
    invalid = !isTrainerId(typed);
    if (!invalid) {
      add(normaliseTrainerId(typed));
      typed = '';
    }
  }

  const lines = $derived.by<ChartLine[]>(() => {
    if (page.phase !== 'done') return [];
    const { trainers, previous } = page.details;
    return trainers.map((trainer, i) => ({
      label: labels[i] ?? String(trainer.trainer_id),
      fans: trainer.daily_fans,
      previousFans: previous?.trainers[i] ?? null,
      colour: COLOURS[i] ?? COLOURS[0]!,
    }));
  });

  // Members and the compared trainers, ranked together.
  const rows = $derived(
    page.phase === 'done'
      ? compareRows(
          page.details.members,
          page.details.trainers.map((trainer) => trainer.pace_per_day),
        )
      : [],
  );

  // How a pace stands against the requirement, or `null` without one.
  function standing(pace: number | null) {
    if (need === null || pace === null || page.phase !== 'done') return null;
    const gap = pace - perDay(need, page.details.month);
    return { meets: gap >= 0, gap: formatPace(Math.abs(gap)) };
  }

  // The member whose line the chart highlights.
  let selected = $state<number | null>(null);
  let chart = $state<HTMLElement>();

  function select(member: number) {
    selected = selected === member ? null : member;
    if (selected === null || !chart) return;
    const box = chart.getBoundingClientRect();
    if (box.top < 0 || box.bottom > window.innerHeight) {
      const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
      chart.scrollIntoView?.({ block: 'start', behavior: still ? 'auto' : 'smooth' });
    }
  }

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title =
      page.phase === 'done' ? t('compare.title', { name: page.details.club.name }) : t('app.name');
  });
</script>

{#snippet badge(state: { meets: boolean; gap: string } | null)}
  {#if state}
    <span class="badge" class:meets={state.meets} class:short={!state.meets}>
      <span aria-hidden="true">{state.meets ? '✓' : '✗'}</span>
      {t(state.meets ? 'compare.req.meets' : 'compare.req.short', { gap: state.gap })}
    </span>
  {/if}
{/snippet}

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main>
  <a class="back" href={clubApplicationsPath(clubId)}>← {t('compare.back')}</a>
  <header class="intro">
    <p class="eyebrow">{t('compare.eyebrow')}</p>
    <p class="muted">{t('compare.intro')}</p>
  </header>

  {#if access === 'loading'}
    <p class="status" role="status">{t('leader.loading')}</p>
  {:else if access === 'signed-out'}
    <section class="status">
      <p>{t('leader.signIn')}</p>
      <a class="provider-btn discord" href={loginUrl(clubComparePath(clubId, ids))}>
        {t('applications.session.signIn')}
      </a>
    </section>
  {:else if access === 'forbidden'}
    <p class="status" role="alert">
      {t('leader.forbidden')}
      <a href={CLUB_APPLICATIONS_PATH}>{t('compare.back')}</a>
    </p>
  {:else if access === 'error'}
    <p class="status" role="alert">{t('leader.loadError')}</p>
  {:else}
    <section class="add" aria-labelledby="add-heading">
      <h2 id="add-heading">{t('compare.add.heading')}</h2>
      {#if full}
        <p class="muted">{t('compare.add.full', { max: MAX_COMPARED })}</p>
      {:else}
        <div class="add-forms">
          <div class="field">
            <label for="applicant">{t('compare.add.applicant')}</label>
            <div class="row">
              <select id="applicant" bind:value={pick} disabled={options.length === 0}>
                <option value="">
                  {options.length === 0 ? t('compare.add.none') : t('compare.add.choose')}
                </option>
                {#each options as application (application.trainerId)}
                  <option value={application.trainerId}>
                    {application.trainerName} ({application.trainerId})
                  </option>
                {/each}
              </select>
              <button type="button" onclick={addPicked} disabled={pick === ''}>
                {t('compare.add.button')}
              </button>
            </div>
          </div>
          <form class="field" onsubmit={addTyped}>
            <label for="trainer-id">{t('compare.add.byId')}</label>
            <div class="row">
              <input
                id="trainer-id"
                type="text"
                inputmode="numeric"
                autocomplete="off"
                bind:value={typed}
                aria-invalid={invalid}
                aria-describedby={invalid ? 'trainer-id-error' : undefined}
              />
              <button type="submit">{t('compare.add.button')}</button>
            </div>
            {#if invalid}
              <p id="trainer-id-error" class="failed" role="alert">{t('compare.add.invalid')}</p>
            {/if}
          </form>
        </div>
      {/if}
    </section>

    <section class="add" aria-labelledby="req-heading">
      <h2 id="req-heading">{t('compare.req.heading')}</h2>
      <p class="muted">{t('compare.req.intro')}</p>
      <form class="req-form" onsubmit={setRequirement}>
        <div class="field">
          <label for="req-amount">{t('compare.req.amount')}</label>
          <input
            id="req-amount"
            type="text"
            inputmode="decimal"
            autocomplete="off"
            placeholder="8"
            bind:value={needAmount}
            aria-invalid={needInvalid}
            aria-describedby={needInvalid ? 'req-error' : undefined}
          />
        </div>
        <div class="field">
          <label for="req-per">{t('compare.req.interval')}</label>
          <select id="req-per" bind:value={needPer}>
            {#each INTERVALS as interval (interval)}
              <option value={interval}>{t(`compare.req.${interval}`)}</option>
            {/each}
          </select>
        </div>
        <div class="req-buttons">
          <button type="submit" class="primary">{t('compare.req.set')}</button>
          {#if need}
            <button type="button" onclick={clearRequirement}>{t('compare.req.clear')}</button>
          {/if}
        </div>
      </form>
      {#if needInvalid}
        <p id="req-error" class="failed" role="alert">{t('compare.req.invalid')}</p>
      {/if}
      {#if need && page.phase === 'done'}
        <p class="muted req-summary">
          {t('compare.req.summary', { pace: formatPace(perDay(need, page.details.month)) })}
        </p>
      {/if}
    </section>

    {#if ids.length === 0}
      <p class="status">{t('compare.empty')}</p>
    {:else if page.phase === 'loading'}
      <div class="status" role="status">
        <RunnerProgress finished={() => page.phase === 'done'} />
        <p class="muted">{t('compare.loading')}</p>
      </div>
    {:else if page.phase === 'error'}
      <div class="status error" role="alert">
        <p class="title">{page.message.title}</p>
        <p>{page.message.detail}</p>
        {#if page.message.canRetry}
          <button type="button" onclick={() => run([...ids])}>{t('status.tryAgain')}</button>
        {/if}
      </div>
    {:else}
      {@const { club, median_per_day: median } = page.details}
      <header class="club">
        <TierBadge tier={club.tier} />
        <div class="info">
          <h1>
            <a href={club.url} target="_blank" rel="noopener noreferrer">{club.name}</a>
          </h1>
          <ClubTags {club} />
        </div>
        {#if median !== null}
          <dl class="nums">
            <div>
              <dt>{t('clubPage.median')}</dt>
              <dd>{formatPace(median)}</dd>
            </div>
          </dl>
        {/if}
      </header>

      <div class="chart-box" bind:this={chart}>
        <PaceChart
          details={{
            members: page.details.members,
            month: page.details.month,
            previous: page.details.previous ? { members: page.details.previous.members } : null,
          }}
          {lines}
          requirementPerDay={need && page.phase === 'done'
            ? perDay(need, page.details.month)
            : null}
          highlight={selected}
        />
      </div>

      <section aria-labelledby="members-heading">
        <h2 id="members-heading">{t('clubPage.members')}</h2>
        <p class="muted meta">
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
          {#each rows as row (row.candidate === null ? `m${row.member}` : `c${row.candidate}`)}
            <li>
              {#if row.candidate !== null}
                {@const trainer = page.details.trainers[row.candidate]!}
                <div class="card candidate" style:--line={COLOURS[row.candidate]}>
                  <span class="place">{row.place ?? '–'}</span>
                  <span class="who-member">
                    <span class="member-name">
                      <span class="swatch" aria-hidden="true"></span>{labels[row.candidate]}
                    </span>
                    <span class="pace">
                      {formatPace(trainer.pace_per_day)}
                      {#if page.details.median_per_day !== null}
                        · {t('compare.vsMedian')}
                        {formatGap(trainer.pace_per_day - page.details.median_per_day)}
                      {/if}
                    </span>
                    {@render badge(standing(trainer.pace_per_day))}
                    {#if trainer.in_club}
                      <span class="note">{t('compare.alreadyMember')}</span>
                    {/if}
                  </span>
                  <button
                    type="button"
                    class="remove"
                    aria-label={t('compare.remove', { name: labels[row.candidate] ?? '' })}
                    disabled={ids.length === 1}
                    onclick={() => setIds(ids.filter((id) => id !== String(trainer.trainer_id)))}
                    >×</button
                  >
                </div>
              {:else}
                {@const member = page.details.members[row.member!]!}
                <button
                  type="button"
                  class="card"
                  class:chosen={selected === row.member}
                  aria-pressed={selected === row.member}
                  onclick={() => select(row.member!)}
                >
                  <span class="place">{row.place ?? '–'}</span>
                  <span class="who-member">
                    <span class="member-name">{member.name ?? t('clubPage.unknown')}</span>
                    <span class="pace">
                      {member.pace_per_day === null
                        ? t('clubPage.new')
                        : formatPace(member.pace_per_day)}
                    </span>
                    {@render badge(standing(member.pace_per_day))}
                  </span>
                </button>
              {/if}
            </li>
          {/each}
        </ol>
      </section>
    {/if}
  {/if}
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  main {
    display: grid;
    gap: 16px;
    max-width: 1240px;
  }

  .back {
    justify-self: start;
    font-size: 0.875rem;
    text-decoration: none;
  }

  .back:hover,
  .back:focus-visible {
    text-decoration: underline;
  }

  .eyebrow {
    margin: 0 0 4px;
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  .meta {
    margin-bottom: 10px;
    font-size: 0.8125rem;
  }

  .status,
  .add,
  .club {
    margin: 0;
    padding: 14px 16px;
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

  h2 {
    margin: 0 0 8px;
    font-size: 1rem;
  }

  .add-forms {
    display: grid;
    gap: 12px;
  }

  @media (width >= 720px) {
    .add-forms {
      grid-template-columns: 1fr 1fr;
    }
  }

  .field {
    display: grid;
    gap: 4px;
    margin: 0;
  }

  .field label {
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 700;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .row select,
  .row input {
    flex: 1;
    min-width: 0;
  }

  .req-form {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: end;
    margin-top: 8px;
  }

  .req-form .field {
    min-width: 140px;
  }

  .req-buttons {
    display: flex;
    gap: 8px;
  }

  .req-summary {
    margin-top: 8px;
    font-size: 0.8125rem;
  }

  .failed {
    margin: 0;
    color: var(--closed);
    font-size: 0.8125rem;
  }

  .club {
    display: grid;
    grid-template-columns: 48px minmax(0, 1fr) auto;
    gap: 8px 12px;
    align-items: center;
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

  dl {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 20px;
    margin: 0;
  }

  dt {
    color: var(--muted);
    font-size: 0.6875rem;
  }

  dd {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .card.candidate {
    grid-template-columns: 1.75rem minmax(0, 1fr) auto;
    cursor: default;
    background: color-mix(in srgb, var(--line) 12%, var(--card));
    border-color: color-mix(in srgb, var(--line) 50%, transparent);
    border-left: 4px solid var(--line);
  }

  .swatch {
    display: inline-block;
    width: 14px;
    height: 0;
    margin-right: 6px;
    border-top: 3px solid var(--line);
    vertical-align: middle;
  }

  .remove {
    padding: 0 8px;
    line-height: 1.4;
  }

  .badge {
    font-size: 0.75rem;
    font-weight: 600;
  }

  .badge.meets {
    color: var(--up);
  }

  .badge.short {
    color: var(--down);
  }

  .note {
    color: var(--muted);
    font-size: 0.75rem;
  }

  .chart-box {
    scroll-margin-top: 72px;
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
    cursor: pointer;
  }

  .card:hover {
    border-color: var(--border-strong);
  }

  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .card.chosen {
    background: color-mix(in srgb, var(--highlight) 14%, var(--card));
    border-color: var(--highlight);
    box-shadow: inset 3px 0 var(--highlight);
  }

  .place {
    color: var(--muted);
    font-size: 0.875rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .who-member {
    display: grid;
    min-width: 0;
  }

  .member-name {
    overflow: hidden;
    font-size: 0.875rem;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pace {
    color: var(--text-2);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }

  @media (max-width: 720px) {
    .club {
      grid-template-columns: 48px minmax(0, 1fr);
    }

    .club .nums {
      grid-column: 1 / -1;
    }
  }
</style>
