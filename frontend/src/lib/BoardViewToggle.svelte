<!--
  @component
  Grid / Table switch for the board, as icons with tooltips.
-->

<script lang="ts">
  import type { BoardView } from './ads';
  import { t } from './i18n';

  interface Props {
    view: BoardView;
  }

  let { view = $bindable() }: Props = $props();

  const options: BoardView[] = ['grid', 'table'];
</script>

<fieldset class="segmented small layout">
  <legend class="visually-hidden">{t('ads.layout.label')}</legend>
  {#each options as option (option)}
    <label class="tooltip" data-tooltip={t(`ads.layout.${option}`)}>
      <input type="radio" name="board-view" value={option} bind:group={view} />
      <svg
        viewBox="0 0 16 16"
        width="16"
        height="16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        {#if option === 'grid'}
          <rect x="2" y="2" width="5" height="5" rx="1" />
          <rect x="9" y="2" width="5" height="5" rx="1" />
          <rect x="2" y="9" width="5" height="5" rx="1" />
          <rect x="9" y="9" width="5" height="5" rx="1" />
        {:else}
          <path d="M2 4h12M2 8h12M2 12h12" />
        {/if}
      </svg>
      <span class="visually-hidden">{t(`ads.layout.${option}`)}</span>
    </label>
  {/each}
</fieldset>

<style>
  .layout label {
    display: inline-flex;
    padding: 6px 9px;
  }
</style>
