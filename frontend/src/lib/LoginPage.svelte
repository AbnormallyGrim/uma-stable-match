<script lang="ts">
  import { onMount } from 'svelte';

  import { loginUrl } from './applications';
  import { applyFeatures } from './features';
  import { APPLICATIONS_PATH } from './route';
  import { language } from './language.svelte';
  import { t } from './i18n';
  import NavBar from './NavBar.svelte';
  import { websiteUrl } from './pages';
  import SiteFooter from './SiteFooter.svelte';

  const next = new URLSearchParams(window.location.search).get('next') ?? APPLICATIONS_PATH;

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = 'Sign in to Uma Stable Match';
  });

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    return () => features.abort();
  });
</script>

<NavBar onreleases={() => undefined} unseen={false} />

<main class="page">
  <section class="login-card" aria-labelledby="login-heading">
    <p class="eyebrow">Club applications</p>
    <h1 id="login-heading">Sign in to Uma Stable Match</h1>
    <p class="description">Connect your account to access club applications features</p>

    <a class="provider-btn discord" href={loginUrl(next)}>
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
      <span>Sign in with Discord</span>
    </a>
  </section>

  <p class="privacy-link">
    <a href={`${websiteUrl(window.location)}/privacy/`}>{t('footer.privacy')}</a>
  </p>
</main>

<SiteFooter />

<style>
  .page {
    display: grid;
    align-content: center;
    place-items: center;
    min-height: calc(100vh - 136px);
    padding: 40px var(--gutter);
  }

  .login-card {
    width: min(100%, 420px);
    padding: 28px 24px 24px;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: 18px;
    box-shadow: 0 12px 36px rgb(0 0 0 / 8%);
    text-align: center;
  }

  .eyebrow {
    margin: 0 0 10px;
    color: var(--accent);
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  h1 {
    margin: 0;
    color: var(--text);
    font-size: clamp(1.8rem, 3vw, 2.4rem);
    line-height: 1.15;
    letter-spacing: -0.03em;
  }

  .description {
    margin: 12px 0 22px;
    color: var(--text-2);
    font-size: 0.98rem;
    line-height: 1.6;
  }

  .provider-btn {
    width: 100%;
  }

  .privacy-link {
    margin: 16px 0 0;
    font-size: 0.875rem;
  }
</style>
