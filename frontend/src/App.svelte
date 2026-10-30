<script lang="ts">
  import { onMount } from 'svelte';

  import { fetchFeatures, fetchRecommendations, type RecommendationQuery } from './lib/api';
  import { HumanCheck } from './lib/human';
  import HowToUse from './lib/HowToUse.svelte';
  import { t } from './lib/i18n';
  import { language } from './lib/language.svelte';
  import NavBar from './lib/NavBar.svelte';
  import ReleaseNotes from './lib/ReleaseNotes.svelte';
  import { hasUnseenRelease, markReleaseSeen, RELEASES } from './lib/releases';
  import Results from './lib/Results.svelte';
  import { SearchRunner, type SearchState as RunnerState } from './lib/runner';
  import SearchForm from './lib/SearchForm.svelte';
  import SearchStatus from './lib/SearchStatus.svelte';
  import SiteFooter from './lib/SiteFooter.svelte';
  import { resultNotices } from './lib/states';
  import { hasSeenTour, markTourSeen, tourSteps } from './lib/tour';
  import Tour from './lib/Tour.svelte';
  import {
    DEFAULT_SEARCH,
    fromUrlParams,
    loadRememberedTrainerId,
    saveRememberedTrainerId,
    toUrlParams,
    validate,
    type SearchState,
  } from './lib/search';

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;
  const fromUrl = fromUrlParams(new URLSearchParams(window.location.search));
  const remembered = loadRememberedTrainerId(storage);
  const initial: SearchState =
    fromUrl ?? (remembered ? { ...DEFAULT_SEARCH, trainerId: remembered } : DEFAULT_SEARCH);

  // The table lists the best fits only; the summary still counts every scored club.
  const RESULT_LIMIT = 100;

  let search = $state<RunnerState>({ status: { phase: 'idle' }, result: null });
  // The tour starts on a first visit, unless the visitor arrived with a search to run; then it
  // waits for a later visit.
  let touring = $state(fromUrl === null && !hasSeenTour(storage));

  function endTour() {
    touring = false;
    markTourSeen(storage);
  }

  let readingReleases = $state(false);
  let unseenRelease = $state(hasUnseenRelease(storage));

  function openReleases() {
    readingReleases = true;
    unseenRelease = false;
    markReleaseSeen(storage);
  }

  const human = new HumanCheck();

  const runner = new SearchRunner({
    fetch: (query, signal) =>
      fetchRecommendations({ ...query, limit: RESULT_LIMIT }, { signal, human }),
    onchange: (state) => {
      search = state;
    },
  });

  const busy = $derived(search.status.phase === 'loading');
  const announcement = $derived.by(() => {
    if (search.status.phase !== 'done' || !search.result) {
      return '';
    }
    const total = search.result.total;
    const found = t('app.found', { n: total });
    return [found, ...resultNotices(search.result)].join(' ');
  });
  const showResults = $derived(
    search.result !== null && (search.status.phase === 'done' || search.status.phase === 'loading'),
  );

  function onsearch(query: RecommendationQuery, state: SearchState, remember: boolean) {
    if (state.mode === 'trainer') {
      saveRememberedTrainerId(storage, remember ? state.trainerId : null);
    }
    window.history.replaceState(null, '', `${window.location.pathname}?${toUrlParams(query)}`);
    runner.run(query);
  }

  $effect(() => {
    document.documentElement.lang = language.current;
  });

  onMount(() => {
    // A failed request leaves Japanese off.
    const features = new AbortController();
    fetchFeatures({ signal: features.signal }).then(
      (found) => (language.japaneseEnabled = found.japanese),
      () => {},
    );
    if (fromUrl) {
      const validation = validate(fromUrl);
      if (validation.ok) {
        runner.run(validation.query);
      }
    }
    return () => {
      features.abort();
      runner.dispose();
    };
  });
</script>

<NavBar onhelp={() => (touring = true)} onreleases={openReleases} unseen={unseenRelease} />

<main>
  <div class="hero">
    <h1 class="visually-hidden">{t('app.heading')}</h1>
    <SearchForm {initial} remember={remembered !== null} {busy} {onsearch} />
    <HowToUse />
  </div>

  <SearchStatus status={search.status} onretry={() => runner.retryNow()} />

  <div aria-live="polite" class="visually-hidden">{announcement}</div>

  {#if showResults && search.result}
    <Results result={search.result} stale={busy} />
  {/if}
</main>

<SiteFooter />

{#if readingReleases}
  <ReleaseNotes releases={RELEASES} onclose={() => (readingReleases = false)} />
{/if}

{#if touring}
  <Tour steps={tourSteps()} onclose={endTour} />
{/if}
