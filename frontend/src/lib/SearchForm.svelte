<script lang="ts">
  import { untrack } from 'svelte';

  import type { RecommendationQuery } from './api';
  import { validate, type SearchErrors, type SearchState } from './search';

  // The search form. It owns its fields after the first render, so later
  // `initial` changes are ignored. `onsearch` only receives valid searches.
  interface Props {
    initial: SearchState;
    remember: boolean;
    busy?: boolean;
    onsearch: (query: RecommendationQuery, state: SearchState, remember: boolean) => void;
  }

  let { initial, remember: rememberInitially, busy = false, onsearch }: Props = $props();

  let search = $state<SearchState>(untrack(() => ({ ...initial })));
  let remember = $state(untrack(() => rememberInitially));
  let errors = $state<SearchErrors>({});

  function submit(event: SubmitEvent) {
    event.preventDefault();
    const result = validate(search);
    if (!result.ok) {
      errors = result.errors;
      return;
    }
    errors = {};
    onsearch(result.query, { ...search }, remember);
  }
</script>

<form onsubmit={submit} novalidate aria-label="Find clubs">
  <fieldset>
    <legend>Search by</legend>
    <label>
      <input type="radio" name="mode" value="trainer" bind:group={search.mode} />
      Trainer ID
    </label>
    <label>
      <input type="radio" name="mode" value="pace" bind:group={search.mode} />
      Daily pace
    </label>
  </fieldset>

  {#if search.mode === 'trainer'}
    <div class="field">
      <label for="trainer-id">Trainer ID</label>
      <input
        id="trainer-id"
        name="trainer_id"
        inputmode="numeric"
        autocomplete="off"
        placeholder="123456789012"
        bind:value={search.trainerId}
        aria-invalid={errors.trainerId ? 'true' : undefined}
        aria-describedby={errors.trainerId ? 'trainer-id-error' : undefined}
      />
      {#if errors.trainerId}
        <p id="trainer-id-error" class="error">{errors.trainerId}</p>
      {/if}
      <label class="check">
        <input type="checkbox" bind:checked={remember} />
        Remember my trainer ID on this device
      </label>
    </div>
  {:else}
    <div class="field">
      <label for="pace">Daily fan gain (millions)</label>
      <input
        id="pace"
        name="pace"
        inputmode="decimal"
        autocomplete="off"
        placeholder="7.6"
        bind:value={search.paceMillions}
        aria-invalid={errors.pace ? 'true' : undefined}
        aria-describedby={errors.pace ? 'pace-error' : undefined}
      />
      {#if errors.pace}
        <p id="pace-error" class="error">{errors.pace}</p>
      {/if}
    </div>
  {/if}

  <div class="field">
    <label class="check">
      <input type="checkbox" bind:checked={search.balance} />
      Balance scoring
    </label>
    <label for="balance-weight">Balance strength: {search.balanceWeight.toFixed(2)}</label>
    <input
      id="balance-weight"
      type="range"
      min="0"
      max="1"
      step="0.05"
      bind:value={search.balanceWeight}
      disabled={!search.balance}
    />
  </div>

  <label class="check">
    <input type="checkbox" bind:checked={search.includeClosed} />
    Include closed clubs
  </label>

  <button type="submit" disabled={busy}>{busy ? 'Searching…' : 'Find clubs'}</button>
</form>
