<!--
  @component
  The release notes: a modal dialog listing every release as a timeline, newest first, each with
  its build number, date and changes. Focus stays inside it; Escape, the close button or a click
  outside closes it.
-->

<script lang="ts">
  import { formatDate } from './format';
  import { t } from './i18n';
  import type { Release } from './releases';

  interface Props {
    releases: Release[];
    /** Called when the notes are closed. */
    onclose: () => void;
  }

  let { releases, onclose }: Props = $props();

  let panel: HTMLDivElement | undefined = $state();
  let closeButton: HTMLButtonElement | undefined = $state();
  let returnFocus: HTMLElement | null = null;

  function close() {
    onclose();
    returnFocus?.focus?.();
  }

  // Keeps Tab inside the panel, and closes on Escape.
  function onkeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    } else if (event.key === 'Tab' && panel) {
      const stops = [...panel.querySelectorAll<HTMLElement>('button, [tabindex="0"]')];
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

  // The page behind doesn't scroll while the notes are open.
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
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby="releases-title"
    aria-describedby="releases-intro"
    bind:this={panel}
  >
    <header>
      <div class="heading">
        <h2 id="releases-title">{t('releases.title')}</h2>
        <p id="releases-intro">{t('releases.intro')}</p>
      </div>
      {#if releases[0]}
        <span class="build">{t('releases.current', { build: releases[0].build })}</span>
      {/if}
      <button
        type="button"
        class="close"
        aria-label={t('releases.close')}
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

    <!-- A scrolling region has to be reachable by keyboard. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="scroll" tabindex="0" role="region" aria-labelledby="releases-title">
      <ol class="timeline">
        {#each releases as release, index (release.build)}
          <li class="release">
            <div class="meta">
              <h3>{release.build}</h3>
              <span class="kind {release.kind}">{t(`releases.kind.${release.kind}`)}</span>
              {#if index === 0}
                <span class="latest">{t('releases.latest')}</span>
              {/if}
              <time datetime={release.date}>{formatDate(release.date)}</time>
            </div>
            <ul class="changes">
              {#each release.changes as change (change.title)}
                <li>
                  <span class="chip {change.kind}">{t(`releases.change.${change.kind}`)}</span>
                  <div>
                    <h4>{t(change.title)}</h4>
                    <p>{t(change.body)}</p>
                  </div>
                </li>
              {/each}
            </ul>
          </li>
        {/each}
      </ol>
    </div>
  </div>
</div>

<style>
  /* Above the sticky navbar, below the tour. */
  .overlay {
    position: fixed;
    z-index: 900;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 55%);
    backdrop-filter: blur(3px);
    animation: fade 0.15s ease-out;
  }

  .panel {
    display: flex;
    flex-direction: column;
    width: min(640px, 100%);
    max-height: min(780px, 100%);
    overflow: hidden;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: 16px;
    box-shadow: 0 24px 64px rgb(0 0 0 / 35%);
    animation: rise 0.2s ease-out;
  }

  header {
    display: flex;
    flex: none;
    gap: 12px;
    align-items: flex-start;
    padding: 20px 20px 16px 24px;
    border-bottom: 1px solid var(--border);
  }

  .heading {
    flex: 1;
    min-width: 0;
  }

  h2 {
    margin: 0;
    font-size: 1.25rem;
    letter-spacing: -0.01em;
  }

  .heading p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 0.875rem;
  }

  .build {
    flex: none;
    padding: 3px 10px;
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
    border-radius: 999px;
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* Round, like the buttons in the navbar. */
  .close {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    margin-top: -4px;
    padding: 0;
    background: var(--surface);
    border-color: var(--border);
    border-radius: 999px;
    color: var(--text-2);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 24px 24px 28px;
    overscroll-behavior: contain;
  }

  /* A rail down the left with a dot at each release. */
  .timeline {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .release {
    position: relative;
    padding: 0 0 28px 28px;
  }

  .release:last-child {
    padding-bottom: 0;
  }

  .release::before {
    position: absolute;
    top: 8px;
    bottom: -8px;
    left: 5px;
    width: 2px;
    background: var(--border-strong);
    content: '';
  }

  .release:last-child::before {
    display: none;
  }

  .release::after {
    position: absolute;
    top: 6px;
    left: 0;
    width: 12px;
    height: 12px;
    background: var(--card);
    border: 2px solid var(--control-border);
    border-radius: 50%;
    content: '';
  }

  .release:first-child::after {
    background: var(--accent);
    border-color: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    margin-bottom: 12px;
  }

  h3 {
    margin: 0;
    font-size: 1.05rem;
    font-variant-numeric: tabular-nums;
  }

  time {
    margin-left: auto;
    color: var(--muted);
    font-size: 0.8125rem;
  }

  .kind,
  .latest {
    padding: 1px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--text-2);
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .kind.hotfix {
    border-color: color-mix(in srgb, var(--approval) 50%, transparent);
    color: var(--approval);
  }

  .latest {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .changes {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .changes li {
    display: grid;
    grid-template-columns: 5.5rem 1fr;
    gap: 12px;
    padding: 12px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .chip {
    align-self: start;
    justify-self: start;
    padding: 1px 8px;
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    font-weight: 700;
  }

  .chip.new {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }

  .chip.improved {
    background: color-mix(in srgb, var(--open) 16%, transparent);
    color: var(--open);
  }

  .chip.fixed {
    background: color-mix(in srgb, var(--approval) 16%, transparent);
    color: var(--approval);
  }

  h4 {
    margin: 0 0 2px;
    font-size: 0.9375rem;
  }

  .changes p {
    margin: 0;
    color: var(--text-2);
    font-size: 0.875rem;
    line-height: 1.55;
  }

  @media (width <= 520px) {
    .overlay {
      padding: 8px;
    }

    .changes li {
      grid-template-columns: 1fr;
      gap: 6px;
    }

    .scroll {
      padding-inline: 16px;
    }

    header {
      padding-inline: 16px;
    }

    .build {
      display: none;
    }
  }

  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.98);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .panel {
      animation: none;
    }
  }
</style>
