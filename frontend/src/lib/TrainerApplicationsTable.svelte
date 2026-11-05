<!--
  @component
  The applications a trainer has sent that haven't expired, newest first, filtered by status. On
  narrow screens each row shows as a card.
-->

<script lang="ts">
  import {
    activeApplications,
    expiresAt,
    type ApplicationStatus,
    type TrainerApplication,
  } from './applications';
  import { formatDate } from './format';
  import { t } from './i18n';
  import { clubPath } from './route';

  interface Props {
    applications: TrainerApplication[];
    /** The time to judge expiry by. */
    now?: Date;
  }

  let { applications, now = new Date() }: Props = $props();

  type Filter = 'all' | ApplicationStatus;
  const FILTERS: Filter[] = ['all', 'pending', 'accepted', 'declined'];

  let filter = $state<Filter>('all');

  const active = $derived(
    activeApplications(applications, now).sort((a, b) =>
      b.submittedAt.localeCompare(a.submittedAt),
    ),
  );
  const shown = $derived(
    filter === 'all' ? active : active.filter((application) => application.status === filter),
  );

  function count(which: Filter): number {
    return which === 'all'
      ? active.length
      : active.filter((application) => application.status === which).length;
  }

  function day(date: Date): string {
    return formatDate(date.toISOString().slice(0, 10));
  }
</script>

<section class="card" aria-labelledby="trainer-applications-heading">
  <h2 id="trainer-applications-heading">{t('trainer.table.heading')}</h2>
  {#if active.length === 0}
    <div class="empty">
      <strong>{t('trainer.table.empty')}</strong>
      <span>{t('trainer.table.emptyHint')}</span>
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
          <th scope="col">{t('trainer.table.club')}</th>
          <th scope="col">{t('trainer.table.trainerId')}</th>
          <th scope="col">{t('leader.table.submitted')}</th>
          <th scope="col">{t('trainer.table.expires')}</th>
          <th scope="col">{t('leader.table.status')}</th>
          <th scope="col"><span class="visually-hidden">{t('leader.table.actions')}</span></th>
        </tr>
      </thead>
      <tbody>
        {#each shown as application (application.id)}
          <tr>
            <td class="club" data-label={t('trainer.table.club')}>
              <a href={clubPath(application.clubId)}>{application.clubName}</a>
            </td>
            <td class="num" data-label={t('trainer.table.trainerId')}>{application.trainerId}</td>
            <td data-label={t('leader.table.submitted')}>
              {day(new Date(application.submittedAt))}
            </td>
            <td data-label={t('trainer.table.expires')}
              >{day(expiresAt(application.submittedAt))}</td
            >
            <td data-label={t('leader.table.status')}>
              <span class="tag {application.status}">
                {t(`leader.applicationStatus.${application.status}`)}
              </span>
            </td>
            <td class="act">
              {#if application.ticketUrl !== null}
                <a
                  class="ticket"
                  href={application.ticketUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  aria-label={t('trainer.table.ticketFor', { name: application.clubName })}
                  >{t('trainer.table.ticket')}</a
                >
              {:else}
                <span class="muted">{t('trainer.table.noTicket')}</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
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

  .club a {
    font-weight: 700;
  }

  .muted {
    color: var(--muted);
    font-size: 0.85rem;
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

  .ticket {
    display: inline-block;
    padding: 4px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
    font-size: 0.85rem;
    font-weight: 600;
    text-decoration: none;
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

    .club {
      display: block;
    }

    .club::before {
      content: none;
    }

    .act {
      padding-top: 8px;
    }

    .act::before {
      content: none;
    }

    .ticket {
      width: 100%;
      text-align: center;
    }
  }
</style>
