<!--
  @component
  The page applications dashboard: what taking applications here does, what it needs, and the steps to
  set it up: sign in with Discord, pick the club, confirm, and add the bot. Discord sends
  the leader back to the club's settings page, or here with `?error=` when it did not work.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { applyFeatures } from './features';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import SiteFooter from './SiteFooter.svelte';

  interface Props {
    error?: string | null;
  }

  let { error = null }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

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

  function selectRole(_role: RoleKey) {
    // Placeholder: future leader and trainer pages will live here.
  }

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('applications.title');
  });

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
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
      {#each roles as role}
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

  @media (max-width: 640px) {
    .selector {
      padding: 0 4px;
    }
  }
</style>
