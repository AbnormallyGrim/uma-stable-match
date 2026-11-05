// TrainerApplicationsTable component tests. All names and IDs are made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { TrainerApplication } from './applications';
import { t } from './i18n';
import TrainerApplicationsTable from './TrainerApplicationsTable.svelte';

const application = (
  clubName: string,
  status: TrainerApplication['status'],
  submittedAt: string,
  ticketUrl: string | null = 'https://discord.com/channels/1/2',
): TrainerApplication => ({
  id: clubName,
  clubId: '100000001',
  clubName,
  trainerId: '100000000011',
  status,
  submittedAt,
  ticketUrl,
});

const now = new Date('2026-11-05T12:00:00Z');

const applications: TrainerApplication[] = [
  application('Alpha Club', 'accepted', '2026-10-30T10:00:00Z'),
  application('Bravo Club', 'pending', '2026-11-02T10:00:00Z', null),
  application('Charlie Club', 'declined', '2026-10-31T10:00:00Z'),
  application('Expired Club', 'pending', '2026-10-04T10:00:00Z'),
];

const clubs = () =>
  screen
    .getAllByRole('row')
    .slice(1)
    .map((row) => within(row).getByRole('link', { name: /^\w+ Club$/ }).textContent);

describe('TrainerApplicationsTable', () => {
  it('says when there are no active applications', () => {
    render(TrainerApplicationsTable, {
      applications: [application('Expired Club', 'pending', '2026-10-04T10:00:00Z')],
      now,
    });
    expect(screen.getByText(t('trainer.table.empty'))).toBeInTheDocument();
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('lists active applications newest first and filters by status', async () => {
    render(TrainerApplicationsTable, { applications, now });
    expect(clubs()).toEqual(['Bravo Club', 'Charlie Club', 'Alpha Club']);
    expect(screen.getByRole('button', { name: `${t('leader.filter.all')} (3)` })).toBeVisible();
    await fireEvent.click(
      screen.getByRole('button', { name: `${t('leader.filter.pending')} (1)` }),
    );
    expect(clubs()).toEqual(['Bravo Club']);
  });

  it('links to the club and its ticket, and says when the ticket is gone', () => {
    render(TrainerApplicationsTable, { applications, now });
    expect(screen.getByRole('link', { name: 'Alpha Club' })).toHaveAttribute(
      'href',
      '/club/100000001',
    );
    expect(
      screen.getByRole('link', { name: t('trainer.table.ticketFor', { name: 'Alpha Club' }) }),
    ).toHaveAttribute('href', 'https://discord.com/channels/1/2');
    const bravo = screen.getByRole('link', { name: 'Bravo Club' }).closest('tr')!;
    expect(within(bravo).getByText(t('trainer.table.noTicket'))).toBeInTheDocument();
  });
});
