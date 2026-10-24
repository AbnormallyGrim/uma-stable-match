<script lang="ts">
  import { onMount } from 'svelte';

  import { fetchRecommendations, type RecommendationQuery } from './lib/api';
  import HowToUse from './lib/HowToUse.svelte';
  import NavBar from './lib/NavBar.svelte';
  import Results from './lib/Results.svelte';
  import { SearchRunner, type SearchState as RunnerState } from './lib/runner';
  import SearchForm from './lib/SearchForm.svelte';
  import SearchStatus from './lib/SearchStatus.svelte';
  import SiteFooter from './lib/SiteFooter.svelte';
  import { resultNotices } from './lib/states';
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

  const runner = new SearchRunner({
    fetch: (query, signal) => fetchRecommendations({ ...query, limit: RESULT_LIMIT }, { signal }),
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
    const found = `Found ${total} ${total === 1 ? 'club' : 'clubs'}.`;
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

  onMount(() => {
    if (fromUrl) {
      const validation = validate(fromUrl);
      if (validation.ok) {
        runner.run(validation.query);
      }
    }
    return () => runner.dispose();
  });
</script>

<NavBar />

<main>
  <div class="hero">
    <h1 class="visually-hidden">Uma Stable Match club finder</h1>
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
