<!--
  @component
  A modal dialog with a title and a close button. Focus stays inside it; Escape, the close button
  or a click outside closes it, and focus goes back to where it was.
-->

<script lang="ts">
  import type { Snippet } from 'svelte';

  import { t } from './i18n';

  interface Props {
    /** ID prefix for the dialog's title, so the page can have more than one. */
    id: string;
    title: string;
    /** What the heading shows in place of the plain `title`, such as a link. */
    heading?: Snippet;
    /** Called when the dialog is closed. */
    onclose: () => void;
    children: Snippet;
  }

  let { id, title, heading, onclose, children }: Props = $props();

  let panel: HTMLDivElement | undefined = $state();
  let closeButton: HTMLButtonElement | undefined = $state();
  let returnFocus: HTMLElement | null = null;

  function close() {
    onclose();
    returnFocus?.focus?.();
  }

  // Keeps Tab inside the panel, and closes on Escape, unless something inside, such as an open
  // dropdown, already used it.
  function onkeydown(event: KeyboardEvent) {
    if (event.defaultPrevented) {
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    } else if (event.key === 'Tab' && panel) {
      const stops = [
        ...panel.querySelectorAll<HTMLElement>(
          'button:not([disabled]), a[href], input:not([disabled]), [tabindex="0"]',
        ),
      ];
      const first = stops[0];
      const final = stops[stops.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        final?.focus();
      } else if (!event.shiftKey && document.activeElement === final) {
        event.preventDefault();
        first?.focus();
      }
    }
  }

  // The page behind doesn't scroll while the dialog is open.
  $effect(() => {
    returnFocus = document.activeElement as HTMLElement | null;
    closeButton?.focus();
    const root = document.documentElement;
    const overflow = root.style.overflow;
    root.style.overflow = 'hidden';
    return () => {
      root.style.overflow = overflow;
    };
  });
</script>

<svelte:window {onkeydown} />

<!-- Escape is the keyboard way out; the backdrop click is a pointer shortcut for the same. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="overlay"
  onclick={(event) => {
    if (event.target === event.currentTarget) close();
  }}
>
  <div class="panel" role="dialog" aria-modal="true" aria-labelledby="{id}-title" bind:this={panel}>
    <header>
      <h2 id="{id}-title">
        {#if heading}{@render heading()}{:else}{title}{/if}
      </h2>
      <button
        type="button"
        class="close"
        aria-label={t('modal.close')}
        bind:this={closeButton}
        onclick={close}
      >
        <svg
          viewBox="0 0 16 16"
          width="16"
          height="16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          aria-hidden="true"
        >
          <path d="m4 4 8 8M12 4l-8 8" />
        </svg>
      </button>
    </header>
    <div class="body">
      {@render children()}
    </div>
  </div>
</div>

<style>
  /* Above the sticky navbar, below the tour. */
  /* The overlay scrolls, not the panel, so an open dropdown's list is never cut off. */
  .overlay {
    position: fixed;
    z-index: 900;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    overflow-y: auto;
    background: rgb(0 0 0 / 55%);
    backdrop-filter: blur(3px);
  }

  .panel {
    display: flex;
    flex-direction: column;
    width: min(640px, 100%);
    margin: auto 0;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: 16px;
    box-shadow: 0 24px 64px rgb(0 0 0 / 35%);
  }

  header {
    display: flex;
    flex: none;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 16px 16px 12px 24px;
    border-bottom: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: 1.25rem;
    letter-spacing: -0.01em;
  }

  .close {
    display: grid;
    flex: none;
    place-items: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: var(--surface);
    border-color: var(--border);
    border-radius: 999px;
    color: var(--text-2);
  }

  .body {
    padding: 20px 24px 24px;
  }
</style>
