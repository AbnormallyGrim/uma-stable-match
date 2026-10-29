<!--
  @component
  The search at a glance: pace used, clubs analysed, comfortable fits and typical place.
-->

<script lang="ts">
  import type { Recommendations } from './api';
  import { formatPerDay, formatUpdated } from './format';
  import { t } from './i18n';
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
    <dt>{t('summary.pace')}</dt>
    <dd>{formatPerDay(result.trainer.pace_per_day)}</dd>
  </div>
  <div class="stat">
    <dt>{t('summary.analysed')}</dt>
    <dd>{result.coverage.considered}</dd>
  </div>
  <div class="stat">
    <dt>{t('summary.comfortable')}</dt>
    <dd title={t('summary.comfortableTitle')}>{fits}</dd>
  </div>
  <div class="stat">
    <dt>{t('summary.typical')}</dt>
    <dd title={t('summary.typicalTitle')}>{typical}</dd>
  </div>
</dl>
<p class="data-age" data-testid="data-age">
  {t('summary.updated')}
  <time datetime={result.updated_at} title={result.updated_at}
    >{formatUpdated(result.updated_at)}</time
  >.
</p>

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

  .data-age {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 0.8125rem;
  }

  @media (max-width: 720px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
