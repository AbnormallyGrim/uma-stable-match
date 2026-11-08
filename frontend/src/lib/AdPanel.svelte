<!--
  @component
  A trainer's own ad: the form to post it, or the ad with its numbers and how long it stays, with
  Renew, Edit and Delete, and the invitations leaders sent, each to apply to or dismiss.
-->

<script lang="ts">
  import {
    adDay,
    DEFAULT_CHOICES,
    deleteAd,
    dismissInvite,
    rankRange,
    renewAd,
    saveAd,
    type AdChoices,
    type MyAd,
  } from './ads';
  import AdForm from './AdForm.svelte';
  import { ApiError } from './api';
  import { applyUrl } from './applications';
  import { formatMillions, formatPerDay } from './format';
  import { t } from './i18n';
  import { clubPath, SETTINGS_PATH } from './route';

  interface Props {
    /** The trainer's ad, or `null` without one. Changes as they post, renew and delete it. */
    ad: MyAd | null;
    /** Called after the ad or its invitations changed. */
    onchange?: () => void;
  }

  let { ad = $bindable(), onchange }: Props = $props();

  let editing = $state(false);
  let busy = $state(false);
  /** Why the last form or button action failed. */
  let failure = $state<string | null>(null);
  let noTrainer = $state(false);

  const showForm = $derived(ad === null || editing);

  function why(reason: unknown, fallback: string): string {
    noTrainer = false;
    if (reason instanceof ApiError && reason.status === 403) {
      noTrainer = true;
      return t('ad.form.noTrainer');
    }
    if (reason instanceof ApiError && reason.status === 404) {
      return t('ad.form.notFound');
    }
    return fallback;
  }

  /** Runs an action on the ad, showing why it failed. */
  async function run(action: () => Promise<void>, fallback: string) {
    busy = true;
    failure = null;
    noTrainer = false;
    try {
      await action();
      onchange?.();
    } catch (reason) {
      failure = why(reason, fallback);
    } finally {
      busy = false;
    }
  }

  function post(choices: AdChoices) {
    return run(async () => {
      ad = await saveAd(choices);
      editing = false;
    }, t('ad.form.saveError'));
  }

  function renew() {
    return run(async () => {
      ad = await renewAd();
    }, t('ad.actionError'));
  }

  function remove() {
    return run(async () => {
      await deleteAd();
      ad = null;
      editing = false;
    }, t('ad.actionError'));
  }

  function dismiss(clubId: string) {
    return run(async () => {
      await dismissInvite(clubId);
      if (ad !== null) {
        ad = { ...ad, invites: ad.invites.filter((invite) => invite.clubId !== clubId) };
      }
    }, t('ad.actionError'));
  }
</script>

