<!--
  @component
  The page where a club's leader takes applications through Discord. It finds the club from the
  leader's linked trainer. Until the bot is in the club's server it shows how to add it; then the
  bot's status and settings beside the applications the club has had.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { ApiError } from './api';
  import {
    fetchMyClub,
    fetchSettings,
    loginUrl,
    saveSettings,
    fetchClubApplications,
    type Application,
    type Settings,
  } from './applications';
  import ApplicationsTable from './ApplicationsTable.svelte';
  import BotPanel from './BotPanel.svelte';
  import BotSetup from './BotSetup.svelte';
  import DiscordIcon from './DiscordIcon.svelte';
  import { applyFeatures } from './features';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { CLUB_APPLICATIONS_PATH, clubApplicationsPath } from './route';
  import SiteFooter from './SiteFooter.svelte';

  interface Props {
    /** The club the address names, if any; the leader's own club replaces it. */
    clubId?: string | null;
    /** Whether the leader just added the bot. */
    linked?: boolean;
  }

  let { clubId: named = null, linked = false }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  /** `setup`: the bot isn't in the club's server yet, or the leader can't add it yet. */
  type Load = 'loading' | 'setup' | 'ready' | 'signed-out' | 'forbidden' | 'error';
  let load = $state<Load>('loading');
  /** The club the signed-in leader leads, once known. */
  let clubId = $state<string | null>(null);
  let clubName = $state<string | null>(null);
  /** The signed-in person's linked trainer, if any. */
  let trainer = $state<string | null>(null);
  /** Whether that trainer leads a club. */
  let leads = $state(false);
  let settings = $state<Settings | null>(null);
  let resumeFailed = $state(false);

  let applications = $state<Application[]>([]);

  type Status = 'off' | 'open' | 'paused' | 'problem';
  const status = $derived<Status>(
    settings === null ? 'off' : settings.problem ? 'problem' : settings.open ? 'open' : 'paused',
  );

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('leader.title');
  });

  function failed(reason: unknown): Load {
    const code = reason instanceof ApiError ? reason.status : null;
    return code === 404
      ? 'setup'
      : code === 401
        ? 'signed-out'
        : code === 403
          ? 'forbidden'
          : 'error';
  }

  async function open(signal: AbortSignal) {
    try {
      const mine = await fetchMyClub({ signal });
      trainer = mine.trainer;
      leads = mine.club?.leader === true;
      if (!leads || mine.club === null) {
        load = 'setup';
        return;
      }
      clubId = mine.club.id;
      clubName = mine.club.name;
      if (named !== clubId) {
        // The address follows the club the leader leads.
        history.replaceState(history.state, '', clubApplicationsPath(clubId, linked));
      }
      settings = await fetchSettings(clubId, { signal });
      load = 'ready';
      // The page works without the list, which shows its empty state.
      applications = await fetchClubApplications(clubId, { signal }).catch(() => []);
    } catch (reason) {
      if (!signal.aborted) {
        load = failed(reason);
      }
    }
  }

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    const loading = new AbortController();
    void open(loading.signal);
    return () => {
      features.abort();
      loading.abort();
    };
  });

  async function resume() {
    resumeFailed = false;
    if (clubId === null) {
      return;
    }
    try {
      settings = await saveSettings(clubId, { open: true });
    } catch {
      resumeFailed = true;
    }
  }

  function removed() {
    settings = null;
    load = 'setup';
  }
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<!-- Everything but the dashboard is one column, so it narrows to keep lines readable. -->
<main class:narrow={load !== 'ready'}>
  <header class="intro">
    <p class="eyebrow">{t('leader.eyebrow')}</p>
    <div class="title">
      <h1>{clubName ?? t('leader.heading')}</h1>
      {#if load === 'setup' || load === 'ready'}
        <span class="status {status}">{t(`leader.state.${status}`)}</span>
      {/if}
    </div>
  </header>

  {#if load === 'loading'}
    <p class="card muted" role="status">{t('leader.loading')}</p>
  {:else if load === 'signed-out'}
    <section class="card">
      <p>{t('leader.signIn')}</p>
      <a
        class="provider-btn discord"
        href={loginUrl(clubId === null ? CLUB_APPLICATIONS_PATH : clubApplicationsPath(clubId))}
      >
        <span class="provider-icon"><DiscordIcon /></span>{t('applications.session.signIn')}
      </a>
    </section>
  {:else if load === 'forbidden'}
    <p class="card" role="alert">{t('leader.forbidden')}</p>
  {:else if load === 'error'}
    <p class="card" role="alert">{t('leader.loadError')}</p>
  {:else if load === 'setup'}
    <div class="stack">
      <BotSetup {clubId} {clubName} {trainer} {leads} />
    </div>
  {:else if settings !== null && clubId !== null}
    <div class="split">
      <aside class="stack">
        <BotPanel
          {clubId}
          {clubName}
          {settings}
          onsaved={(next) => (settings = next)}
          onremoved={removed}
        />
      </aside>
      <div class="stack">
        {#if linked}
          <p class="notice" role="status">{t('leader.linked')}</p>
        {/if}
        {#if settings.problem}
          <p class="notice problem" role="alert">{t('leader.callout.problem')}</p>
        {:else if !settings.open}
          <div class="notice paused" role="status">
            <span>{t('leader.callout.paused')}</span>
            <button type="button" onclick={resume}>{t('leader.callout.resume')}</button>
          </div>
          {#if resumeFailed}<p class="failed" role="alert">{t('leader.settings.saveError')}</p>{/if}
        {/if}
        <ApplicationsTable {applications} {clubName} {clubId} />
      </div>
    </div>
  {/if}
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  main {
    display: grid;
    gap: 20px;
    max-width: 1240px;
  }

  main.narrow {
    max-width: 960px;
  }

  .eyebrow {
    margin: 0;
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .title {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
  }

  h1 {
    margin: 2px 0;
    font-size: 1.6rem;
  }

  .status {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    padding: 3px 10px;
    border: 1px solid currentColor;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 700;
  }

  .status::before {
    content: '';
    width: 8px;
    height: 8px;
    background: currentColor;
    border-radius: 50%;
  }

  .status.open {
    color: var(--open);
  }

  .status.paused {
    color: var(--approval);
  }

  .status.problem {
    color: var(--closed);
  }

  .status.off {
    color: var(--muted);
  }

  .card {
    margin: 0;
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .card p {
    margin-top: 0;
  }

  .muted {
    color: var(--muted);
  }

  .stack {
    display: grid;
    gap: 16px;
    align-content: start;
  }

  .split {
    display: grid;
    gap: 16px;
    align-items: start;
  }

  @media (width >= 960px) {
    .split {
      grid-template-columns: 340px 1fr;
    }
  }

  .notice {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
    justify-content: space-between;
    margin: 0;
    padding: 12px 14px;
    border: 1px solid var(--open);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--open) 10%, transparent);
    color: var(--text);
  }

  .notice.problem {
    border-color: var(--closed);
    background: color-mix(in srgb, var(--closed) 10%, transparent);
  }

  .notice.paused {
    border-color: var(--approval);
    background: color-mix(in srgb, var(--approval) 10%, transparent);
  }

  .failed {
    margin: 0;
    color: var(--closed);
  }
</style>
