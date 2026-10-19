<script lang="ts">
  import { onMount } from 'svelte';

  import { fetchRecommendations, type RecommendationQuery } from './lib/api';
  import DataStatus from './lib/DataStatus.svelte';
  import Results from './lib/Results.svelte';
  import { SearchRunner, type SearchState as RunnerState } from './lib/runner';
  import SearchForm from './lib/SearchForm.svelte';
  import SearchStatus from './lib/SearchStatus.svelte';
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

  // Above a scan's default 80 clubs, so the table can list them all.
  const RESULT_LIMIT = 100;

  let search = $state<RunnerState>({ status: { phase: 'idle' }, result: null });

  const runner = new SearchRunner({
    fetch: (query, signal) => fetchRecommendations({ ...query, limit: RESULT_LIMIT }, { signal }),
    onchange: (state) => {
      search = state;
    },
  });

  const busy = $derived(search.status.phase === 'loading');
  const showResults = $derived(
    search.result !== null && (search.status.phase === 'done' || search.status.phase === 'loading'),
  );

  // A finished scan means a search waiting on a 503 can retry now.
  function onscanready() {
    if (search.status.phase === 'waiting-for-scan') {
      runner.retryNow();
    }
  }

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

<main>
  <h1>Uma Stable Match</h1>
  <DataStatus {onscanready} />
  <SearchForm {initial} remember={remembered !== null} {busy} {onsearch} />

  <SearchStatus status={search.status} onretry={() => runner.retryNow()} />

  <div aria-live="polite" class="visually-hidden">
    {#if search.status.phase === 'done' && search.result}
      Found {search.result.total}
      {search.result.total === 1 ? 'club' : 'clubs'}.
    {/if}
  </div>

  {#if showResults && search.result}
    <Results result={search.result} stale={busy} />
  {/if}
</main>
