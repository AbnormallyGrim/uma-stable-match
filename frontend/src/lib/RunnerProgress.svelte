<!--
  @component
  A running figure moves along a track while a search runs. The bar is only decorative: it eases
  towards the end in time (see `progress.ts`) and starts over each time it's shown. `finished`
  says whether the search succeeded when the bar leaves, in which case the runner dashes to the
  end first.
-->

<script lang="ts">
  import { progressAt } from './progress';

  interface Props {
    finished?: () => boolean;
  }

  let { finished = () => false }: Props = $props();

  const FINISH_MS = 250;

  let progress = $state(0);

  $effect(() => {
    const start = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      progress = progressAt(now - start);
      frame = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(frame);
  });

  function finish(node: Element) {
    const reduce = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    if (!finished() || reduce || typeof node.animate !== 'function') {
      return { duration: 0 };
    }
    const from = progress;
    return {
      duration: FINISH_MS,
      css: (_t: number, u: number) => `--progress: ${from + (1 - from) * u}`,
    };
  }
</script>

<div class="track" style:--progress={progress} out:finish|global aria-hidden="true">
  <div class="fill"></div>
  <div class="runner"></div>
</div>

<style>
  .track {
    --runner-w: 69.5px;
    --runner-h: 48px;
    --gold: #b09551;

    position: relative;
    height: var(--runner-h);
    border-bottom: 3px solid var(--border);
  }

  .fill {
    position: absolute;
    bottom: -3px;
    left: 0;
    width: calc(var(--runner-w) / 2 + (100% - var(--runner-w)) * var(--progress));
    height: 3px;
    background: var(--gold);
    border-radius: 2px;
  }

  .runner {
    position: absolute;
    bottom: 0;
    left: calc((100% - var(--runner-w)) * var(--progress));
    width: var(--runner-w);
    height: var(--runner-h);
    background: url('../assets/runner/runner.webp') 0 0 / calc(var(--runner-w) * 8) 100% no-repeat;
    animation: run 0.4s steps(8) infinite;
  }

  @keyframes run {
    to {
      background-position-x: calc(var(--runner-w) * -8);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .runner {
      animation: none;
    }
  }
</style>
