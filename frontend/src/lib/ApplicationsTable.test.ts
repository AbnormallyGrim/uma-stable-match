// ApplicationsTable component tests. All names and IDs are made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import type { Application } from './applications';
import ApplicationsTable from './ApplicationsTable.svelte';
import { t } from './i18n';

const application = (id: string, status: Application['status'], submittedAt: string) => ({
  id,
  trainerName: `Trainer ${id}`,
  trainerId: '100000000011',
  discordName: `applicant_${id}`,
  pace: 1_420_000,
  fansThisMonth: 18_300_000,
  currentClub: null,
  status,
  submittedAt,
  ticketUrl: 'https://discord.com/channels/1/2',
});

const applications: Application[] = [
  application('Alpha', 'accepted', '2026-10-30T10:00:00Z'),
  application('Bravo', 'pending', '2026-11-02T10:00:00Z'),
  application('Charlie', 'declined', '2026-10-31T10:00:00Z'),
];

const names = () =>
  screen
    .getAllByRole('row')
    .slice(1)
    .map((row) => within(row).getByText(/^Trainer /).textContent);

describe('ApplicationsTable', () => {
  it('says when there are no applications yet', () => {
    render(ApplicationsTable, { applications: [], clubName: 'Example Club', clubId: '7' });
    expect(screen.getByText(t('leader.table.empty'))).toBeInTheDocument();
    expect(screen.queryByRole('table')).toBeNull();
  });

  it('lists the newest first and filters by status', async () => {
    render(ApplicationsTable, { applications, clubName: 'Example Club', clubId: '7' });
    expect(names()).toEqual(['Trainer Bravo', 'Trainer Charlie', 'Trainer Alpha']);
    await fireEvent.click(
      screen.getByRole('button', { name: `${t('leader.filter.pending')} (1)` }),
    );
    expect(names()).toEqual(['Trainer Bravo']);
  });

  it('links an application to its comparison in the club', async () => {
    render(ApplicationsTable, { applications, clubName: 'Example Club', clubId: '7' });
    await fireEvent.click(
      screen.getByRole('button', { name: t('leader.table.viewFor', { name: 'Trainer Alpha' }) }),
    );
    expect(
      within(screen.getByRole('dialog')).getByRole('link', { name: t('leader.details.compare') }),
    ).toHaveAttribute('href', '/applications/club/7/compare?trainers=100000000011');
  });

  it('opens an application as the bot posted it, and closes it', async () => {
    render(ApplicationsTable, { applications, clubName: 'Example Club', clubId: '7' });
    await fireEvent.click(
      screen.getByRole('button', { name: t('leader.table.viewFor', { name: 'Trainer Alpha' }) }),
    );
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText('@applicant_Alpha')).toBeInTheDocument();
    expect(
      within(dialog).getByText(t('leader.details.embedTitle', { name: 'Example Club' })),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByRole('link', { name: t('leader.details.openTicket') }),
    ).toHaveAttribute('href', 'https://discord.com/channels/1/2');
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
