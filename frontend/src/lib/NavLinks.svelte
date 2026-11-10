<!--
  @component
  The pages as a pill of links in the navbar, like the theme switch: the website's home page, the
  club finder, the wishlist page, club applications and recruitment ads, each with an icon, and the page being shown raised. On narrow
  screens only the icons show, and a tooltip names each page. With the account menu or sign-in
  button there, the names wait for a wider screen.
-->

<script lang="ts">
  import { showsAccount } from './account.svelte';
  import { t } from './i18n';
  import { isAnnouncedPage } from './announcement';
  import { currentPage, hasSeenPage, markPageSeen, NEW_PAGES, PAGES, type Page } from './pages';

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  const current = currentPage(window.location.pathname);

  // Being on a new page counts as seeing it; the sign-in page, which belongs to applications, doesn't.
  if (current && isAnnouncedPage(window.location.pathname)) {
    markPageSeen(storage, current);
  }

  /** Whether the page's "New" badge shows: a new page the visitor hasn't been to. */
  function fresh(id: Page['id']): boolean {
    return NEW_PAGES.includes(id) && !hasSeenPage(storage, id);
  }
</script>

<!-- eslint-disable svelte/no-at-html-tags -- first-party icon drawings from pages.ts -->
<nav class="pages glass" class:with-account={showsAccount()} aria-label={t('nav.pages')}>
  {#each PAGES as page (page.id)}
    <a
      class="tooltip"
      href={page.href}
      data-tooltip={t(page.label)}
      aria-current={page.id === current ? 'page' : undefined}
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
        aria-hidden="true">{@html page.icon}</svg
      >
      <span class="label">{t(page.label)}</span>
      {#if fresh(page.id)}
        <span class="new-badge"><span class="new-badge-text">{t('nav.new')}</span></span>
      {/if}
    </a>
  {/each}
</nav>

<style>
  .pages {
    display: inline-flex;
    gap: 2px;
    padding: 3px;
    border-radius: 999px;
  }

  a {
    position: relative;
    display: inline-flex;
    gap: 7px;
    align-items: center;
    padding: 6px 9px;
    border-radius: 999px;
    color: var(--text-2);
    font-size: 0.875rem;
    font-weight: 500;
    text-decoration: none;
    white-space: nowrap;
  }

  a:hover {
    color: var(--text);
  }

  a[aria-current='page'] {
    background: var(--glass-strong);
    box-shadow:
      inset 0 1px 0 var(--glass-shine),
      0 1px 4px var(--glass-shadow);
    color: var(--accent);
    font-weight: 700;
  }

  /* A dot at the icon's corner; with room for the names, a small "New" beside the name. */
  .new-badge {
    position: absolute;
    top: 3px;
    right: 5px;
    width: 8px;
    height: 8px;
    background: var(--accent);
    border-radius: 50%;
  }

  .new-badge-text {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  /* Without room for the names, they stay for screen readers and show as tooltips. */
  .label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  @media (width >= 900px) {
    .pages:not(.with-account) a {
      padding: 6px 14px;
    }

    .pages:not(.with-account) .label {
      position: static;
      width: auto;
      height: auto;
      overflow: visible;
      clip: auto;
    }

    .pages:not(.with-account) a::after {
      display: none;
    }

    .pages:not(.with-account) .new-badge {
      position: static;
      width: auto;
      height: auto;
      padding: 1px 6px;
      color: var(--on-accent, #fff);
      font-size: 0.65rem;
      font-weight: 700;
      letter-spacing: 0.03em;
      text-transform: uppercase;
      border-radius: 999px;
    }

    .pages:not(.with-account) .new-badge-text {
      position: static;
      width: auto;
      height: auto;
      overflow: visible;
      clip: auto;
    }
  }

  @media (width >= 1440px) {
    a {
      padding: 6px 14px;
    }

    .label {
      position: static;
      width: auto;
      height: auto;
      overflow: visible;
      clip: auto;
    }

    a::after {
      display: none;
    }

    .new-badge {
      position: static;
      width: auto;
      height: auto;
      padding: 1px 6px;
      color: var(--on-accent, #fff);
      font-size: 0.65rem;
      font-weight: 700;
      letter-spacing: 0.03em;
      text-transform: uppercase;
      border-radius: 999px;
    }

    .new-badge-text {
      position: static;
      width: auto;
      height: auto;
      overflow: visible;
      clip: auto;
    }
  }
</style>
