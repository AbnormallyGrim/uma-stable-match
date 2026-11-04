<!--
  @component
  The page applications dashboard: what taking applications here does, what it needs, and the steps to
  set it up: sign in with Discord, pick the club, confirm, and add the bot. Discord sends
  the leader back to the club's settings page, or here with `?error=` when it did not work.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { ApiError } from './api';
  import {
    checkOptIn,
    loginUrl,
    parseClubInput,
    signOut,
    signedInUser,
  } from './applications';
  import { applyFeatures } from './features';
  import { t, hasMessage } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { APPLICATIONS_PATH, settingsPath } from './route';
  import SiteFooter from './SiteFooter.svelte';

  interface Props {
    error?: string | null;
  }

  let { error = null }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));
  let session = $state<'loading' | 'signed-out' | 'signed-in' | 'error'>('loading');
  let clubInput = $state('');
  let invalidClub = $state(false);
  let selectedClubId = $state<string | null>(null);
  let selectedClubName = $state<string | null>(null);
  let showConfirm = $state(false);
  let statusMessage = $state<string | null>(null);
  let busy = $state(false);
  const routeError = $derived(appErrorMessage(error));

  $effect(() => {
    if (error !== null && statusMessage === null) {
      statusMessage = routeError;
    }
  });

  function appErrorMessage(label: string | null): string | null {
    if (label === null) {
      return null;
    }
    const key = `applications.error.${label}` as const;
    return hasMessage(key) ? t(key) : t('applications.error.generic');
  }

  const privacyHref = '/privacy/';

  // Keep the prop available for future route/error handling without causing a state-capture warning.
  $effect(() => {
    void error;
  });

  type RoleKey = 'leader' | 'trainer';

  const roles: Array<{ key: RoleKey; icon: string; title: string; description: string }> = [
    {
      key: 'leader',
      icon: '🎯',
      title: t('applications.role.leader.title'),
      description: t('applications.role.leader.description'),
    },
    {
      key: 'trainer',
      icon: '🧭',
      title: t('applications.role.trainer.title'),
      description: t('applications.role.trainer.description'),
    },
  ];

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  function selectRole(role: RoleKey) {
    void role;
    // Placeholder: future leader and trainer pages will live here.
  }

  function resetPick(keepStatus = false) {
    clubInput = '';
    invalidClub = false;
    selectedClubId = null;
    selectedClubName = null;
    showConfirm = false;
    statusMessage = keepStatus ? statusMessage : routeError ?? null;
  }

  async function checkSession() {
    try {
      const discordId = await signedInUser();
      session = discordId === null ? 'signed-out' : 'signed-in';
      if (discordId === null) {
        resetPick();
      }
    } catch (reason) {
      console.warn(reason);
      session = 'error';
      statusMessage = t('applications.session.error');
    }
  }

  async function pickClub() {
    const parsed = parseClubInput(clubInput);
    if (parsed === null) {
      invalidClub = true;
      statusMessage = t('applications.pick.invalid');
      return;
    }

    invalidClub = false;
    statusMessage = null;
    busy = true;
    try {
      const response = await fetch(`/api/v1/clubs/${encodeURIComponent(parsed)}`);
      if (response.ok) {
        const body = (await response.json()) as { club?: { name?: string } };
        selectedClubName = body.club?.name ?? null;
      }
    } catch {
      // Ignore a missing club lookup when the user is already entering a valid ID.
    } finally {
      selectedClubId = parsed;
      selectedClubName = selectedClubName ?? t('applications.confirm.unknownClub', { id: parsed });
      showConfirm = true;
      busy = false;
    }
  }

  async function confirmClub() {
    if (!selectedClubId) {
      return;
    }

    busy = true;
    statusMessage = null;
    try {
      await checkOptIn(selectedClubId);
      const next = settingsPath(selectedClubId, true);
      window.location.assign(`/api/v1/discord/opt-in?${new URLSearchParams({ club: selectedClubId, next })}`);
    } catch (reason) {
      const status = reason instanceof ApiError ? reason.status : null;
      if (status === 401) {
        session = 'signed-out';
        statusMessage = t('applications.error.signIn');
        resetPick(true);
        return;
      }

      statusMessage =
        status === 404
          ? t('applications.error.notFound')
          : status === 422
            ? t('applications.error.noInvite')
            : status === 502
              ? t('applications.error.unavailable')
              : status === 500
                ? t('applications.error.generic')
                : status === 403
                  ? t('applications.error.forbidden')
                  : t('applications.error.generic');
    } finally {
      busy = false;
    }
  }

  async function signOutNow() {
    try {
      await signOut();
      session = 'signed-out';
      resetPick();
      statusMessage = null;
    } catch {
      session = 'signed-out';
      statusMessage = t('applications.session.error');
    }
  }

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('applications.title');
  });

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    void checkSession();
    return () => {
      features.abort();
    };
  });
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main class="page">
  <header class="intro">
    <h1>{t('applications.heading')}</h1>
    <p class="lede">{t('applications.intro')}</p>
  </header>

  <section class="selector" aria-label={t('applications.role.heading')}>
    <div class="role-grid">
      {#each roles as role (role.key)}
        <button type="button" class="role-card" onclick={() => selectRole(role.key)}>
          <span class="role-icon" aria-hidden="true">{role.icon}</span>
          <div class="role-copy">
            <h2>{role.title}</h2>
            <p>{role.description}</p>
          </div>
        </button>
      {/each}
    </div>
  </section>

  {#if error !== null}
    <p class="status-alert" role="alert">{appErrorMessage(error)}</p>
  {/if}

  <p class="privacy-link"><a href={privacyHref}>Privacy</a></p>

  {#if session === 'loading'}
    <p class="card muted" role="status">{t('applications.session.loading')}</p>
  {:else if session === 'error'}
    <p class="status-alert" role="alert">{t('applications.session.error')}</p>
  {:else if session === 'signed-out'}
    <section class="session-card">
      {#if statusMessage}
        <p role="alert">{statusMessage}</p>
      {/if}
      <p>{t('applications.session.signedOut')}</p>
      <a class="button primary" href={loginUrl(APPLICATIONS_PATH)}>{t('applications.session.signIn')}</a>
    </section>
  {:else}
    {#if !showConfirm}
      <section class="session-card pick-card">
        <label for="club-input">{t('applications.pick.label')}</label>
        <input
          id="club-input"
          bind:value={clubInput}
          aria-invalid={invalidClub ? 'true' : 'false'}
          type="text"
          autocomplete="off"
        />
        {#if statusMessage}
          <p role="alert">{statusMessage}</p>
        {/if}
        <button type="button" class="button primary" onclick={pickClub} disabled={busy}>
          {t('applications.pick.next')}
        </button>
      </section>
    {:else}
      <section class="session-card confirm-card">
        <h2>{t('applications.confirm.heading')}</h2>
        <p>{selectedClubName ?? t('applications.confirm.unknownClub', { id: selectedClubId ?? '' })}</p>
        <p>{t('applications.confirm.steps')}</p>
        {#if statusMessage}
          <p role="alert">{statusMessage}</p>
        {/if}
        <div class="actions">
          <button type="button" class="button primary" onclick={confirmClub} disabled={busy}>
            {busy ? t('applications.confirm.checking') : t('applications.confirm.add')}
          </button>
          <button type="button" onclick={() => resetPick()} disabled={busy}>
            {t('applications.confirm.back')}
          </button>
        </div>
      </section>
    {/if}

    <div class="session-actions">
      <button type="button" class="button secondary" onclick={signOutNow}>
        {t('applications.session.signOut')}
      </button>
    </div>
  {/if}
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  .page {
    padding-top: 24px;
  }

  .intro {
    max-width: 920px;
    margin: 0 auto 20px;
    text-align: center;
  }

  .intro h1 {
    margin: 0;
    font-size: clamp(2rem, 3vw, 2.75rem);
    line-height: 1.15;
  }

  .intro p {
    margin: 0 0 6px;
    color: var(--text-2);
    font-size: 0.94rem;
    line-height: 1.5;
  }

  .selector {
    max-width: 920px;
    margin: 0 auto;
  }

  .role-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 18px;
  }

  .role-card {
    display: flex;
    gap: 16px;
    width: 100%;
    padding: 18px 20px;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    box-shadow: 0 2px 12px rgb(0 0 0 / 6%);
    transition:
      border-color 0.12s ease,
      box-shadow 0.12s ease,
      transform 0.12s ease;
  }

  .role-card:hover {
    border-color: var(--accent);
    box-shadow: 0 6px 18px rgb(0 0 0 / 10%);
    transform: translateY(-1px);
  }

  .role-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 40px;
    height: 40px;
    background: var(--surface);
    border-radius: 12px;
    font-size: 1.25rem;
    line-height: 1;
  }

  .role-copy {
    flex: 1;
    min-width: 0;
  }

  .role-copy h2 {
    margin: 0 0 6px;
    font-size: 1.1rem;
    line-height: 1.3;
  }

  .role-copy p {
    margin: 0;
    color: var(--text-2);
    font-size: 0.94rem;
    line-height: 1.5;
  }

  .session-card {
    max-width: 560px;
    margin: 20px auto 0;
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 2px 20px rgb(0 0 0 / 4%);
  }

  .session-card label {
    display: block;
    margin-bottom: 8px;
    font-weight: 600;
  }

  .session-card input {
    width: 100%;
    box-sizing: border-box;
    margin-bottom: 12px;
    padding: 0.75rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    color: var(--text);
  }

  .session-card input[aria-invalid='true'] {
    border-color: var(--danger, #d9485f);
  }

  .session-card p {
    margin: 0 0 12px;
    color: var(--text-2);
    line-height: 1.5;
  }

  .button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 44px;
    padding: 0.75rem 1.05rem;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    text-decoration: none;
    cursor: pointer;
  }

  .button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .button:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  .session-actions {
    display: flex;
    justify-content: center;
    margin-top: 18px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 12px;
  }

  .status-alert {
    max-width: 560px;
    margin: 20px auto 0;
    padding: 12px 14px;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    background: var(--surface);
    color: var(--text);
  }

  .card.muted {
    max-width: 560px;
    margin: 20px auto 0;
    padding: 14px 16px;
    border-radius: 10px;
    background: var(--surface);
    color: var(--text-2);
  }

  @media (max-width: 640px) {
    .selector {
      padding: 0 4px;
    }
  }
</style>
