<!--
  @component
  The bot's side of a linked club: whether players can apply and what stops them, the club's
  settings, and removing the bot. The server checks the leader's rights with Discord on every
  change, so this only shows what it's told.
-->

<script lang="ts">
  import {
    checkTicketName,
    previewTicketName,
    saveSettings,
    TICKET_NAME_MAX,
    unlinkClub,
    type Settings,
    type SettingsChange,
    type TicketNameMode,
  } from './applications';
  import { t } from './i18n';

  interface Props {
    clubId: string;
    /** The club's name, for the ticket name example. */
    clubName?: string | null;
    settings: Settings;
    /** Called with the new settings after a save. */
    onsaved: (settings: Settings) => void;
    /** Called once the bot is removed from the club. */
    onremoved: () => void;
  }

  let { clubId, clubName = null, settings, onsaved, onremoved }: Props = $props();

  let open = $state(true);
  let autoJoin = $state(true);
  let nameMode = $state<TicketNameMode>('default');
  let nameText = $state('');
  let saving = $state(false);
  let saved = $state(false);
  let saveFailed = $state(false);
  let confirming = $state(false);
  let removeFailed = $state(false);

  // The form starts from, and follows, what the server last said.
  $effect(() => {
    open = settings.open;
    autoJoin = settings.auto_join;
    nameMode = settings.ticket_name.mode;
    nameText = settings.ticket_name.value ?? '';
  });

  // What the server would store for the text so far, or what's wrong with it.
  const nameCheck = $derived(nameMode === 'default' ? null : checkTicketName(nameText, nameMode));
  const nameProblem = $derived(
    nameCheck !== null && 'problem' in nameCheck ? nameCheck.problem : null,
  );
  const example = $derived(
    previewTicketName(
      {
        mode: nameMode,
        value: nameCheck !== null && 'value' in nameCheck ? nameCheck.value : null,
      },
      clubName,
    ),
  );

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (nameProblem !== null) return;
    saving = true;
    saved = false;
    saveFailed = false;
    const change: SettingsChange = { open, auto_join: autoJoin };
    // Only a changed name is sent.
    const value = nameCheck !== null && 'value' in nameCheck ? nameCheck.value : null;
    if (nameMode !== settings.ticket_name.mode || value !== settings.ticket_name.value) {
      change.ticket_name = { mode: nameMode, value };
    }
    try {
      onsaved(await saveSettings(clubId, change));
      saved = true;
    } catch {
      saveFailed = true;
    } finally {
      saving = false;
    }
  }

  async function remove() {
    removeFailed = false;
    try {
      await unlinkClub(clubId);
      onremoved();
    } catch {
      removeFailed = true;
    } finally {
      confirming = false;
    }
  }
</script>

