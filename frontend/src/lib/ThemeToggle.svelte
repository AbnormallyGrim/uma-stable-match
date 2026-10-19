<!--
  @component
  System / Light / Dark switch. Applies the choice to the page and remembers it.
-->

<script lang="ts">
  import { untrack } from 'svelte';

  import { applyTheme, loadTheme, saveTheme, type ThemeChoice } from './theme';

  interface Props {
    storage?: Storage;
  }

  let { storage = typeof localStorage === 'undefined' ? undefined : localStorage }: Props =
    $props();

  const options: { value: ThemeChoice; label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ];

  let choice = $state<ThemeChoice>(untrack(() => loadTheme(storage)));

  $effect(() => {
    applyTheme(choice);
    saveTheme(storage, choice);
  });
</script>

<fieldset class="segmented small">
  <legend class="visually-hidden">Theme</legend>
  {#each options as option (option.value)}
    <label>
      <input type="radio" name="theme" value={option.value} bind:group={choice} />
      {option.label}
    </label>
  {/each}
</fieldset>
