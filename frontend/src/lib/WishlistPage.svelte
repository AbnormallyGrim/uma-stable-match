<!--
  @component
  The wishlist page: ideas for what to build next, each a StrawPoll poll in a frame. StrawPoll handles
  the voting, so there's nothing to store here.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { applyFeatures } from './features';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import { embedUrl, POLLS } from './polls';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import SiteFooter from './SiteFooter.svelte';

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('wishlist.title');
  });

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    return () => features.abort();
  });
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main>
  <header class="intro">
    <h1>{t('wishlist.heading')}</h1>
    <p>{t('wishlist.intro')}</p>
  </header>

  <section class="polls" aria-label={t('wishlist.polls')}>
    {#each POLLS as poll (poll.id)}
      <div class="poll">
        <iframe
          title={t(poll.title)}
          src={embedUrl(poll.id)}
          style:height="{poll.height}px"
          loading="lazy"
        ></iframe>
      </div>
    {/each}
  </section>

  <aside class="about" aria-label={t('wishlist.aboutLabel')}>
    <svg
      viewBox="0 0 16 16"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <circle cx="8" cy="8" r="6.25" />
      <path d="M8 7.25v4" />
      <circle cx="8" cy="4.75" r=".5" fill="currentColor" stroke="none" />
    </svg>
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- first-party text from the message catalog -->
    <p>{@html t('wishlist.about')}</p>
  </aside>
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  main {
    max-width: 1100px;
    padding-top: 40px;
  }

  .intro {
    text-align: center;
  }

  h1 {
    margin: 0 0 8px;
    font-size: clamp(1.75rem, 4vw, 2.5rem);
    letter-spacing: -0.02em;
    line-height: 1.15;
  }

  .intro p {
    max-width: 56ch;
    margin: 0 auto;
    color: var(--text-2);
    font-size: 1.05rem;
  }

  .polls {
    display: grid;
    gap: 24px;
    margin-top: 32px;
  }

  @media (width > 980px) {
    .polls {
      grid-template-columns: 1fr 1fr;
      align-items: start;
    }
  }

  .poll {
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 1px 2px rgb(0 0 0 / 6%);
  }

  iframe {
    display: block;
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
    border: 0;
    /* StrawPoll's page is light, so the frame stays transparent beside it in the dark theme. */
    color-scheme: light;
  }

  .about {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    max-width: 760px;
    margin: 24px auto 0;
    padding: 12px 18px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-2);
    font-size: 0.875rem;
  }

  .about svg {
    flex: none;
    margin-top: 2px;
    color: var(--accent);
  }

  .about p {
    margin: 0;
  }
</style>
