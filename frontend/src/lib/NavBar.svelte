<!--
  @component
  The top bar: the brand, linking back to the website, the pages, a button that opens the release
  notes, a help button that starts the product tour if there is one, the theme switch and, at the
  far right for a person signed in with Discord, the account menu. It matches the website's navbar
  and stays at the top while scrolling: see-through at the top of the page, filling in over the
  first scroll, as the name slides away behind the logo.
-->

<script lang="ts">
  import { showsAccount } from './account.svelte';
  import AccountMenu from './AccountMenu.svelte';
  import BrandLogo from './BrandLogo.svelte';
  import { t } from './i18n';
  import LanguageToggle from './LanguageToggle.svelte';
  import NavLinks from './NavLinks.svelte';
  import { websiteUrl } from './pages';
  import ThemeToggle from './ThemeToggle.svelte';

  interface Props {
    /** Starts the product tour. Without it, there's no help button. */
    onhelp?: () => void;
    /** Opens the release notes. */
    onreleases: () => void;
    /** Whether there's a release the visitor hasn't seen; marks the button with a dot. */
    unseen: boolean;
  }

  let { onhelp, onreleases, unseen }: Props = $props();

  const WEBSITE = websiteUrl(window.location);

  /** How far to scroll before the bar is filled in and the name is hidden. */
  const COLLAPSE_PX = 120;

  let scrollY = $state(0);
  // 0 at the top of the page, 1 once scrolled COLLAPSE_PX.
  const progress = $derived(Math.min(1, Math.max(0, scrollY / COLLAPSE_PX)));
</script>

<svelte:window bind:scrollY />

<nav class="navbar" aria-label={t('nav.label')} style:--p={progress}>
  <div class="inner" class:with-account={showsAccount()}>
    <a class="brand" href={WEBSITE}>
      <BrandLogo size={40} />
      <span class="title">{t('app.name')}</span>
    </a>
    <NavLinks />
    <div class="tools">
      <button
        type="button"
        class="round glass tooltip"
        data-tooltip={t('nav.releases')}
        aria-label={t('nav.releases')}
        onclick={onreleases}
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
          <path d="M6.5 2.5 7.8 6.2l3.7 1.3-3.7 1.3-1.3 3.7-1.3-3.7-3.7-1.3 3.7-1.3z" />
          <path d="M12.5 1.5v3M11 3h3" />
        </svg>
        {#if unseen}
          <span class="dot" data-testid="unseen-release" aria-hidden="true"></span>
        {/if}
      </button>
      {#if onhelp}
        <button
          type="button"
          class="round glass tooltip"
          data-tooltip={t('nav.tour')}
          data-tour="help"
          aria-label={t('nav.tour')}
          onclick={onhelp}
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
            <circle cx="8" cy="8" r="6.25" />
            <path d="M6.25 6.25a1.75 1.75 0 1 1 2.5 1.6c-.5.25-.75.65-.75 1.15v.25" />
            <circle cx="8" cy="11.5" r=".5" fill="currentColor" stroke="none" />
          </svg>
        </button>
      {/if}
      <LanguageToggle />
      <ThemeToggle />
      <AccountMenu />
    </div>
  </div>
</nav>

<style>
  /* --p is the scroll progress, 0 to 1: the background, its blur and the line fade in with it. */
  .navbar {
    position: sticky;
    z-index: 100;
    top: 0;
    background: color-mix(in srgb, var(--bg) calc(var(--p) * 85%), transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--border) calc(var(--p) * 100%), transparent);
    backdrop-filter: blur(calc(var(--p) * 12px));
  }

  .inner {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
    align-items: center;
    justify-content: space-between;
    max-width: var(--page-max);
    margin: 0 auto;
    padding: 10px var(--gutter);
  }

  /* With room, the brand, pages and tools sit in three columns whose outer two are as wide as the
     wider of them, so the pages stay put from one page to the next. The website's navbar does the
     same. The account menu or sign-in button makes the tools wider, so then it needs more room. */
  @media (width >= 960px) {
    .inner:not(.with-account) {
      display: grid;
      grid-template-columns: minmax(max-content, 1fr) auto minmax(max-content, 1fr);
    }
  }

  @media (width >= 1100px) {
    .inner.with-account {
      display: grid;
      grid-template-columns: minmax(max-content, 1fr) auto minmax(max-content, 1fr);
    }
  }

  /* Clips the name once it has slid past the logo. */
  .brand {
    justify-self: start;
    display: flex;
    gap: 12px;
    align-items: center;
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    text-decoration: none;
  }

  /* Above the name, with a solid fill, so the name passes behind it. */
  .brand :global(.logo) {
    position: relative;
    z-index: 1;
    background: var(--bg);
    border-radius: var(--radius-sm);
  }

  .tools {
    justify-self: end;
    display: flex;
    flex: none;
    gap: 8px;
    align-items: center;
  }

  /* Round, the height of the theme switch beside it. */
  .round {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border-radius: 999px;
    color: var(--text-2);
  }

  .round:hover:not(:disabled) {
    background: var(--glass-strong);
    color: var(--text);
  }

  /* Marks the release notes button while there's something new in them. */
  .dot {
    position: absolute;
    top: 3px;
    right: 3px;
    width: 9px;
    height: 9px;
    background: var(--accent);
    border: 2px solid var(--bg);
    border-radius: 50%;
    box-sizing: content-box;
    translate: 25% -25%;
  }

  .title {
    overflow: hidden;
    font-size: 1.15rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: calc(1 - var(--p) * var(--p));
    translate: calc(var(--p) * (-100% - 12px)) 0;
  }

  @media (prefers-reduced-motion: reduce) {
    .title {
      translate: none;
    }
  }

  /* On phones the pages' Home link takes the brand's place, so the pages and the buttons fit. */
  @media (max-width: 560px) {
    .brand {
      position: absolute;
      width: 1px;
      height: 1px;
      overflow: hidden;
      clip: rect(0 0 0 0);
    }
  }
</style>