<section class="card" aria-labelledby="status-heading">
  <h2 id="status-heading">{t('leader.status.heading')}</h2>
  <ul class="checks">
    <li class="ok"><span class="icon" aria-hidden="true">✓</span>{t('leader.status.inServer')}</li>
    {#if settings.problem}
      <li class="bad">
        <span class="icon" aria-hidden="true">!</span>{t('leader.status.problem', {
          problem: settings.problem,
        })}
      </li>
    {/if}
    {#if settings.open}
      <li class="ok"><span class="icon" aria-hidden="true">✓</span>{t('leader.status.open')}</li>
    {:else}
      <li class="warn">
        <span class="icon" aria-hidden="true">‖</span>{t('leader.status.paused')}
      </li>
    {/if}
  </ul>
</section>

<form class="card" aria-labelledby="settings-heading" onsubmit={save}>
  <h2 id="settings-heading">{t('leader.settings.heading')}</h2>
  <label class="switch-row">
    <span
      ><strong>{t('leader.settings.open')}</strong><small>{t('leader.settings.openHint')}</small
      ></span
    >
    <input type="checkbox" role="switch" bind:checked={open} />
  </label>
  <label class="switch-row">
    <span>
      <strong>{t('leader.settings.autoJoin')}</strong><small
        >{t('leader.settings.autoJoinHint')}</small
      >
    </span>
    <input type="checkbox" role="switch" bind:checked={autoJoin} />
  </label>
  <fieldset class="ticket-name">
    <legend>{t('leader.ticket.heading')}</legend>
    <p class="hint">{t('leader.ticket.hint')}</p>
    {#each ['default', 'prefix', 'override'] as const as mode (mode)}
      <label class="choice">
        <input type="radio" name="ticket-name-mode" value={mode} bind:group={nameMode} />
        {t(`leader.ticket.${mode}`)}
      </label>
    {/each}
    {#if nameMode !== 'default'}
      <label class="text-field">
        <span
          >{t(
            nameMode === 'prefix' ? 'leader.ticket.prefixLabel' : 'leader.ticket.overrideLabel',
          )}</span
        >
        <input
          type="text"
          bind:value={nameText}
          maxlength={TICKET_NAME_MAX[nameMode]}
          autocomplete="off"
          spellcheck="false"
          aria-invalid={nameProblem !== null}
          aria-describedby="ticket-name-help"
        />
      </label>
      <p id="ticket-name-help" class="hint" class:failed={nameProblem !== null}>
        {nameProblem === null
          ? t('leader.ticket.valueHint')
          : t(`leader.ticket.problem.${nameProblem}`)}
      </p>
    {/if}
    <p class="example">
      {t('leader.ticket.example')} <code>{example}</code>
    </p>
  </fieldset>
  <div class="actions">
    <button class="primary" type="submit" disabled={saving || nameProblem !== null}>
      {t(saving ? 'leader.settings.saving' : 'leader.settings.save')}
    </button>
    {#if saved}<p class="success" role="status">{t('leader.settings.saved')}</p>{/if}
    {#if saveFailed}<p class="failed" role="alert">{t('leader.settings.saveError')}</p>{/if}
  </div>
</form>

<section class="card" aria-labelledby="remove-heading">
  <h2 id="remove-heading">{t('leader.remove.heading')}</h2>
  <p class="hint">{t('leader.remove.hint')}</p>
  {#if confirming}
    <p role="alert">{t('leader.remove.confirm')}</p>
    <div class="actions">
      <button class="danger" type="button" onclick={remove}>{t('leader.remove.yes')}</button>
      <button type="button" onclick={() => (confirming = false)}>{t('leader.remove.cancel')}</button
      >
    </div>
  {:else}
    <button class="danger" type="button" onclick={() => (confirming = true)}>
      {t('leader.remove.button')}
    </button>
  {/if}
  {#if removeFailed}<p class="failed" role="alert">{t('leader.remove.error')}</p>{/if}
</section>

<style>
  .card {
    padding: 20px;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  h2 {
    margin: 0 0 12px;
    font-size: 1.05rem;
  }

  small,
  .hint {
    display: block;
    color: var(--muted);
    font-size: 0.85rem;
  }

  .hint {
    margin: 0 0 12px;
  }

  .checks {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .checks li {
    display: flex;
    gap: 10px;
    align-items: start;
  }

  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: var(--on-accent);
    font-size: 0.75rem;
    font-weight: 800;
  }

  .ok .icon {
    background: var(--open);
  }

  .warn .icon {
    background: var(--approval);
  }

  .bad .icon {
    background: var(--closed);
  }

  .switch-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
  }

  input[role='switch'] {
    position: relative;
    flex: none;
    width: 40px;
    height: 22px;
    margin: 0;
    background: var(--surface);
    border: 1px solid var(--control-border);
    border-radius: 999px;
    appearance: none;
    cursor: pointer;
  }

  input[role='switch']::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    background: var(--muted);
    border-radius: 50%;
    transition: left 0.15s;
  }

  input[role='switch']:checked {
    background: var(--open);
    border-color: var(--open);
  }

  input[role='switch']:checked::after {
    left: 20px;
    background: #ffffff;
  }

  .ticket-name {
    margin: 12px 0 0;
    padding: 0;
    border: 0;
  }

  .ticket-name legend {
    padding: 0;
    font-weight: 700;
  }

  .ticket-name .hint {
    margin: 4px 0 8px;
  }

  .choice {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 0;
    cursor: pointer;
  }

  .text-field {
    display: grid;
    gap: 4px;
    margin-top: 8px;
    font-size: 0.875rem;
  }

  .example {
    margin: 8px 0 0;
    overflow-wrap: anywhere;
    font-size: 0.875rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    margin-top: 16px;
  }

  .actions p {
    margin: 0;
  }

  .success {
    color: var(--open);
  }

  .failed {
    color: var(--closed);
  }

  .danger {
    border-color: var(--closed);
    color: var(--closed);
  }
</style>
