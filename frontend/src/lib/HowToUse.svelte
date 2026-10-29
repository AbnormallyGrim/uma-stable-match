<!--
  @component
  A collapsible "How to use" section: finding the trainer ID or daily pace, the options, and picking
  a club. Closed by default. Keep the labels it quotes in step with the search form and results.
-->

<script lang="ts">
  import { t } from './i18n';

  const RANKINGS = 'https://uma.moe/rankings';
</script>

<!-- eslint-disable svelte/no-at-html-tags -- first-party text from the message catalog -->
<details class="how-to" data-tour="how-to">
  <summary>{t('howTo.summary')}</summary>
  <ol>
    <li>{@html t('howTo.step1')}</li>
    <li>{@html t('howTo.step2')}</li>
    <li>{@html t('howTo.step3', { rankings: RANKINGS })}</li>
    <li>{@html t('howTo.step4')}</li>
    <li>{@html t('howTo.step5')}</li>
  </ol>
  <p class="note">{t('howTo.note')}</p>
</details>

<style>
  .how-to {
    margin-top: 12px;
    padding: 0 16px;
    background: color-mix(in srgb, var(--card) 55%, transparent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    backdrop-filter: blur(2px);
  }

  summary {
    padding: 12px 0;
    font-weight: 600;
    cursor: pointer;
  }

  ol {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0 0 0 20px;
  }

  li {
    line-height: 1.5;
  }

  .how-to :global(a) {
    color: var(--accent);
  }

  .note {
    margin: 12px 0 16px;
    color: var(--muted);
    font-size: 0.875rem;
  }
</style>
