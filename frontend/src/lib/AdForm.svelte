<!--
  @component
  What a trainer asks for in an ad: the club ranks they'd join, whether the club needs a Discord
  server and the language it speaks. Their fan numbers aren't asked for; the server adds them from
  uma.moe.
-->

<script lang="ts">
  import {
    LANGUAGE_CODES,
    MAX_RANK_SPAN,
    RANK_CODES,
    rankLabel,
    type AdChoices,
    type AdLanguage,
    type DiscordWish,
  } from './ads';
  import Dropdown from './Dropdown.svelte';
  import { t } from './i18n';

  interface Props {
    initial: AdChoices;
    /** The label of the button that sends the choices. */
    submitLabel: string;
    /** Whether the choices are being sent. */
    busy?: boolean;
    /** Why the last send failed, if it did. */
    error?: string | null;
    onsubmit: (choices: AdChoices) => void;
    /** Shown as a cancel button when given. */
    oncancel?: () => void;
  }

  let { initial, submitLabel, busy = false, error = null, onsubmit, oncancel }: Props = $props();

  // The form starts from the choices it's given and keeps its own from there.
  /* svelte-ignore state_referenced_locally */
  const start = initial;
  let rankMin = $state(start.rankMin);
  let rankMax = $state(start.rankMax);
  let discord = $state<DiscordWish>(start.discord);
  let language = $state<AdLanguage>(start.language);

  const WISHES: DiscordWish[] = ['required', 'preferred', 'any'];

  const rankOptions = $derived(RANK_CODES.map((code) => ({ value: code, label: rankLabel(code) })));
  const wishOptions = $derived(
    WISHES.map((wish) => ({ value: wish, label: t(`ad.discord.${wish}`) })),
  );
  const languageOptions = $derived(
    LANGUAGE_CODES.map((code) => ({ value: code, label: t(`ad.language.${code}`) })),
  );

  const ranksReversed = $derived(rankMin > rankMax);
  const ranksTooWide = $derived(rankMax - rankMin >= MAX_RANK_SPAN);
  const ranksWrong = $derived(ranksReversed || ranksTooWide);

  function submit(event: SubmitEvent) {
    event.preventDefault();
    if (ranksWrong || busy) {
      return;
    }
    onsubmit({ rankMin, rankMax, discord, language });
  }
</script>

<form onsubmit={submit} novalidate>
  <h2>{t('ad.form.heading')}</h2>

  <div class="row">
    <div class="field">
      <span class="label" id="ad-rank-min-label">{t('ad.form.rankMin')}</span>
      <Dropdown
        id="ad-rank-min"
        labelledby="ad-rank-min-label"
        options={rankOptions}
        bind:value={rankMin}
      />
    </div>
    <div class="field">
      <span class="label" id="ad-rank-max-label">{t('ad.form.rankMax')}</span>
      <Dropdown
        id="ad-rank-max"
        labelledby="ad-rank-max-label"
        options={rankOptions}
        bind:value={rankMax}
      />
    </div>
  </div>
  {#if ranksReversed}
    <p class="problem" role="alert">{t('ad.form.rankError')}</p>
  {:else if ranksTooWide}
    <p class="problem" role="alert">{t('ad.form.rankSpanError')}</p>
  {/if}

  <div class="row">
    <div class="field">
      <span class="label" id="ad-discord-label">{t('ad.form.discord')}</span>
      <Dropdown
        id="ad-discord"
        labelledby="ad-discord-label"
        options={wishOptions}
        bind:value={discord}
      />
    </div>
    <div class="field">
      <span class="label" id="ad-language-label">{t('ad.form.language')}</span>
      <Dropdown
        id="ad-language"
        labelledby="ad-language-label"
        options={languageOptions}
        bind:value={language}
      />
    </div>
  </div>

  {#if error}
    <p class="problem" role="alert">{error}</p>
  {/if}

  <div class="actions">
    <button type="submit" class="primary" disabled={busy || ranksWrong}>
      {submitLabel}
    </button>
    {#if oncancel}
      <button type="button" onclick={oncancel} disabled={busy}>{t('ad.form.cancel')}</button>
    {/if}
  </div>
</form>

<style>
  form {
    display: grid;
    gap: 14px;
  }

  h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }

  .field {
    display: grid;
    align-content: start;
    gap: 6px;
  }

  .label {
    color: var(--text-2);
    font-size: 0.8125rem;
    font-weight: 600;
  }

  .problem {
    margin: 0;
    color: var(--closed);
    font-size: 0.85rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
