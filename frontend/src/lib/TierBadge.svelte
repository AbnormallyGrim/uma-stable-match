<!--
  @component
  A club's tier as its in-game icon (Cygames artwork, see `assets/tiers/README.md`).
  Unknown tiers show a dashed "?" with screen-reader text.
-->

<script lang="ts">
  import { tierIndex } from './format';

  const icons = import.meta.glob<string>('../assets/tiers/tier-*.webp', {
    eager: true,
    import: 'default',
  });

  interface Props {
    tier: string | null;
    size?: 'large' | 'small';
  }

  let { tier, size = 'large' }: Props = $props();

  const index = $derived(tierIndex(tier));
  const src = $derived(index >= 0 ? icons[`../assets/tiers/tier-${index + 1}.webp`] : undefined);
  const pixels = $derived(size === 'large' ? 48 : 26);
</script>

{#if src && tier}
  <img
    class="tier {size}"
    {src}
    alt="Club tier {tier}"
    title="Club tier {tier}"
    width={pixels}
    height={Math.round(pixels * (74 / 64))}
  />
{:else}
  <span class="tier unknown {size}" title="Club tier unknown">
    <span aria-hidden="true">?</span>
    <span class="visually-hidden">Club tier unknown</span>
  </span>
{/if}

<style>
  .tier {
    display: inline-block;
    flex: none;
  }

  .unknown {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--muted);
    font-weight: 700;
  }

  .unknown.large {
    width: 48px;
    height: 56px;
  }

  .unknown.small {
    width: 26px;
    height: 30px;
  }
</style>
