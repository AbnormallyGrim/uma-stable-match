<!--
  @component
  The navbar's account menu for a person signed in with Discord: a button with their name that
  opens who they're signed in as, a link to the settings page, and signing out. It follows the
  ARIA menu button pattern. Signed out, it's a round silhouette linking to sign-in, shown once the
  server has answered and while applications are on.
-->

<script lang="ts">
  import { tick } from 'svelte';

  import { account, clearAccount } from './account.svelte';
  import { signOut } from './applications';
  import { flags } from './flags.svelte';
  import { t } from './i18n';
  import {
    APPLICATIONS_PATH,
    isLoginRoute,
    isSettingsRoute,
    LOGIN_PATH,
    SETTINGS_PATH,
  } from './route';

  let open = $state(false);
  let failed = $state(false);
  let root: HTMLDivElement | undefined = $state();
  let button: HTMLButtonElement | undefined = $state();
  let menu: HTMLUListElement | undefined = $state();

  const name = $derived(account.current?.name ?? t('account.unnamed'));
  const here = `${window.location.pathname}${window.location.search}`;
  const signInHref = `${LOGIN_PATH}?next=${encodeURIComponent(here)}`;
  const onLoginPage = isLoginRoute(window.location.pathname);

  function items(): HTMLElement[] {
    return [...(menu?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
  }

  async function show(focus: 'first' | 'last' = 'first') {
    open = true;
    failed = false;
    await tick();
    const all = items();
    (focus === 'first' ? all[0] : all.at(-1))?.focus();
  }

  function hide(refocus = true) {
    open = false;
    if (refocus) {
      button?.focus();
    }
  }

  function onButtonKey(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      void show('first');
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      void show('last');
    }
  }

  function onMenuKey(event: KeyboardEvent) {
    const all = items();
    const at = all.indexOf(document.activeElement as HTMLElement);
    const move = (to: number) => all[(to + all.length) % all.length]?.focus();
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        move(at + 1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        move(at - 1);
        break;
      case 'Home':
        event.preventDefault();
        move(0);
        break;
      case 'End':
        event.preventDefault();
        move(all.length - 1);
        break;
      case 'Escape':
        event.preventDefault();
        hide();
        break;
      case 'Tab':
        hide(false);
        break;
    }
  }

  function onWindowClick(event: MouseEvent) {
    if (open && root && !root.contains(event.target as Node)) {
      hide(false);
    }
  }

  async function signOutNow() {
    try {
      await signOut();
    } catch {
      failed = true;
      return;
    }
    clearAccount();
    open = false;
    // Pages that need a session send the person away; elsewhere the menu just goes.
    const path = window.location.pathname;
    if (isSettingsRoute(path) || path.startsWith(APPLICATIONS_PATH)) {
      window.location.assign('/');
    }
  }
</script>

<svelte:window onclick={onWindowClick} />

{#if account.current}
  <div class="account" bind:this={root}>
    <button
      bind:this={button}
      type="button"
      class="trigger"
      id="account-button"
      aria-haspopup="menu"
      aria-expanded={open}
      aria-controls="account-menu"
      onclick={() => (open ? hide() : show())}
      onkeydown={onButtonKey}
    >
      <svg
        viewBox="0 0 16 16"
        width="16"
        height="16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <circle cx="8" cy="5.25" r="2.5" />
        <path d="M3 13.5c.5-2.75 2.5-4 5-4s4.5 1.25 5 4" />
      </svg>
      <span class="name">{name}</span>
      <svg
        class="chevron"
        viewBox="0 0 16 16"
        width="12"
        height="12"
        fill="none"
        stroke="currentColor"
        stroke-width="1.75"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <path d="m4 6 4 4 4-4" />
      </svg>
    </button>

    {#if open}
      <div class="popup">
        <p class="who">{t('account.signedInAs', { name })}</p>
        <ul
          bind:this={menu}
          id="account-menu"
          role="menu"
          aria-labelledby="account-button"
          tabindex="-1"
          onkeydown={onMenuKey}
        >
          <li role="none">
            <a role="menuitem" href={SETTINGS_PATH} tabindex="-1">{t('account.settings')}</a>
          </li>
          <li role="none">
            <button role="menuitem" type="button" tabindex="-1" onclick={signOutNow}>
              {t('account.signOut')}
            </button>
          </li>
        </ul>
        {#if failed}<p class="failed" role="alert">{t('account.signOutError')}</p>{/if}
      </div>
    {/if}
  </div>
{:else if account.known && flags.applications && !onLoginPage}
  <a
    class="sign-in tooltip"
    href={signInHref}
    data-tooltip={t('account.signIn')}
    aria-label={t('account.signIn')}
  >
    <svg
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <circle cx="8" cy="5.25" r="2.5" />
      <path d="M3 13.5c.5-2.75 2.5-4 5-4s4.5 1.25 5 4" />
    </svg>
  </a>
{/if}

<style>
  .account {
    position: relative;
    min-width: 0;
  }

  /* As tall as the round buttons beside it. */
  .trigger {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    height: 34px;
    max-width: 12rem;
    padding: 0 10px 0 9px;
    background: var(--surface);
    border-color: var(--border);
    border-radius: 999px;
    color: var(--text-2);
  }

  /* Round, like the help button beside it. */
  .sign-in {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-sizing: border-box;
    color: var(--text-2);
  }

  .sign-in:hover {
    background: var(--card);
    color: var(--text);
  }

  .trigger:hover:not(:disabled),
  .trigger[aria-expanded='true'] {
    background: var(--card);
    color: var(--text);
  }

  .trigger svg {
    flex: none;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    transition: rotate 0.15s ease;
  }

  .trigger[aria-expanded='true'] .chevron {
    rotate: 180deg;
  }

  .popup {
    position: absolute;
    z-index: 110;
    top: calc(100% + 6px);
    right: 0;
    width: max-content;
    min-width: 200px;
    max-width: min(280px, calc(100vw - 2 * var(--gutter)));
    padding: 6px;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 12px 32px rgb(0 0 0 / 14%);
  }

  .who {
    margin: 0;
    padding: 8px 10px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8rem;
    overflow-wrap: anywhere;
  }

  ul {
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }

  ul:focus {
    outline: none;
  }

  [role='menuitem'] {
    display: block;
    box-sizing: border-box;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    font-size: 0.875rem;
    text-align: left;
    text-decoration: none;
    cursor: pointer;
  }

  [role='menuitem']:hover,
  [role='menuitem']:focus-visible {
    background: var(--surface);
  }

  .failed {
    margin: 6px 10px 4px;
    color: var(--closed);
    font-size: 0.8rem;
  }

  /* On phones the name gets less room, so the other buttons still fit. */
  @media (max-width: 560px) {
    .trigger {
      max-width: 8rem;
    }
  }
</style>
