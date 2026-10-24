<script lang="ts">
  import { untrack } from 'svelte';

  import type { JoinStyle, RecommendationQuery } from './api';
  import Dropdown from './Dropdown.svelte';
  import { joinStyleLabel, PLAYSTYLES } from './format';
  import {
    DISCORD_NOTE,
    JOIN_STYLES,
    narrowsSearch,
    validate,
    type SearchErrors,
    type SearchState,
  } from './search';

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
  const PLAYSTYLE_OPTIONS: { value: number | null; label: string }[] = [
    { value: null, label: 'Any playstyle' },
    ...PLAYSTYLES.map(([code, label]) => ({ value: code, label })),
  ];

  const narrowed = $derived(narrowsSearch(search.joinStyle, search.playstyle));

  // Picking the selected join style again clears it, back to the default.
  function pickJoinStyle(style: JoinStyle) {
    search.joinStyle = search.joinStyle === style ? null : style;
  }

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
  <fieldset class="segmented" data-tour="search-by">
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
      <div class="field" data-tour="search-field">
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
        <label class="switch">
          <input type="checkbox" role="switch" bind:checked={remember} />
          <span class="track" aria-hidden="true"></span>
          Remember my trainer ID on this device
        </label>
      </div>
    {:else}
      <div class="field" data-tour="search-field">
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
      <div class="group" data-tour="balance">
        <label class="switch">
          <input type="checkbox" role="switch" bind:checked={search.balance} />
          <span class="track" aria-hidden="true"></span>
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
      </div>
      <div class="group" data-tour="filters">
        <label class="switch">
          <input type="checkbox" role="switch" bind:checked={search.includeFull} />
          <span class="track" aria-hidden="true"></span>
          Include full clubs
        </label>
        <label class="switch">
          <input type="checkbox" role="switch" bind:checked={search.hasDiscord} />
          <span class="track" aria-hidden="true"></span>
          Has Discord
        </label>

        <span class="label" id="join-style-label">Join style</span>
        <div class="join-styles" role="group" aria-labelledby="join-style-label">
          {#each JOIN_STYLES as style (style)}
            <button
              type="button"
              class="join {style}"
              aria-pressed={search.joinStyle === style}
              onclick={() => pickJoinStyle(style)}
            >
              <svg
                viewBox="0 0 16 16"
                width="14"
                height="14"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                {#if style === 'open'}
                  <rect x="3" y="7" width="10" height="7" rx="1.5" />
                  <path d="M5.5 7V4.5a2.5 2.5 0 0 1 4.8-1" />
                {:else if style === 'approval'}
                  <circle cx="6" cy="5" r="2.5" />
                  <path d="M1.5 14c.5-2.5 2.3-4 4.5-4s4 1.5 4.5 4" />
                  <path d="m10.5 7.5 1.5 1.5 3-3" />
                {:else}
                  <rect x="3" y="7" width="10" height="7" rx="1.5" />
                  <path d="M5.5 7V4.5a2.5 2.5 0 0 1 5 0V7" />
                {/if}
              </svg>
              {joinStyleLabel(style)}
            </button>
          {/each}
        </div>

        <span class="label" id="playstyle-label">Playstyle</span>
        <Dropdown
          id="playstyle"
          labelledby="playstyle-label"
          options={PLAYSTYLE_OPTIONS}
          bind:value={search.playstyle}
        />

        {#if narrowed}
          <p class="warning" data-testid="filter-warning">
            Filtering by join style or playstyle leaves fewer clubs to choose from, so your best
            matches may be further from your pace than usual.
          </p>
        {/if}
      </div>
      {#if search.hasDiscord}
        <p class="warning" data-testid="discord-warning">{DISCORD_NOTE}</p>
      {/if}
    </div>
  </div>

  <div class="actions">
    <button class="primary" data-tour="find" type="submit" disabled={busy}
      >{busy ? 'Searching…' : 'Find clubs'}</button
    >
  </div>
</form>

<style>
  /* Two columns (field, options) on wide screens; one column below 720px. Translucent, so the
     background artwork shows through. */
  /* Raised above the cards after it, so the playstyle list can open over them. */
  .search {
    position: relative;
    z-index: 2;
    display: grid;
    gap: 16px;
    padding: 16px;
    background: color-mix(in srgb, var(--card) 55%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    backdrop-filter: blur(2px);
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

  /* Groups the tour highlights, laid out like the rest of the options column. */
  .options .group {
    display: grid;
    gap: 6px;
    align-content: start;
  }

  .options .switch + .slider-label {
    margin-top: 2px;
  }

  .options .group + .group {
    margin-top: 6px;
  }

  .options .label {
    margin-top: 6px;
  }

  .join-styles {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .join {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    padding: 4px 12px;
    background: var(--card);
    border-color: var(--border-strong);
    border-radius: 999px;
    color: var(--text-2);
  }

  .join[aria-pressed='true'] {
    border-color: var(--style);
    background: color-mix(in srgb, var(--style) 15%, var(--card));
    color: var(--style);
    font-weight: 600;
  }

  .join[aria-pressed='true']:hover:not(:disabled) {
    background: color-mix(in srgb, var(--style) 22%, var(--card));
  }

  .join.open {
    --style: var(--open);
  }

  .join.approval {
    --style: var(--approval);
  }

  .join.closed {
    --style: var(--closed);
  }

  .join:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .warning {
    margin: 0;
    padding: 8px 10px;
    border: 1px solid color-mix(in srgb, var(--approval) 45%, transparent);
    border-radius: var(--radius);
    color: var(--text-2);
    font-size: 0.8125rem;
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
