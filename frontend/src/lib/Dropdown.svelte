<!--
  @component
  A themed single-choice dropdown: a button that opens a list of options. It follows the ARIA
  select-only combobox pattern, so it works with the keyboard and screen readers like a native
  `<select>`, but the open list matches the site instead of the browser's.
-->

<script lang="ts" generics="T">
  import { tick } from 'svelte';

  interface Option {
    value: T;
    label: string;
  }

  interface Props {
    id: string;
    /** ID of the element that labels the dropdown. */
    labelledby: string;
    options: Option[];
    value: T;
  }

  let { id, labelledby, options, value = $bindable() }: Props = $props();

  let open = $state(false);
  let active = $state(0);
  let root: HTMLDivElement | undefined = $state();
  let list: HTMLUListElement | undefined = $state();
  let typed = '';
  let typedAt = 0;

  const selectedIndex = $derived(
    Math.max(
      0,
      options.findIndex((o) => o.value === value),
    ),
  );
  const selectedLabel = $derived(options[selectedIndex]?.label ?? '');

  async function show() {
    active = selectedIndex;
    open = true;
    await tick();
    scrollToActive();
  }

  function hide() {
    open = false;
  }

  function choose(index: number) {
    const option = options[index];
    if (option) {
      value = option.value;
    }
    hide();
  }

  function scrollToActive() {
    list?.querySelector(`#${id}-option-${active}`)?.scrollIntoView?.({ block: 'nearest' });
  }

  function move(to: number) {
    active = Math.min(options.length - 1, Math.max(0, to));
    scrollToActive();
  }

  // Typing letters jumps to the next option starting with them, like a native select. Repeating
  // one letter cycles through the options starting with it.
  function typeahead(key: string) {
    const now = Date.now();
    typed = now - typedAt < 700 ? typed + key.toLowerCase() : key.toLowerCase();
    typedAt = now;
    const repeated = [...typed].every((c) => c === typed[0]);
    const search = repeated ? typed[0]! : typed;
    const start = repeated ? active + 1 : active;
    for (let i = 0; i < options.length; i++) {
      const index = (start + i) % options.length;
      if (options[index]!.label.toLowerCase().startsWith(search)) {
        move(index);
        return true;
      }
    }
    return false;
  }

  function onkeydown(event: KeyboardEvent) {
    const key = event.key;
    if (!open) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(key)) {
        event.preventDefault();
        show();
      }
      return;
    }
    const handled = (() => {
      switch (key) {
        case 'ArrowDown':
          move(active + 1);
          return true;
        case 'ArrowUp':
          move(active - 1);
          return true;
        case 'Home':
          move(0);
          return true;
        case 'End':
          move(options.length - 1);
          return true;
        case 'PageDown':
          move(active + 8);
          return true;
        case 'PageUp':
          move(active - 8);
          return true;
        case 'Enter':
        case ' ':
          choose(active);
          return true;
        case 'Escape':
          hide();
          return true;
        case 'Tab':
          choose(active);
          return false;
        default:
          return key.length === 1 && typeahead(key);
      }
    })();
    if (handled) {
      event.preventDefault();
    }
  }

  function onwindowpointerdown(event: PointerEvent) {
    if (open && root && !root.contains(event.target as Node)) {
      hide();
    }
  }
</script>

<svelte:window onpointerdown={onwindowpointerdown} />

<div class="dropdown" bind:this={root}>
  <button
    {id}
    type="button"
    class="trigger"
    role="combobox"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls="{id}-list"
    aria-labelledby="{labelledby} {id}"
    aria-activedescendant={open ? `${id}-option-${active}` : undefined}
    onclick={() => (open ? hide() : show())}
    {onkeydown}
  >
    <span class="value">{selectedLabel}</span>
    <svg
      class="chevron"
      viewBox="0 0 12 12"
      width="12"
      height="12"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"><path d="m3 4.5 3 3 3-3" /></svg
    >
  </button>
  {#if open}
    <ul id="{id}-list" class="list" role="listbox" aria-labelledby={labelledby} bind:this={list}>
      {#each options as option, i (i)}
        <!-- Keyboard focus stays on the button, which handles keys for the list. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          id="{id}-option-{i}"
          role="option"
          aria-selected={i === selectedIndex}
          class:active={i === active}
          onpointermove={() => (active = i)}
          onclick={() => choose(i)}
        >
          <span>{option.label}</span>
          {#if i === selectedIndex}
            <svg
              class="check"
              viewBox="0 0 12 12"
              width="12"
              height="12"
              fill="none"
              stroke="currentColor"
              stroke-width="1.75"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"><path d="m2.5 6.5 2.5 2.5 4.5-5" /></svg
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .dropdown {
    position: relative;
    width: 100%;
    max-width: 320px;
  }

  .trigger {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 7px 10px;
    background: var(--card);
    border: 1px solid var(--control-border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 1rem;
    font-weight: 400;
    text-align: left;
  }

  .trigger:hover:not(:disabled) {
    background: var(--card);
    border-color: var(--border-strong);
  }

  .trigger[aria-expanded='true'] {
    border-color: var(--accent);
  }

  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chevron {
    flex: none;
    color: var(--muted);
    transition: transform 0.15s;
  }

  .trigger[aria-expanded='true'] .chevron {
    transform: rotate(180deg);
  }

  .list {
    position: absolute;
    z-index: 10;
    top: calc(100% + 4px);
    right: 0;
    left: 0;
    max-height: 280px;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: 0 8px 24px rgb(0 0 0 / 18%);
    list-style: none;
  }

  li {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    border-radius: calc(var(--radius-sm) - 2px);
    color: var(--text-2);
    font-size: 0.875rem;
    cursor: pointer;
  }

  li.active {
    background: var(--surface);
    color: var(--text);
  }

  li[aria-selected='true'] {
    color: var(--accent);
    font-weight: 600;
  }

  .check {
    flex: none;
  }

  @media (max-width: 720px) {
    .dropdown {
      max-width: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron {
      transition: none;
    }
  }
</style>
