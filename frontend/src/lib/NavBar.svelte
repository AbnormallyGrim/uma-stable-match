<!--
  @component
  The top bar: the brand, linking back to the website, a button that opens the release notes, a
  help button that starts the product tour, and the theme switch. It matches the website's navbar
  and stays at the top while scrolling.
-->

<script lang="ts">
  import BrandLogo from './BrandLogo.svelte';
  import { t } from './i18n';
  import LanguageToggle from './LanguageToggle.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  interface Props {
    /** Starts the product tour. */
    onhelp: () => void;
    /** Opens the release notes. */
    onreleases: () => void;
    /** Whether there's a release the visitor hasn't seen; marks the button with a dot. */
    unseen: boolean;
  }

  let { onhelp, onreleases, unseen }: Props = $props();

  const WEBSITE = 'https://umastable.com';
</script>

<nav class="navbar" aria-label={t('nav.label')}>
  <div class="inner">
    <a class="brand" href={WEBSITE}>
      <BrandLogo size={40} />
      <span class="title">{t('app.name')}</span>
    </a>
    <div class="tools">
      <button
        type="button"
        class="round tooltip"
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
      <button
        type="button"
        class="round tooltip"
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
      <LanguageToggle />
      <ThemeToggle />
    </div>
  </div>
</nav>

<style>
  .navbar {
    position: sticky;
    z-index: 100;
    top: 0;
    background: color-mix(in srgb, var(--bg) 85%, transparent);
    border-bottom: 1px solid var(--border);
    backdrop-filter: blur(12px);
  }

  .inner {
    display: flex;
    gap: 16px;
    align-items: center;
    justify-content: space-between;
    max-width: var(--page-max);
    margin: 0 auto;
    padding: 10px var(--gutter);
  }

  .brand {
    display: flex;
    gap: 12px;
    align-items: center;
    min-width: 0;
    color: var(--text);
    text-decoration: none;
  }

  .tools {
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
    background: var(--surface);
    border-color: var(--border);
    border-radius: 999px;
    color: var(--text-2);
  }

  .round:hover:not(:disabled) {
    background: var(--card);
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
  }

  @media (max-width: 400px) {
    .title {
      font-size: 1rem;
    }
  }
</style>
