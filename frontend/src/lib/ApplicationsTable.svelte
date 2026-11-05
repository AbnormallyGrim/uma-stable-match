<!--
  @component
  The applications a club has had, newest first, filtered by status. Each row opens the
  application as the bot posted it. On narrow screens each row shows as a card.
-->

<script lang="ts">
  import ApplicationDetails from './ApplicationDetails.svelte';
  import type { Application, ApplicationStatus } from './applications';
  import { formatDate, formatMillions, formatPace } from './format';
  import { t } from './i18n';

  interface Props {
    applications: Application[];
    /** The club's name, or `null` while it's unknown. */
    clubName: string | null;
  }

  let { applications, clubName }: Props = $props();

  type Filter = 'all' | ApplicationStatus;
  const FILTERS: Filter[] = ['all', 'pending', 'accepted', 'declined'];

  let filter = $state<Filter>('all');
  let showing = $state<Application | null>(null);

  const newest = $derived(
    [...applications].sort((a, b) => b.submittedAt.localeCompare(a.submittedAt)),
  );
  const shown = $derived(
    filter === 'all' ? newest : newest.filter((application) => application.status === filter),
  );

  function count(which: Filter): number {
    return which === 'all'
      ? applications.length
      : applications.filter((application) => application.status === which).length;
  }
</script>

<section class="card" aria-labelledby="applications-heading">
  <h2 id="applications-heading">{t('leader.table.heading')}</h2>
  {#if applications.length === 0}
    <div class="empty">
      <strong>{t('leader.table.empty')}</strong>
      <span>{t('leader.table.emptyHint')}</span>
    </div>
  {:else}
    <div class="filters" role="group" aria-label={t('leader.filter.label')}>
      {#each FILTERS as which (which)}
        <button type="button" aria-pressed={filter === which} onclick={() => (filter = which)}>
          {t(`leader.filter.${which}`)} ({count(which)})
        </button>
      {/each}
    </div>
    <table>
      <thead>
        <tr>
          <th scope="col">{t('leader.table.trainer')}</th>
          <th scope="col">{t('leader.table.submitted')}</th>
          <th scope="col">{t('leader.table.pace')}</th>
          <th scope="col">{t('leader.table.fans')}</th>
          <th scope="col">{t('leader.table.currentClub')}</th>
          <th scope="col">{t('leader.table.status')}</th>
          <th scope="col"><span class="visually-hidden">{t('leader.table.actions')}</span></th>
        </tr>
      </thead>
      <tbody>
        {#each shown as application (application.id)}
          <tr>
            <td class="who" data-label={t('leader.table.trainer')}>
              <strong>{application.trainerName}</strong><small>{application.trainerId}</small>
            </td>
            <td data-label={t('leader.table.submitted')}>
              {formatDate(application.submittedAt.slice(0, 10))}
            </td>
            <td class="num" data-label={t('leader.table.pace')}>{formatPace(application.pace)}</td>
            <td class="num" data-label={t('leader.table.fans')}>
              {formatMillions(application.fansThisMonth)}
            </td>
            <td data-label={t('leader.table.currentClub')}>
              {application.currentClub ?? t('leader.table.noClub')}
            </td>
            <td data-label={t('leader.table.status')}>
              <span class="tag {application.status}">
                {t(`leader.applicationStatus.${application.status}`)}
              </span>
            </td>
            <td class="act">
              <button
                type="button"
                aria-label={t('leader.table.viewFor', { name: application.trainerName })}
                onclick={() => (showing = application)}>{t('leader.table.view')}</button
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

{#if showing}
  <ApplicationDetails application={showing} {clubName} onclose={() => (showing = null)} />
{/if}

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

  .empty {
    display: grid;
    gap: 4px;
    padding: 32px 16px;
    color: var(--muted);
    text-align: center;
  }

  .empty strong {
    color: var(--text);
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 12px;
  }

  .filters button {
    padding: 4px 12px;
    border-radius: 999px;
    font-size: 0.8rem;
  }

  .filters button[aria-pressed='true'] {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 700;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.75rem;
    letter-spacing: 0.05em;
    text-align: left;
    text-transform: uppercase;
  }

  td {
    padding: 10px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  .num {
    font-variant-numeric: tabular-nums;
  }

  .who strong,
  .who small {
    display: block;
  }

  .who small {
    color: var(--muted);
  }

  .tag {
    padding: 2px 8px;
    background: var(--surface);
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .pending {
    color: var(--approval);
  }

  .accepted {
    color: var(--open);
  }

  .declined {
    color: var(--closed);
  }

  .act {
    text-align: right;
  }

  /* Each row becomes a card, with the column names beside the values. */
  @media (width < 720px) {
    table,
    tbody,
    tr,
    td {
      display: block;
    }

    thead {
      display: none;
    }

    tr {
      padding: 12px 0;
      border-bottom: 1px solid var(--border);
    }

    tbody tr:last-child {
      border-bottom: 0;
    }

    td {
      display: flex;
      justify-content: space-between;
      gap: 12px;
      padding: 3px 0;
      border: 0;
    }

    td::before {
      content: attr(data-label);
      color: var(--muted);
      font-size: 0.85rem;
    }

    .who {
      display: block;
    }

    .who::before {
      content: none;
    }

    .act {
      padding-top: 8px;
    }

    .act button {
      width: 100%;
    }
  }
</style>
