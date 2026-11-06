<!--
  @component
  One application in a modal, laid out like the message the bot posted to Discord, with a link to
  its ticket channel. Escape or the backdrop closes it, and focus goes back where it was.
-->

<script lang="ts">
  import type { Application } from './applications';
  import { formatDate, formatMillions, formatPace } from './format';
  import { t } from './i18n';
  import { clubComparePath } from './route';

  interface Props {
    application: Application;
    /** The club's name, or `null` while it's unknown. */
    clubName: string | null;
    /** The club's ID, or `null` while it's unknown. */
    clubId: string | null;
    onclose: () => void;
  }

  let { application, clubName, clubId, onclose }: Props = $props();

  let closeButton: HTMLButtonElement | undefined = $state();
  let returnFocus: HTMLElement | null = null;

  function close() {
    onclose();
    returnFocus?.focus?.();
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      close();
    }
  }

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
  <div class="panel" role="dialog" aria-modal="true" aria-labelledby="application-title">
    <header>
      <h2 id="application-title">
        {t('leader.details.heading', { name: application.trainerName })}
      </h2>
      <button type="button" bind:this={closeButton} onclick={close}
        >{t('leader.details.close')}</button
      >
    </header>
    <p class="muted">{t('leader.details.intro')}</p>
    <div class="embed">
      <h3>
        {clubName === null
          ? t('leader.details.embedUnnamed')
          : t('leader.details.embedTitle', { name: clubName })}
      </h3>
      <dl>
        <div>
          <dt>{t('leader.table.trainer')}</dt>
          <dd>{application.trainerName}</dd>
        </div>
        <div>
          <dt>{t('leader.details.trainerId')}</dt>
          <dd>{application.trainerId}</dd>
        </div>
        <div>
          <dt>{t('leader.details.discord')}</dt>
          <dd class="mention">@{application.discordName}</dd>
        </div>
        <div>
          <dt>{t('leader.table.pace')}</dt>
          <dd>{formatPace(application.pace)}</dd>
        </div>
        <div>
          <dt>{t('leader.table.fans')}</dt>
          <dd>{formatMillions(application.fansThisMonth)}</dd>
        </div>
        <div>
          <dt>{t('leader.table.currentClub')}</dt>
          <dd>{application.currentClub ?? t('leader.table.noClub')}</dd>
        </div>
      </dl>
      <p class="footer">
        {t('leader.details.submitted', { date: formatDate(application.submittedAt.slice(0, 10)) })}
        · {t(`leader.applicationStatus.${application.status}`)}
      </p>
    </div>
    <div class="actions">
      {#if clubId !== null}
        <a class="provider-btn compare" href={clubComparePath(clubId, [application.trainerId])}>
          {t('leader.details.compare')}
        </a>
      {/if}
      {#if application.ticketUrl}
        <a
          class="provider-btn discord"
          href={application.ticketUrl}
          target="_blank"
          rel="noopener noreferrer"
        >
          {t('leader.details.openTicket')}
        </a>
      {/if}
    </div>
    {#if !application.ticketUrl}
      <p class="muted">{t('leader.details.ticketGone')}</p>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgb(0 0 0 / 55%);
  }

  .panel {
    display: grid;
    gap: 14px;
    width: min(560px, 100%);
    max-height: calc(100dvh - 32px);
    overflow-y: auto;
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  .embed {
    padding: 14px 16px;
    background: var(--surface);
    border-left: 4px solid #5865f2;
    border-radius: var(--radius-sm);
  }

  h3 {
    margin: 0 0 10px;
    font-size: 1rem;
  }

  dl {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 10px 16px;
    margin: 0;
  }

  dt {
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 700;
  }

  dd {
    margin: 0;
  }

  .mention {
    color: var(--discord);
  }

  .footer {
    margin: 12px 0 0;
    color: var(--muted);
    font-size: 0.75rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
  }

  .compare {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }

  .compare:hover {
    background: color-mix(in srgb, var(--accent) 88%, var(--text));
    border-color: color-mix(in srgb, var(--accent) 88%, var(--text));
  }
</style>
