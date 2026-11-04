<!--
  @component
  The page applications dashboard: what taking applications here does, what it needs, and the steps to
  set it up: sign in with Discord, pick the club, confirm, and add the bot. Discord sends
  the leader back to the club's settings page, or here with `?error=` when it did not work.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { ApiError, fetchClub } from './api';
  import { applyFeatures } from './features';
  import { hasMessage, t, type MessageKey } from './i18n';
  import {
    checkOptIn,
    loginUrl,
    optInUrl,
    parseClubInput,
    signedInUser,
    signOut,
  } from './applications';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import { websiteUrl } from './pages';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import SiteFooter from './SiteFooter.svelte';

  interface Props {
    /** The failure label Discord's callback sent the visitor back with, if any. */
    error?: string | null;
  }

  let { error = null }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  let session = $state<'loading' | 'out' | 'in' | 'error'>('loading');
  let input = $state('');
  let invalid = $state(false);
  let clubId = $state<string | null>(null);
  let clubName = $state<string | null>(null);
  let checking = $state(false);
  let failure = $state<MessageKey | null>(null);

  const callbackError = $derived.by((): MessageKey | null => {
    if (error === null) {
      return null;
    }
    const key = `applications.error.${error}`;
    return hasMessage(key) ? key : 'applications.error.generic';
  });
  const shownError = $derived(failure ?? callbackError);
  const privacyHref = `${websiteUrl(window.location)}/privacy/`;

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('applications.title');
  });

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    const loading = new AbortController();
    signedInUser({ signal: loading.signal }).then(
      (id) => (session = id === null ? 'out' : 'in'),
      (reason) => {
        if (!loading.signal.aborted) {
          session = 'error';
          console.warn(reason);
        }
      },
    );
    return () => {
      features.abort();
      loading.abort();
    };
  });

  async function pick(event: SubmitEvent) {
    event.preventDefault();
    const id = parseClubInput(input);
    invalid = id === null;
    failure = null;
    if (id === null) {
      return;
    }
    clubId = id;
    clubName = null;
    // The name is only a courtesy, so a club that is not loaded yet or a slow answer is fine.
    fetchClub(id, null).then(
      (details) => {
        if (clubId === id) {
          clubName = details.club.name;
        }
      },
      () => undefined,
    );
  }

  function failureFor(reason: unknown): MessageKey {
    if (!(reason instanceof ApiError)) {
      return 'applications.error.generic';
    }
    switch (reason.status) {
      case 401:
        return 'applications.error.signIn';
      case 404:
        return 'applications.error.notFound';
      case 422:
        return 'applications.error.noInvite';
      case 502:
      case 503:
        return 'applications.error.unavailable';
      default:
        return 'applications.error.generic';
    }
  }

  async function add() {
    if (clubId === null) {
      return;
    }
    checking = true;
    failure = null;
    try {
      // Checking first keeps problems on this page; the redirect itself leaves for Discord.
      await checkOptIn(clubId);
      window.location.assign(optInUrl(clubId));
    } catch (reason) {
      failure = failureFor(reason);
      if (reason instanceof ApiError && reason.status === 401) {
        session = 'out';
      }
      checking = false;
    }
  }

  async function leave() {
    try {
      await signOut();
      session = 'out';
      clubId = null;
    } catch {
      failure = 'applications.error.generic';
    }
  }
</script>

