<!--
  @component
  One club's Discord settings: whether it takes applications, auto-join, the ticket category and
  the leader roles, and unlinking it. The server checks with Discord on every request that the
  signed-in person can still manage the club's server, so this page only shows what it is told.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { ApiError, fetchClub } from './api';
  import { applyFeatures } from './features';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import { fetchSettings, loginUrl, saveSettings, unlinkClub, type Settings } from './applications';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { APPLICATIONS_PATH, settingsPath } from './route';
  import SiteFooter from './SiteFooter.svelte';

  interface Props {
    clubId: string;
    /** Whether the visitor just opted the club in. */
    linked?: boolean;
  }

  let { clubId, linked = false }: Props = $props();

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  type Load =
    'loading' | 'ready' | 'signed-out' | 'forbidden' | 'not-linked' | 'error' | 'unlinked';
  let load = $state<Load>('loading');
  let clubName = $state<string | null>(null);
  let settings = $state<Settings | null>(null);
  let open = $state(true);
  let autoJoin = $state(true);
  let category = $state('');
  let roles = $state<string[]>([]);
  let saving = $state(false);
  let saved = $state(false);
  let saveFailed = $state(false);
  let confirming = $state(false);
  let unlinkFailed = $state(false);

  const heading = $derived(
    t('settings.heading', { name: clubName ?? t('settings.club', { id: clubId }) }),
  );

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('settings.title');
  });

  function show(next: Settings) {
    settings = next;
    open = next.open;
    autoJoin = next.auto_join;
    category = next.category ?? '';
    roles = [...next.leader_roles];
  }

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    const loading = new AbortController();
    fetchClub(clubId, null, { signal: loading.signal }).then(
      (details) => (clubName = details.club.name),
      () => undefined,
    );
    fetchSettings(clubId, { signal: loading.signal }).then(
      (found) => {
        show(found);
        load = 'ready';
      },
      (reason) => {
        if (loading.signal.aborted) {
          return;
        }
        const status = reason instanceof ApiError ? reason.status : null;
        load =
          status === 401
            ? 'signed-out'
            : status === 403
              ? 'forbidden'
              : status === 404
                ? 'not-linked'
                : 'error';
      },
    );
    return () => {
      features.abort();
      loading.abort();
    };
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    saved = false;
    saveFailed = false;
    try {
      show(
        await saveSettings(clubId, {
          open,
          auto_join: autoJoin,
          category: category === '' ? null : category,
          leader_roles: roles,
        }),
      );
      saved = true;
    } catch {
      saveFailed = true;
    } finally {
      saving = false;
    }
  }

  async function unlink() {
    unlinkFailed = false;
    try {
      await unlinkClub(clubId);
      load = 'unlinked';
    } catch {
      unlinkFailed = true;
    } finally {
      confirming = false;
    }
  }

  function toggleRole(id: string, on: boolean) {
    roles = on ? [...roles, id] : roles.filter((role) => role !== id);
  }
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main>
  <header class="intro">
    <h1>{heading}</h1>
    <p><a href={APPLICATIONS_PATH}>{t('settings.back')}</a></p>
  </header>

  {#if load === 'loading'}
    <p class="card muted" role="status">{t('settings.loading')}</p>
  {:else if load === 'signed-out'}
    <section class="card">
      <p>{t('settings.signIn')}</p>
      <a class="provider-btn discord" href={loginUrl(settingsPath(clubId))}>
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
    </section>
  {:else if load === 'forbidden'}
    <p class="card" role="alert">{t('settings.forbidden')}</p>
  {:else if load === 'not-linked'}
    <section class="card">
      <p>{t('settings.notLinked')}</p>
      <a class="button primary" href={APPLICATIONS_PATH}>{t('settings.optIn')}</a>
    </section>
  {:else if load === 'unlinked'}
    <p class="card" role="status">{t('settings.unlinked')}</p>
  {:else if load === 'error' || settings === null}
    <p class="card" role="alert">{t('settings.loadError')}</p>
  {:else}
    {#if linked}
      <p class="notice" role="status">{t('settings.linked')}</p>
    {/if}
    {#if settings.problem}
      <p class="notice problem" role="alert">
        {t('settings.problem', { problem: settings.problem })}
      </p>
    {/if}

    <form class="card" onsubmit={save}>
      <div class="field">
        <label class="switch">
          <input type="checkbox" bind:checked={open} />
          <span class="track" aria-hidden="true"></span>
          <span>{t('settings.open')}</span>
        </label>
        <p class="hint">{t('settings.openHint')}</p>
      </div>

      <div class="field">
        <label class="switch">
          <input type="checkbox" bind:checked={autoJoin} />
          <span class="track" aria-hidden="true"></span>
          <span>{t('settings.autoJoin')}</span>
        </label>
      </div>

      <div class="field">
        <label for="category">{t('settings.category')}</label>
        <select id="category" bind:value={category} aria-describedby="category-hint">
          <option value="">{t('settings.categoryNone')}</option>
          {#each settings.categories as option (option.id)}
            <option value={option.id}>{option.name}</option>
          {/each}
        </select>
        <p id="category-hint" class="hint">{t('settings.categoryHint')}</p>
      </div>

      <fieldset class="field">
        <legend>{t('settings.leaderRoles')}</legend>
        <p class="hint">{t('settings.leaderRolesHint')}</p>
        {#if settings.roles.length === 0}
          <p class="hint">{t('settings.noRoles')}</p>
        {/if}
        {#each settings.roles as role (role.id)}
          <label class="check">
            <input
              type="checkbox"
              checked={roles.includes(role.id)}
              onchange={(event) => toggleRole(role.id, event.currentTarget.checked)}
            />
            <span>{role.name}</span>
          </label>
        {/each}
      </fieldset>

      <div class="actions">
        <button class="primary" type="submit" disabled={saving}>
          {saving ? t('settings.saving') : t('settings.save')}
        </button>
        {#if saved}<span role="status">{t('settings.saved')}</span>{/if}
        {#if saveFailed}<span class="failed" role="alert">{t('settings.saveError')}</span>{/if}
      </div>
    </form>

    <section class="card danger">
      <h2>{t('settings.unlink')}</h2>
      <p class="hint">{t('settings.unlinkHint')}</p>
      {#if confirming}
        <p role="alert">{t('settings.unlinkConfirm')}</p>
        <div class="actions">
          <button class="primary" type="button" onclick={unlink}>{t('settings.unlinkYes')}</button>
          <button type="button" onclick={() => (confirming = false)}>{t('settings.cancel')}</button>
        </div>
      {:else}
        <button type="button" onclick={() => (confirming = true)}>{t('settings.unlink')}</button>
      {/if}
      {#if unlinkFailed}<p class="failed" role="alert">{t('settings.unlinkError')}</p>{/if}
    </section>
  {/if}
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
    font-size: clamp(1.5rem, 4vw, 2.25rem);
    letter-spacing: -0.02em;
    line-height: 1.15;
    overflow-wrap: anywhere;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 1.1rem;
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

  .muted,
  .hint {
    color: var(--text-2);
    font-size: 0.875rem;
  }

  .notice {
    margin: 24px 0 0;
    padding: 12px 16px;
    background: color-mix(in srgb, var(--open) 10%, var(--card));
    border: 1px solid var(--open);
    border-radius: var(--radius);
  }

  .notice.problem {
    background: color-mix(in srgb, var(--closed) 10%, var(--card));
    border-color: var(--closed);
  }

  .field {
    display: grid;
    gap: 6px;
    justify-items: start;
    margin: 0 0 20px;
    padding: 0;
    border: 0;
  }

  legend {
    padding: 0;
    font-weight: 600;
  }

  label:not(.switch):not(.check) {
    font-weight: 600;
  }

  select {
    padding: 7px 10px;
    background: var(--card);
    border: 1px solid var(--control-border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
  }

  .hint {
    margin: 0;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .failed {
    color: var(--closed);
  }

  .danger {
    border-color: color-mix(in srgb, var(--closed) 40%, var(--border));
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
