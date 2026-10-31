<!--
  @component
  The pages as a pill of links in the navbar, like the theme switch: the website's home page, the
  club finder and the vote page, each with an icon, and the page being shown raised. On narrow
  screens only the icons show, and a tooltip names each page.
-->

<script lang="ts">
  import { t } from './i18n';
  import { currentPage, PAGES } from './pages';

  const current = currentPage(window.location.pathname);
</script>

<!-- eslint-disable svelte/no-at-html-tags -- first-party icon drawings from pages.ts -->
<nav class="pages" aria-label={t('nav.pages')}>
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
    </a>
  {/each}
</nav>

<style>
  .pages {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
  }

  a {
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
    background: var(--card);
    box-shadow: 0 0 0 1px var(--border);
    color: var(--accent);
    font-weight: 700;
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
  }
</style>
