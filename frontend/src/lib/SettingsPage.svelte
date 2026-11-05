<!--
  @component
  The settings page for a person signed in with Discord: link a trainer ID by putting a key in its
  profile comment, sign out, or delete the account. The server keeps checking for the key in the
  background, so the page picks up a pending key and a running check when it's opened again.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { clearAccount } from './account.svelte';
  import { ApiError } from './api';
  import {
    checkTrainerLink,
    deleteAccount,
    discardVerificationKey,
    fetchLinkedTrainers,
    type LinkCheck,
    type LinkedTrainer,
    type LinkedTrainers,
    requestVerificationKey,
    signOut,
    unlinkTrainer,
  } from './applications';
  import { applyFeatures } from './features';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { LOGIN_PATH, SETTINGS_PATH } from './route';
  import { isTrainerId, normaliseTrainerId } from './search';
  import SiteFooter from './SiteFooter.svelte';

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  /** How often the page asks how a running check is going. It only reads the server's state. */
  const POLL_MS = 5000;

  // Linked trainers: `null` until loaded.
  let linked = $state<LinkedTrainer[] | null>(null);
  let max = $state(5);
  let listState = $state<'loading' | 'ready' | 'unavailable' | 'error'>('loading');

  // Linking: the ID typed, then the key to put in its profile while one is pending.
  let trainerInput = $state('');
  let invalid = $state(false);
  let pending = $state<{ trainerId: string; key: string } | null>(null);
  let busy = $state(false);
  // While the server checks for the key in the background.
  let verifying = $state(false);
  let poll: ReturnType<typeof setInterval> | undefined;
  let copied = $state(false);
  let linkError = $state<string | null>(null);
  let linkSuccess = $state<string | null>(null);

  let signOutFailed = $state(false);
  let confirmingDelete = $state(false);
  let deleteFailed = $state(false);

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  /** Sends a person whose session ended back through sign-in. */
  function signInAgain() {
    window.location.assign(`${LOGIN_PATH}?next=${encodeURIComponent(SETTINGS_PATH)}`);
  }

  function status(error: unknown): number | null {
    return error instanceof ApiError ? error.status : null;
  }

  async function loadLinked(first = false) {
    try {
      const found = await fetchLinkedTrainers();
      linked = found.linked;
      max = found.max;
      listState = 'ready';
      follow(found, first);
    } catch (error) {
      if (status(error) === 401) {
        signInAgain();
      }
      listState = status(error) === 404 ? 'unavailable' : 'error';
    }
  }

  /** Brings the key and its check in line with the server's answer. */
  function follow(found: LinkedTrainers, first: boolean) {
    if (!pending) {
      // Opened again: pick up the newest pending key.
      const newest = found.pending[0];
      if (newest) {
        pending = { trainerId: newest.trainer, key: newest.key };
        settle(newest.check, first);
      }
      return;
    }
    const trainerId = pending.trainerId;
    const still = found.pending.find((link) => link.trainer === trainerId);
    if (still) {
      settle(still.check, first);
      return;
    }
    // The key is gone: it linked, or it expired.
    const done = found.linked.find((trainer) => trainer.id === trainerId);
    if (done) {
      linkSuccess = t('settings.link.linked', { name: done.name ?? trainerId });
      trainerInput = '';
    } else if (verifying) {
      linkError = t('settings.link.expired');
    }
    stopVerifying();
    pending = null;
  }

  /** Shows a check's state; an outcome only once it ends, or when the page opens. */
  function settle(check: LinkCheck | null, first: boolean) {
    if (check?.state === 'checking') {
      startVerifying();
      return;
    }
    const ended = verifying || first;
    stopVerifying();
    if (!ended || check === null) {
      return;
    }
    linkError =
      check.state === 'missing'
        ? t('settings.link.missing')
        : check.state === 'not_found'
          ? t('settings.link.notFound')
          : t('settings.link.tooMany', { n: max });
  }

  function startVerifying() {
    verifying = true;
    poll ??= setInterval(() => void loadLinked(), POLL_MS);
  }

  function stopVerifying() {
    verifying = false;
    clearInterval(poll);
    poll = undefined;
  }

  async function startLink(event: SubmitEvent) {
    event.preventDefault();
    linkError = null;
    linkSuccess = null;
    if (!isTrainerId(trainerInput)) {
      invalid = true;
      return;
    }
    invalid = false;
    const trainerId = normaliseTrainerId(trainerInput);
    busy = true;
    try {
      const found = await requestVerificationKey(trainerId);
      pending = { trainerId, key: found.key };
      copied = false;
    } catch (error) {
      linkError = startError(status(error));
    } finally {
      busy = false;
    }
  }

  function startError(code: number | null): string {
    switch (code) {
      case 401:
        signInAgain();
        return t('settings.link.error');
      case 409:
        return t('settings.link.already');
      case 429:
        return t('settings.link.tooManyTries');
      default:
        return t('settings.link.error');
    }
  }

  async function check() {
    if (!pending) {
      return;
    }
    linkError = null;
    busy = true;
    try {
      const result = await checkTrainerLink(pending.trainerId);
      if ('linked' in result) {
        linkSuccess = t('settings.link.linked', { name: result.name });
        pending = null;
        trainerInput = '';
        await loadLinked();
      } else {
        startVerifying();
      }
    } catch (error) {
      const code = status(error);
      if (code === 409) {
        // The key expired or was used up: start over.
        pending = null;
      }
      linkError = checkError(code);
    } finally {
      busy = false;
    }
  }

  function checkError(code: number | null): string {
    switch (code) {
      case 401:
        signInAgain();
        return t('settings.link.error');
      case 403:
        return t('settings.link.tooMany', { n: max });
      case 404:
        return t('settings.link.notFound');
      case 409:
        return t('settings.link.expired');
      case 429:
        return t('settings.link.tooManyTries');
      default:
        return t('settings.link.error');
    }
  }

  /** Forgets the key on the server too, which stops its check. */
  function cancel() {
    if (pending) {
      void discardVerificationKey(pending.trainerId).catch(() => undefined);
    }
    stopVerifying();
    pending = null;
    linkError = null;
  }

  async function copyKey() {
    if (!pending) {
      return;
    }
    try {
      await navigator.clipboard.writeText(pending.key);
      copied = true;
    } catch {
      // Without clipboard access the key is still shown to copy by hand.
    }
  }

  async function unlink(trainerId: string) {
    linkError = null;
    linkSuccess = null;
    try {
      await unlinkTrainer(trainerId);
      linked = (linked ?? []).filter((trainer) => trainer.id !== trainerId);
    } catch (error) {
      if (status(error) === 401) {
        signInAgain();
      }
      linkError = t('settings.link.error');
    }
  }

  async function signOutNow() {
    signOutFailed = false;
    try {
      await signOut();
      clearAccount();
      window.location.assign('/');
    } catch {
      signOutFailed = true;
    }
  }

  async function deleteNow() {
    deleteFailed = false;
    try {
      await deleteAccount();
      clearAccount();
      window.location.assign('/');
    } catch {
      deleteFailed = true;
      confirmingDelete = false;
    }
  }

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('settings.title');
  });

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    void loadLinked(true);
    return () => {
      features.abort();
      stopVerifying();
    };
  });
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main class="page">
  <header class="intro">
    <h1>{t('settings.heading')}</h1>
    <p class="lede">{t('settings.lede')}</p>
  </header>

  <section class="card" aria-labelledby="link-heading">
    <h2 id="link-heading">{t('settings.link.heading')}</h2>
    <p class="hint">{t('settings.link.intro')}</p>

    {#if listState === 'unavailable'}
      <p class="hint" role="status">{t('settings.link.unavailable')}</p>
    {:else if pending}
      <div class="key-box" aria-live="polite">
        <h3>{t('settings.link.keyHeading', { id: pending.trainerId })}</h3>
        <div class="key-row">
          <code class="key" data-testid="verification-key">{pending.key}</code>
          <button type="button" onclick={copyKey}>
            {t(copied ? 'settings.link.copied' : 'settings.link.copy')}
          </button>
        </div>
        <p class="hint">{t('settings.link.keySteps')}</p>
        <div class="actions">
          <button
            class="primary"
            class:verifying
            type="button"
            onclick={check}
            disabled={busy || verifying}
            aria-busy={verifying}
          >
            {#if verifying}
              <span class="spinner" aria-hidden="true"></span>
              {t('settings.link.verifying')}
            {:else}
              {t(busy ? 'settings.link.checking' : 'settings.link.check')}
            {/if}
          </button>
          <button type="button" onclick={cancel} disabled={busy}>
            {t('settings.link.cancel')}
          </button>
        </div>
        {#if verifying}
          <p class="hint verifying-hint" role="status">{t('settings.link.verifyingHint')}</p>
        {/if}
      </div>
    {:else}
      <form class="link-form" onsubmit={startLink} novalidate>
        <label for="trainer-id">{t('settings.link.label')}</label>
        <div class="field-row">
          <input
            id="trainer-id"
            type="text"
            inputmode="numeric"
            autocomplete="off"
            placeholder="123456789"
            aria-describedby="trainer-id-hint"
            aria-invalid={invalid ? 'true' : undefined}
            bind:value={trainerInput}
          />
          <button class="primary" type="submit" disabled={busy}>
            {t('settings.link.submit')}
          </button>
        </div>
        <small id="trainer-id-hint">{t('settings.link.hint')}</small>
        {#if invalid}<p class="failed" role="alert">{t('settings.link.invalid')}</p>{/if}
      </form>
    {/if}

    {#if linkError}<p class="failed" role="alert">{linkError}</p>{/if}
    {#if linkSuccess}<p class="success" role="status">{linkSuccess}</p>{/if}

    {#if listState !== 'unavailable'}
      <h3 class="linked-heading">{t('settings.link.linkedHeading')}</h3>
      {#if listState === 'error'}
        <p class="failed" role="alert">{t('settings.link.loadError')}</p>
      {:else if linked !== null && linked.length === 0}
        <p class="hint">{t('settings.link.none')}</p>
      {:else if linked !== null}
        <ul class="linked">
          {#each linked as trainer (trainer.id)}
            <li>
              <span class="trainer">
                {#if trainer.name}<strong class="name">{trainer.name}</strong>{/if}
                <span class="trainer-id">{trainer.id}</span>
              </span>
              <button
                type="button"
                aria-label={t('settings.link.unlinkFor', { id: trainer.id })}
                onclick={() => unlink(trainer.id)}
              >
                {t('settings.link.unlink')}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>

  <section class="card" aria-labelledby="session-heading">
    <h2 id="session-heading">{t('settings.session.heading')}</h2>
    <p class="hint">{t('settings.session.text')}</p>
    <button type="button" onclick={signOutNow}>{t('settings.signOut')}</button>
    {#if signOutFailed}<p class="failed" role="alert">{t('account.signOutError')}</p>{/if}
  </section>

  <section class="card" aria-labelledby="delete-heading">
    <h2 id="delete-heading">{t('settings.delete.heading')}</h2>
    <p class="hint">{t('settings.delete.text')}</p>
    {#if confirmingDelete}
      <p role="alert">{t('settings.delete.confirm')}</p>
      <div class="actions">
        <button class="danger" type="button" onclick={deleteNow}>
          {t('settings.delete.yes')}
        </button>
        <button type="button" onclick={() => (confirmingDelete = false)}>
          {t('settings.delete.cancel')}
        </button>
      </div>
    {:else}
      <button class="danger" type="button" onclick={() => (confirmingDelete = true)}>
        {t('settings.delete.button')}
      </button>
    {/if}
    {#if deleteFailed}<p class="failed" role="alert">{t('settings.delete.error')}</p>{/if}
  </section>
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  .page {
    display: grid;
    gap: 16px;
    width: min(100%, 720px);
    margin: 0 auto;
    padding: 24px var(--gutter) 40px;
    box-sizing: border-box;
  }

  .intro h1 {
    margin: 0;
    font-size: clamp(1.6rem, 3vw, 2.1rem);
    letter-spacing: -0.02em;
  }

  .lede {
    margin: 6px 0 0;
    color: var(--text-2);
  }

  .card {
    min-width: 0;
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  h2 {
    margin: 0 0 6px;
    font-size: 1.05rem;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 0.95rem;
  }

  small,
  .hint {
    display: block;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .hint {
    margin: 0 0 12px;
  }

  .link-form {
    display: grid;
    gap: 6px;
  }

  label {
    font-weight: 600;
  }

  .field-row,
  .key-row,
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .field-row input {
    flex: 1 1 12rem;
    min-width: 0;
  }

  .key-box {
    padding: 14px;
    background: var(--surface);
    border-radius: var(--radius-sm);
  }

  .key-row {
    margin-bottom: 10px;
  }

  .key {
    padding: 6px 12px;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    font-size: 1.1rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    user-select: all;
  }

  .linked-heading {
    margin-top: 18px;
  }

  .linked {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .linked li {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: var(--surface);
    border-radius: var(--radius-sm);
  }

  .trainer {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
    align-items: baseline;
    min-width: 0;
  }

  .name {
    overflow-wrap: anywhere;
  }

  .trainer-id {
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }

  /* Greyed out while the server checks, with a spinner before the label. */
  button.verifying,
  button.verifying:disabled {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    background: var(--surface);
    border-color: var(--border-strong);
    color: var(--muted);
    opacity: 1;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid currentcolor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      rotate: 360deg;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: 2.4s;
    }
  }

  .verifying-hint {
    margin: 10px 0 0;
  }

  p[role='alert'],
  .success {
    margin: 10px 0 0;
  }

  .success {
    color: var(--open);
  }

  .failed {
    color: var(--closed);
  }

  .danger {
    border-color: var(--closed);
    color: var(--closed);
  }
</style>
