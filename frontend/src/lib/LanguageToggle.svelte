<!--
  @component
  EN / JP switch. It only shows when the server has Japanese switched on, and remembers the choice.
-->

<script lang="ts">
  import { t } from './i18n';
  import type { Language } from './language';
  import { language } from './language.svelte';

  const options: { value: Language; short: string }[] = [
    { value: 'en', short: 'EN' },
    { value: 'ja', short: 'JP' },
  ];
</script>

{#if language.japaneseEnabled}
  <fieldset class="segmented language" data-tour="language">
    <legend class="visually-hidden">{t('language.label')}</legend>
    {#each options as option (option.value)}
      <label class="tooltip" data-tooltip={t(`language.${option.value}`)}>
        <input
          type="radio"
          name="language"
          value={option.value}
          aria-label={t(`language.${option.value}`)}
          checked={language.current === option.value}
          onchange={() => language.choose(option.value)}
        />
        <span aria-hidden="true">{option.short}</span>
      </label>
    {/each}
  </fieldset>
{/if}

<style>
  .language label {
    display: inline-flex;
    padding: 6px 10px;
    font-size: 0.8125rem;
    font-weight: 600;
  }
</style>
