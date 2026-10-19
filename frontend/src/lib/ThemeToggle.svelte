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

<fieldset class="theme">
  <legend class="visually-hidden">Theme</legend>
  {#each options as option (option.value)}
    <label class:selected={choice === option.value}>
      <input type="radio" name="theme" value={option.value} bind:group={choice} />
      {option.label}
    </label>
  {/each}
</fieldset>

<style>
  .theme {
    display: inline-flex;
    gap: 2px;
    margin: 0;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
  }

  label {
    position: relative;
    padding: 2px 10px;
    border-radius: 999px;
    color: var(--text-2);
    font-size: 0.75rem;
    cursor: pointer;
  }

  label.selected {
    background: var(--card);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--border);
  }

  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  label:has(input:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>
