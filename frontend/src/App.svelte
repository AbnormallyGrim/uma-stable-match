<script lang="ts">
  import { onMount } from 'svelte';

  import {
    ApiError,
    fetchRecommendations,
    type RecommendationQuery,
    type Recommendations,
  } from './lib/api';
  import Results from './lib/Results.svelte';
  import SearchForm from './lib/SearchForm.svelte';
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

  const RESULT_LIMIT = 100;

  let busy = $state(false);
  let result = $state<Recommendations | null>(null);
  let error = $state<string | null>(null);
  let controller: AbortController | null = null;

  async function search(query: RecommendationQuery) {
    controller?.abort();
    const current = new AbortController();
    controller = current;
    busy = true;
    error = null;
    try {
      result = await fetchRecommendations(
        { ...query, limit: RESULT_LIMIT },
        { signal: current.signal },
      );
    } catch (caught) {
      if (current.signal.aborted) {
        return;
      }
      result = null;
      error = caught instanceof ApiError ? caught.message : 'Something went wrong.';
    } finally {
      if (controller === current) {
        busy = false;
      }
    }
  }

  function onsearch(query: RecommendationQuery, state: SearchState, remember: boolean) {
    if (state.mode === 'trainer') {
      saveRememberedTrainerId(storage, remember ? state.trainerId : null);
    }
    window.history.replaceState(null, '', `${window.location.pathname}?${toUrlParams(query)}`);
    void search(query);
  }

  onMount(() => {
    if (fromUrl) {
      const validation = validate(fromUrl);
      if (validation.ok) {
        void search(validation.query);
      }
    }
    return () => controller?.abort();
  });
</script>

<main>
  <h1>Uma Stable Match</h1>
  <SearchForm {initial} remember={remembered !== null} {busy} {onsearch} />

  <div aria-live="polite">
    {#if error}
      <p class="error" role="alert">{error}</p>
    {:else if result}
      <p class="visually-hidden">
        Found {result.total}
        {result.total === 1 ? 'club' : 'clubs'}.
      </p>
    {/if}
  </div>

  {#if result && !error}
    <Results {result} />
  {/if}
</main>
