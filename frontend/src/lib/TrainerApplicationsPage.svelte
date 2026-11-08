<!--
  @component
  The page where a trainer follows the club applications they've sent in the last month.
-->

<script lang="ts">
  import { onMount } from 'svelte';

  import { ApiError } from './api';
  import { fetchMyAd, type MyAd } from './ads';
  import { fetchMyApplications, loginUrl, type TrainerApplication } from './applications';
  import { applyFeatures } from './features';
  import { t } from './i18n';
  import { language } from './language.svelte';
  import NavBar from './NavBar.svelte';
  import ReleaseNotes from './ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './releases';
  import { TRAINER_APPLICATIONS_PATH } from './route';
  import SiteFooter from './SiteFooter.svelte';
  import TrainerAdsTable from './TrainerAdsTable.svelte';
  import TrainerApplicationsTable from './TrainerApplicationsTable.svelte';

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  let applications = $state<TrainerApplication[]>([]);
  let ad = $state<MyAd | null>(null);

  $effect(() => {
    document.documentElement.lang = language.current;
    document.title = t('trainer.title');
  });

  onMount(() => {
    const features = new AbortController();
    applyFeatures({ signal: features.signal });
    fetchMyApplications({ signal: features.signal }).then(
      (found) => (applications = found),
      (reason) => {
        // Applications are for people signed in with Discord.
        if (reason instanceof ApiError && reason.status === 401) {
          window.location.assign(loginUrl(TRAINER_APPLICATIONS_PATH));
        }
      },
    );
    // The table of ads shows its empty state if the ad can't be read.
    fetchMyAd({ signal: features.signal }).then(
      (found) => (ad = found),
      () => undefined,
    );
    return () => features.abort();
  });
</script>

<NavBar onreleases={openReleases} unseen={unseenRelease} />

<main>
  <header class="intro">
    <p class="eyebrow">{t('trainer.eyebrow')}</p>
    <h1>{t('trainer.heading')}</h1>
    <p class="lede">{t('trainer.lede')}</p>
  </header>

  <TrainerApplicationsTable {applications} />
  <TrainerAdsTable {ad} />
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

<style>
  main {
    display: grid;
    gap: 20px;
    max-width: 960px;
  }

  .eyebrow {
    margin: 0;
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h1 {
    margin: 2px 0;
    font-size: 1.6rem;
  }

  .lede {
    margin: 0;
    color: var(--text-2);
  }
</style>
