<!--
  @component
  A dialog that points visitors to the Applications and Recruitment Board pages, shown once per
  device. It counts as seen as soon as it opens, so it never repeats on the next page.
-->

<script lang="ts">
  import { markAnnouncementSeen } from './announcement';
  import { t } from './i18n';
  import Modal from './Modal.svelte';
  import type { MessageKey } from './i18n';
  import { PAGES, type Page } from './pages';

  const storage = typeof localStorage === 'undefined' ? undefined : localStorage;

  const board = PAGES.find((page) => page.id === 'board')!;
  const applications = PAGES.find((page) => page.id === 'applications')!;

  let open = $state(true);

  markAnnouncementSeen(storage);
</script>

<!-- eslint-disable svelte/no-at-html-tags -- first-party icon drawings from pages.ts -->
{#snippet card(page: Page, description: MessageKey)}
  <a href={page.href}>
    <span class="tile">
      <svg
        viewBox="0 0 16 16"
        width="22"
        height="22"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true">{@html page.icon}</svg
      >
    </span>
    <span class="text">
      <strong>{t(page.label)}</strong>
      <span class="description">{t(description)}</span>
    </span>
    <svg
      class="arrow"
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"><path d="m6 3 5 5-5 5" /></svg
    >
  </a>
{/snippet}

{#if open}
  <Modal id="announcement" title={t('announce.title')} onclose={() => (open = false)}>
    <p class="lede">{t('announce.lede')}</p>
    <ul class="pages">
      <li>{@render card(board, 'announce.board')}</li>
      <li>{@render card(applications, 'announce.applications')}</li>
    </ul>
    <div class="actions">
      <button type="button" class="later" onclick={() => (open = false)}
        >{t('announce.dismiss')}</button
      >
    </div>
  </Modal>
{/if}

<style>
  .lede {
    margin: 0 0 16px;
    color: var(--text-2);
  }

  .pages {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .pages a {
    display: grid;
    grid-template-columns: 44px 1fr 16px;
    gap: 14px;
    align-items: center;
    padding: 14px;
    color: var(--text);
    text-decoration: none;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition:
      border-color 0.15s,
      transform 0.15s;
  }

  .pages a:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }

  .tile {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    color: var(--accent);
    background: var(--card);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
  }

  .text {
    display: grid;
  }

  .description {
    color: var(--muted);
    font-size: 0.88rem;
    line-height: 1.4;
  }

  .arrow {
    color: var(--muted);
  }

  .actions {
    display: flex;
    justify-content: center;
    margin-top: 16px;
  }

  .later {
    padding: 6px 10px;
    color: var(--muted);
    background: none;
    border-color: transparent;
  }

  .later:hover:not(:disabled) {
    background: none;
    color: var(--text);
    text-decoration: underline;
  }

  @media (prefers-reduced-motion: reduce) {
    .pages a {
      transition: none;
    }

    .pages a:hover {
      transform: none;
    }
  }
</style>
