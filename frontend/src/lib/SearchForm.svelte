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

<form class="search" onsubmit={submit} novalidate aria-label="Find clubs">
  <fieldset class="segmented">
    <legend class="visually-hidden">Search by</legend>
    <label>
      <input type="radio" name="mode" value="trainer" bind:group={search.mode} />
      Trainer ID
    </label>
    <label>
      <input type="radio" name="mode" value="pace" bind:group={search.mode} />
      Daily pace
    </label>
  </fieldset>

  <div class="row">
    {#if search.mode === 'trainer'}
      <div class="field">
        <label class="label" for="trainer-id">Trainer ID</label>
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
        <label class="label" for="pace">Daily fan gain (millions)</label>
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

    <div class="field options">
      <label class="check">
        <input type="checkbox" bind:checked={search.balance} />
        Balance scoring
      </label>
      <label class="slider-label" for="balance-weight"
        >Balance strength: {search.balanceWeight.toFixed(2)}</label
      >
      <input
        id="balance-weight"
        type="range"
        min="0"
        max="1"
        step="0.05"
        bind:value={search.balanceWeight}
        disabled={!search.balance}
      />
      <label class="check">
        <input type="checkbox" bind:checked={search.includeClosed} />
        Include closed clubs
      </label>
    </div>
  </div>

  <div class="actions">
    <button class="primary" type="submit" disabled={busy}
      >{busy ? 'Searching…' : 'Find clubs'}</button
    >
  </div>
</form>

<style>
  .search {
    display: grid;
    gap: 16px;
    padding: 16px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .search > .segmented {
    justify-self: start;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 16px 32px;
  }

  .field {
    display: grid;
    gap: 6px;
    align-content: start;
    min-width: 0;
  }

  .label {
    color: var(--text-2);
    font-size: 0.8125rem;
    font-weight: 600;
  }

  .field input:not([type]) {
    width: 100%;
    max-width: 320px;
    font-variant-numeric: tabular-nums;
  }

  .slider-label {
    color: var(--text-2);
    font-size: 0.8125rem;
    font-variant-numeric: tabular-nums;
  }

  input[type='range'] {
    width: 100%;
    max-width: 320px;
    margin: 0;
  }

  .options .check + .slider-label {
    margin-top: 2px;
  }

  .options input[type='range'] + .check {
    margin-top: 6px;
  }

  .error {
    margin: 0;
    color: var(--closed);
    font-size: 0.8125rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  @media (max-width: 720px) {
    .row {
      grid-template-columns: minmax(0, 1fr);
    }

    .field input:not([type]),
    input[type='range'] {
      max-width: none;
    }

    .actions button {
      width: 100%;
    }
  }
</style>
