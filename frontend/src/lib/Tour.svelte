<!--
  @component
  The product tour: a dimmed overlay that highlights one part of the page at a time, with a card
  of instructions and Back, Next and Skip buttons. It's a modal dialog: focus stays inside it,
  Escape skips it, and the arrow keys step through it.
-->

<script lang="ts">
  import { tick } from 'svelte';

  import { t } from './i18n';
  import type { TourStep } from './tour';

  interface Props {
    steps: TourStep[];
    /** Called when the tour is finished or skipped. */
    onclose: () => void;
  }

  let { steps, onclose }: Props = $props();

  const GAP = 12;
  const MARGIN = 16;
  const PADDING = 6;

  let index = $state(0);
  let rect = $state<DOMRect | null>(null);
  let card: HTMLDivElement | undefined = $state();
  let cardHeight = $state(0);
  let cardWidth = $state(0);
  let viewport = $state({ width: window.innerWidth, height: window.innerHeight });
  let returnFocus: HTMLElement | null = null;

  const step = $derived(steps[index]!);
  const last = $derived(index === steps.length - 1);

  const spotlight = $derived(
    rect && {
      top: rect.top - PADDING,
      left: rect.left - PADDING,
      width: rect.width + PADDING * 2,
      height: rect.height + PADDING * 2,
    },
  );

  // Below the highlight if it fits, otherwise above, otherwise centred; always on screen.
  const position = $derived.by(() => {
    const { width, height } = viewport;
    if (!spotlight) {
      return { top: Math.max(MARGIN, (height - cardHeight) / 2), left: (width - cardWidth) / 2 };
    }
    const below = spotlight.top + spotlight.height + GAP;
    const above = spotlight.top - GAP - cardHeight;
    const top =
      below + cardHeight <= height - MARGIN
        ? below
        : above >= MARGIN
          ? above
          : Math.max(MARGIN, height - cardHeight - MARGIN);
    const left = Math.min(Math.max(MARGIN, spotlight.left), width - cardWidth - MARGIN);
    return { top, left: Math.max(MARGIN, left) };
  });

  function target(): HTMLElement | null {
    return step.target ? document.querySelector(`[data-tour="${step.target}"]`) : null;
  }

  function measure() {
    viewport = { width: window.innerWidth, height: window.innerHeight };
    cardHeight = card?.offsetHeight ?? 0;
    cardWidth = card?.offsetWidth ?? 0;
    const element = target();
    rect = element ? element.getBoundingClientRect() : null;
  }

  async function show() {
    const element = target();
    const reduce = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    element?.scrollIntoView?.({ block: 'center', behavior: reduce ? 'auto' : 'smooth' });
    await tick();
    measure();
    card?.querySelector<HTMLElement>('.primary')?.focus();
  }

  function go(to: number) {
    index = Math.min(steps.length - 1, Math.max(0, to));
    show();
  }

  function close() {
    onclose();
    returnFocus?.focus?.();
  }

  // Keeps Tab inside the card, and handles Escape and the arrow keys.
  function onkeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      if (last) close();
      else go(index + 1);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      go(index - 1);
    } else if (event.key === 'Tab' && card) {
      const buttons = [...card.querySelectorAll<HTMLElement>('button:not(:disabled)')];
      const first = buttons[0];
      const final = buttons[buttons.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        final?.focus();
      } else if (!event.shiftKey && document.activeElement === final) {
        event.preventDefault();
        first?.focus();
      }
    }
  }

  $effect(() => {
    returnFocus = document.activeElement as HTMLElement | null;
    show();
  });
</script>

<svelte:window onresize={measure} onscroll={measure} {onkeydown} />

<div class="tour">
  <!-- The dimmed page, with a rounded hole cut out around the highlighted part. -->
  <svg class="backdrop" width="100%" height="100%" aria-hidden="true">
    <defs>
      <mask id="tour-hole">
        <rect width="100%" height="100%" fill="white" />
        {#if spotlight}
          <rect
            x={spotlight.left}
            y={spotlight.top}
            width={spotlight.width}
            height={spotlight.height}
            rx="12"
            fill="black"
          />
        {/if}
      </mask>
    </defs>
    <rect width="100%" height="100%" mask="url(#tour-hole)" />
  </svg>
  {#if spotlight}
    <div
      class="spotlight"
      style:top="{spotlight.top}px"
      style:left="{spotlight.left}px"
      style:width="{spotlight.width}px"
      style:height="{spotlight.height}px"
    ></div>
  {/if}

  <div
    class="card"
    role="dialog"
    aria-modal="true"
    aria-labelledby="tour-title"
    aria-describedby="tour-body"
    bind:this={card}
    style:top="{position.top}px"
    style:left="{position.left}px"
  >
    <p class="count">{t('tour.step', { n: index + 1, total: steps.length })}</p>
    <h2 id="tour-title">{step.title}</h2>
    <p id="tour-body">{step.body}</p>
    <div class="dots" aria-hidden="true">
      {#each steps.keys() as i (i)}
        <span class:current={i === index}></span>
      {/each}
    </div>
    <div class="actions">
      {#if !last}
        <button type="button" class="skip" onclick={close}>{t('tour.skip')}</button>
      {/if}
      <span class="spacer"></span>
      {#if index > 0}
        <button type="button" onclick={() => go(index - 1)}>{t('tour.back')}</button>
      {/if}
      <button type="button" class="primary" onclick={() => (last ? close() : go(index + 1))}
        >{last ? t('tour.done') : t('tour.next')}</button
      >
    </div>
  </div>
</div>

<style>
  /* Above everything, including the sticky navbar. Clicks on the page are blocked meanwhile. */
  .tour {
    position: fixed;
    z-index: 1000;
    inset: 0;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    fill: rgb(0 0 0 / 55%);
  }

  /* An accent ring around the highlighted part, over the hole in the backdrop. */
  .spotlight {
    position: absolute;
    border: 2px solid var(--accent);
    border-radius: var(--radius);
    pointer-events: none;
  }

  .card {
    position: absolute;
    width: min(360px, calc(100vw - 32px));
    padding: 16px;
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 12px 32px rgb(0 0 0 / 30%);
    transition:
      top 0.25s,
      left 0.25s;
  }

  .count {
    margin: 0;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  h2 {
    margin: 4px 0 6px;
    font-size: 1.05rem;
  }

  #tour-body {
    margin: 0;
    color: var(--text-2);
    font-size: 0.9rem;
    line-height: 1.5;
  }

  .dots {
    display: flex;
    gap: 5px;
    margin-top: 14px;
  }

  .dots span {
    width: 6px;
    height: 6px;
    background: var(--border-strong);
    border-radius: 50%;
  }

  .dots span.current {
    width: 16px;
    background: var(--accent);
    border-radius: 999px;
  }

  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 14px;
  }

  .spacer {
    flex: 1;
  }

  .skip {
    padding-inline: 0;
    background: none;
    border-color: transparent;
    color: var(--muted);
  }

  .skip:hover:not(:disabled) {
    background: none;
    color: var(--text);
    text-decoration: underline;
  }

  @media (prefers-reduced-motion: reduce) {
    .card {
      transition: none;
    }
  }
</style>
