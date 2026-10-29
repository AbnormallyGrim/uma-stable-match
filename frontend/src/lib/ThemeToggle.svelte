<!--
  @component
  System / Light / Dark switch, as icons with tooltips. Applies the choice to the page and
  remembers it.
-->

<script lang="ts">
  import { untrack } from 'svelte';

  import { t } from './i18n';
  import { applyTheme, loadTheme, saveTheme, type ThemeChoice } from './theme';

  interface Props {
    storage?: Storage;
  }

  let { storage = typeof localStorage === 'undefined' ? undefined : localStorage }: Props =
    $props();

  const options: ThemeChoice[] = ['system', 'light', 'dark'];

  let choice = $state<ThemeChoice>(untrack(() => loadTheme(storage)));

  $effect(() => {
    applyTheme(choice);
    saveTheme(storage, choice);
  });
</script>

<fieldset class="segmented theme" data-tour="theme">
  <legend class="visually-hidden">{t('theme.label')}</legend>
  {#each options as option (option)}
    <label class="tooltip" data-tooltip={t(`theme.${option}`)}>
      <input type="radio" name="theme" value={option} bind:group={choice} />
      <svg
        viewBox="0 0 16 16"
        width="16"
        height="16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        {#if option === 'system'}
          <rect x="1.75" y="2.5" width="12.5" height="8.5" rx="1.5" />
          <path d="M5.5 14h5M8 11v3" />
        {:else if option === 'light'}
          <circle cx="8" cy="8" r="3" />
          <path
            d="M8 1.5v1.5M8 13v1.5M1.5 8H3M13 8h1.5M3.4 3.4l1.06 1.06M11.54 11.54l1.06 1.06M3.4 12.6l1.06-1.06M11.54 4.46l1.06-1.06"
          />
        {:else}
          <path d="M13.5 9.5A5.5 5.5 0 0 1 6.5 2.5a5.5 5.5 0 1 0 7 7Z" />
        {/if}
      </svg>
      <span class="visually-hidden">{t(`theme.${option}`)}</span>
    </label>
  {/each}
</fieldset>

<style>
  .theme label {
    display: inline-flex;
    padding: 6px 9px;
  }
</style>
