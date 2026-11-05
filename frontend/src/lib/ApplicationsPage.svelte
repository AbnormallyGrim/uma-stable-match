<!--
  @component
  The page for the applications dashboard: what it does, how it works, and the sign-in flow for
  club leaders and trainers.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { loginUrl, signOut, signedInUser } from './applications';
  import { applyFeatures } from './features';
  import { t, hasMessage } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { APPLICATIONS_PATH, clubApplicationsPath } from './route';
  import SiteFooter from './SiteFooter.svelte';

  interface Props {
    error?: string | null;
  }

  let { error = null }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  const privacyHref = '/privacy/';
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));
  let session = $state<'loading' | 'signed-out' | 'signed-in' | 'error'>('loading');
  let statusMessage = $state<string | null>(null);
  let showSignInLink = $state(false);

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
    if (role === 'leader') {
      window.location.assign(clubApplicationsPath(7));
    }
  }

  function appErrorMessage(label: string | null): string | null {
    if (label === null) {
      return null;
    }
    const key = `applications.error.${label}` as const;
    return hasMessage(key) ? t(key) : t('applications.error.generic');
  }

  async function checkSession() {
    try {
      const discordId = await signedInUser();
      session = discordId === null ? 'signed-out' : 'signed-in';
      if (discordId === null) {
        showSignInLink = false;
      }
    } catch (reason) {
      console.warn(reason);
      session = 'error';
      statusMessage = t('applications.session.error');
      showSignInLink = false;
    }
  }

  async function signOutNow() {
    try {
      await signOut();
      session = 'signed-out';
      showSignInLink = true;
      statusMessage = null;
    } catch {
      session = 'signed-out';
      showSignInLink = true;
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

  {#if error !== null && statusMessage === null}
    <p class="status-alert" role="alert">{appErrorMessage(error)}</p>
  {/if}

  <p class="privacy-link"><a href={privacyHref}>Privacy</a></p>

  {#if session === 'loading'}
    <p class="card muted" role="status">{t('applications.session.loading')}</p>
  {:else if session === 'error'}
    <p class="status-alert" role="alert">{t('applications.session.error')}</p>
  {:else if session === 'signed-out'}
    {#if statusMessage !== null}
      <p class="status-alert" role="alert">{statusMessage}</p>
    {/if}
    {#if showSignInLink}
      <div class="card">
        <p>{t('applications.session.signedOut')}</p>
        <a class="provider-btn discord" href={loginUrl(APPLICATIONS_PATH)}>
          <span class="provider-icon" aria-hidden="true">
            <svg
              viewBox="0 0 24 24"
              fill="currentColor"
              stroke="none"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
              width="20"
              height="20"
              aria-hidden="true"
            >
              <path
                d="M20.32 4.37A19.8 19.8 0 0 0 15.43 2.85c-.21.38-.46.9-.63 1.3a18.27 18.27 0 0 0-5.49 0c-.17-.4-.42-.92-.64-1.3A19.74 19.74 0 0 0 3.78 4.38C.69 8.96-.15 13.31.27 17.61a19.94 19.94 0 0 0 6.04 3.05c.49-.66.92-1.36 1.28-2.08-.7-.26-1.37-.59-2-.96.17-.12.33-.25.49-.38a14.25 14.25 0 0 0 11.84 0c.16.13.32.26.49.38-.63.37-1.3.7-2 .96.36.72.79 1.42 1.28 2.08a19.9 19.9 0 0 0 6.04-3.05c.5-5-.84-9.31-3.41-13.24ZM8.02 14.96c-1.18 0-2.15-1.08-2.15-2.41s.95-2.41 2.15-2.41 2.17 1.09 2.15 2.41c0 1.33-.95 2.41-2.15 2.41Zm7.96 0c-1.18 0-2.15-1.08-2.15-2.41s.95-2.41 2.15-2.41 2.17 1.09 2.15 2.41c0 1.33-.95 2.41-2.15 2.41Z"
              />
            </svg>
          </span>
          <span>{t('applications.session.signIn')}</span>
        </a>
      </div>
    {/if}
  {:else if session === 'signed-in'}
    {#if statusMessage !== null}
      <p class="status-alert" role="alert">{statusMessage}</p>
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

  .button:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  .session-actions {
    display: flex;
    justify-content: center;
    margin-top: 18px;
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
