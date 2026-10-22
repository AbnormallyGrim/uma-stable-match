<!--
  @component
  The search at a glance: pace used, clubs analysed, comfortable fits and typical place.
-->

<script lang="ts">
  import type { Recommendations } from './api';
  import { formatPerDay } from './format';
  import { comfortableFits, formatTopShare, typicalPlace } from './stats';

  interface Props {
    result: Recommendations;
  }

  let { result }: Props = $props();

  const fits = $derived(comfortableFits(result.clubs));
  const typical = $derived(formatTopShare(typicalPlace(result.clubs)));
</script>

<dl class="stats" data-testid="result-summary">
  <div class="stat">
    <dt>Your pace</dt>
    <dd>{formatPerDay(result.trainer.pace_per_day)}</dd>
  </div>
  <div class="stat">
    <dt>Clubs analysed</dt>
    <dd>{result.total}</dd>
  </div>
  <div class="stat">
    <dt>Comfortable fits</dt>
    <dd title="Clubs where you'd sit 25% to 60% of the way down the members">{fits}</dd>
  </div>
  <div class="stat">
    <dt>Typical place</dt>
    <dd title="Your middle place across these clubs">{typical}</dd>
  </div>
</dl>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 10px;
    margin: 20px 0 0;
  }

  .stat {
    min-width: 0;
    padding: 12px 14px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  dt {
    color: var(--text-2);
    font-size: 0.75rem;
  }

  dd {
    margin: 2px 0 0;
    overflow: hidden;
    font-size: 1.25rem;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 720px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