<div class="panel">
  <p class="lede">{t('ad.lede')}</p>

  {#if showForm}
    <section>
      <AdForm
        initial={ad ?? DEFAULT_CHOICES}
        submitLabel={ad === null ? t('ad.form.post') : t('ad.form.save')}
        {busy}
        error={failure}
        onsubmit={post}
        oncancel={ad === null ? undefined : () => ((editing = false), (failure = null))}
      />
      {#if noTrainer}
        <p><a href={SETTINGS_PATH}>{t('ad.noTrainerLink')}</a></p>
      {/if}
    </section>
  {:else if ad !== null}
    <section aria-labelledby="my-ad-heading">
      <h3 id="my-ad-heading">{ad.trainerName}</h3>
      <p class="status" class:off={!ad.onBoard} role="status">
        {ad.onBoard
          ? t('ad.status.onBoard', { date: adDay(ad.visibleUntil) })
          : t('ad.status.off', { date: adDay(ad.visibleUntil) })}
        <span class="muted">{t('ad.status.kept', { date: adDay(ad.expiresAt) })}</span>
      </p>

      <div class="columns">
        <div>
          <h4>{t('ad.numbers')}</h4>
          <dl>
            <dt>{t('ad.numbers.club')}</dt>
            <dd>{ad.currentClub ?? t('ad.numbers.noClub')}</dd>
            <dt>{t('ad.numbers.pace')}</dt>
            <dd>{ad.pace === null ? t('ad.numbers.unknown') : formatPerDay(ad.pace)}</dd>
            <dt>{t('ad.numbers.fans')}</dt>
            <dd>
              {ad.fansThisMonth === null
                ? t('ad.numbers.unknown')
                : formatMillions(ad.fansThisMonth)}
            </dd>
          </dl>
          <p class="muted">{t('ad.numbers.hint')}</p>
        </div>
        <div>
          <h4>{t('ad.wants')}</h4>
          <dl>
            <dt>{t('ad.wants.ranks')}</dt>
            <dd>{rankRange(ad.rankMin, ad.rankMax)}</dd>
            <dt>{t('ad.wants.discord')}</dt>
            <dd>{t(`ad.discord.${ad.discord}`)}</dd>
            <dt>{t('ad.wants.language')}</dt>
            <dd>{t(`ad.language.${ad.language}`)}</dd>
          </dl>
        </div>
      </div>

      <div class="actions">
        <button type="button" class="primary" onclick={renew} disabled={busy}>
          {t('ad.renew')}
        </button>
        <button type="button" onclick={() => ((editing = true), (failure = null))} disabled={busy}>
          {t('ad.edit')}
        </button>
        <button type="button" class="danger" onclick={remove} disabled={busy}>
          {t('ad.delete')}
        </button>
      </div>
      {#if failure}
        <p class="problem" role="alert">{failure}</p>
      {/if}
    </section>

    <section aria-labelledby="invites-heading">
      <h3 id="invites-heading">{t('ad.invites.heading')}</h3>
      {#if ad.invites.length === 0}
        <p class="muted">{t('ad.invites.empty')}</p>
      {:else}
        <p class="muted">{t('ad.invites.hint')}</p>
        <ul class="invites">
          {#each ad.invites as invite (invite.clubId)}
            {@const club = invite.clubName ?? t('ad.invites.club', { id: invite.clubId })}
            <li>
              <span class="club">
                <a href={clubPath(invite.clubId)}>{club}</a>
                <span class="muted">{adDay(invite.sentAt)}</span>
              </span>
              <span class="act">
                <a
                  class="button-link"
                  href={applyUrl(invite.clubId)}
                  aria-label={t('ad.invites.applyTo', { club })}>{t('ad.invites.apply')}</a
                >
                <button
                  type="button"
                  onclick={() => dismiss(invite.clubId)}
                  disabled={busy}
                  aria-label={t('ad.invites.dismissFrom', { club })}
                >
                  {t('ad.invites.dismiss')}
                </button>
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

<style>
  .panel {
    display: grid;
    gap: 20px;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 1.05rem;
  }

  h4 {
    margin: 0 0 6px;
    font-size: 0.9rem;
  }

  .lede,
  .muted {
    margin: 0;
    color: var(--text-2);
  }

  .status {
    margin: 0 0 14px;
    font-weight: 600;
  }

  .status .muted {
    display: block;
    font-weight: 400;
    font-size: 0.85rem;
  }

  .status.off {
    color: var(--approval);
  }

  .columns {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 20px;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
    margin: 0 0 8px;
  }

  dt {
    color: var(--muted);
    font-size: 0.85rem;
  }

  dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 16px;
  }

  .danger {
    color: var(--closed);
  }

  .problem {
    margin: 10px 0 0;
    color: var(--closed);
    font-size: 0.85rem;
  }

  .invites {
    display: grid;
    margin: 12px 0 0;
    padding: 0;
    list-style: none;
  }

  .invites li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .invites li:last-child {
    border-bottom: 0;
  }

  .club {
    display: grid;
  }

  .club a {
    font-weight: 700;
  }

  .act {
    display: flex;
    gap: 8px;
  }

  /* A link that looks like a primary button. */
  .button-link {
    display: inline-flex;
    align-items: center;
    padding: 6px 14px;
    background: var(--accent);
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    color: var(--on-accent);
    font-size: 0.875rem;
    font-weight: 600;
    text-decoration: none;
    white-space: nowrap;
  }
</style>