<!-- eslint-disable svelte/no-at-html-tags -- first-party text from the message catalog -->
<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main>
  <header class="intro">
    <h1>{t('applications.heading')}</h1>
    <p>{t('applications.intro')}</p>
  </header>

  <section class="card" aria-labelledby="requirements">
    <h2 id="requirements" class="visually-hidden">{t('applications.link')}</h2>
    <div class="requirements">{@html t('applications.requirements')}</div>
    <p class="privacy">{@html t('applications.privacy', { href: privacyHref })}</p>
  </section>

  {#if shownError}
    <div class="notice error" role="alert">
      <strong>{t('applications.error.heading')}</strong>
      <span>{t(shownError)}</span>
    </div>
  {/if}

  <section class="card" aria-labelledby="steps">
    {#if session === 'loading'}
      <p class="muted" role="status">{t('applications.session.loading')}</p>
    {:else if session === 'error'}
      <p role="alert">{t('applications.session.error')}</p>
    {:else if session === 'out'}
      <p>{t('applications.session.signedOut')}</p>
      <a class="button primary" href={loginUrl()}>{t('applications.session.signIn')}</a>
    {:else}
      <div class="signed-in">
        <span>{t('applications.session.signedIn')}</span>
        <button type="button" onclick={leave}>{t('applications.session.signOut')}</button>
      </div>

      <h2 id="steps">{t('applications.pick.heading')}</h2>
      {#if clubId === null}
        <form onsubmit={pick} novalidate>
          <label for="club">{t('applications.pick.label')}</label>
          <input
            id="club"
            type="text"
            inputmode="text"
            autocomplete="off"
            bind:value={input}
            aria-invalid={invalid}
            aria-describedby={invalid ? 'club-hint club-error' : 'club-hint'}
          />
          <p id="club-hint" class="hint">{t('applications.pick.hint')}</p>
          {#if invalid}
            <p id="club-error" class="field-error" role="alert">{t('applications.pick.invalid')}</p>
          {/if}
          <button class="primary" type="submit">{t('applications.pick.next')}</button>
        </form>
      {:else}
        <div class="confirm">
          <h3>{t('applications.confirm.heading')}</h3>
          <p class="club">
            {t('applications.confirm.club', {
              name: clubName ?? t('applications.confirm.unknownClub', { id: clubId }),
            })}
          </p>
          <p>{t('applications.confirm.steps')}</p>
          <div class="actions">
            <button class="primary" type="button" disabled={checking} onclick={add}>
              {checking ? t('applications.confirm.checking') : t('applications.confirm.add')}
            </button>
            <button
              type="button"
              disabled={checking}
              onclick={() => {
                clubId = null;
                failure = null;
              }}
            >
              {t('applications.confirm.back')}
            </button>
          </div>
        </div>
      {/if}
    {/if}
  </section>
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  main {
    max-width: 760px;
    padding-top: 40px;
  }

  .intro {
    text-align: center;
  }

  h1 {
    margin: 0 0 8px;
    font-size: clamp(1.75rem, 4vw, 2.5rem);
    letter-spacing: -0.02em;
    line-height: 1.15;
  }

  .intro p {
    margin: 0 auto;
    color: var(--text-2);
    font-size: 1.05rem;
  }

  h2 {
    margin: 0 0 12px;
    font-size: 1.15rem;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 1rem;
  }

  .card {
    margin-top: 24px;
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 1px 2px rgb(0 0 0 / 6%);
  }

  .card p {
    margin: 0 0 12px;
  }

  .requirements :global(ul) {
    margin: 8px 0 16px;
    padding-left: 20px;
  }

  .requirements :global(li + li) {
    margin-top: 8px;
  }

  .privacy,
  .hint,
  .muted {
    color: var(--text-2);
    font-size: 0.875rem;
  }

  .notice {
    margin-top: 24px;
    padding: 12px 16px;
    background: color-mix(in srgb, var(--closed) 10%, var(--card));
    border: 1px solid var(--closed);
    border-radius: var(--radius);
  }

  .notice strong {
    display: block;
    color: var(--closed);
  }

  .signed-in {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }

  form {
    display: grid;
    gap: 8px;
    justify-items: start;
  }

  label {
    font-weight: 600;
  }

  input {
    width: min(100%, 360px);
  }

  .field-error {
    color: var(--closed);
    font-size: 0.875rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .club {
    font-weight: 600;
  }

  .button {
    display: inline-block;
    padding: 6px 14px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    font-size: 0.875rem;
    font-weight: 600;
    text-decoration: none;
  }

  .button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
</style>
