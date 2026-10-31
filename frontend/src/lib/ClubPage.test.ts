// ClubPage component tests with a stubbed loader. All data is made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import { ApiError, type ClubDetails, type fetchClub } from './api';
import ClubPage from './ClubPage.svelte';
import { club } from './test-data';

const M = 1_000_000;

function details(overrides: Partial<ClubDetails> = {}): ClubDetails {
  return {
    club: club({ name: 'Page Club', comment: 'Made-up description' }),
    month: '2026-03',
    updated_at: '2026-03-19T15:00:00Z',
    median_per_day: 6.5 * M,
    members: [9 * M, 7 * M, 6 * M, 4 * M, null].map((pace) => ({
      pace_per_day: pace,
      daily_fans: [],
    })),
    trainer: null,
    ...overrides,
  };
}

const trainer = {
  trainer_id: 123456789012,
  pace_per_day: 6.5 * M,
  daily_fans: null,
  in_club: false,
  your_place: 3,
  club_size: 6,
};

function setup(load: typeof fetchClub, trainerId: string | null = null) {
  return render(ClubPage, { route: { circleId: '1', trainerId }, load });
}

const resolving = (value: ClubDetails) => vi.fn<typeof fetchClub>(() => Promise.resolve(value));

function rows() {
  const table = screen.getByRole('table');
  return within(table)
    .getAllByRole('row')
    .slice(1)
    .map((row) =>
      within(row)
        .getAllByRole('cell')
        .map((cell) => cell.textContent?.replace(/\s+/g, ' ').trim()),
    );
}

describe('ClubPage', () => {
  it('shows the club and its members, fastest first', async () => {
    const load = resolving(details());
    setup(load);
    expect(screen.getByRole('status')).toHaveTextContent('Loading the club…');

    const link = await screen.findByRole('link', { name: 'Page Club' });
    expect(link).toHaveAttribute('href', 'https://uma.moe/circles/1');
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Page Club');
    expect(screen.getByText('Made-up description')).toBeInTheDocument();
    expect(screen.getByText('6.5M/day')).toBeInTheDocument();
    expect(
      screen.getByText(/5 current members, fastest first\. Data for March 2026/),
    ).toBeVisible();
    expect(rows()).toEqual([
      ['1', '9.0M/day'],
      ['2', '7.0M/day'],
      ['3', '6.0M/day'],
      ['4', '4.0M/day'],
      ['–', 'Too new to rank'],
    ]);
    expect(load).toHaveBeenCalledWith('1', null, expect.anything());
    expect(document.title).toBe('Page Club · Uma Stable Match');
  });

  it('adds the trainer as a highlighted row where they would rank', async () => {
    const load = resolving(details({ trainer }));
    setup(load, '123456789012');

    await screen.findByRole('table');
    expect(load).toHaveBeenCalledWith('1', '123456789012', expect.anything());
    expect(rows()).toEqual([
      ['1', '9.0M/day', '+2.5M/day'],
      ['2', '7.0M/day', '+500K/day'],
      ['3 You', '6.5M/day', ''],
      ['4', '6.0M/day', '−500K/day'],
      ['5', '4.0M/day', '−2.5M/day'],
      ['–', 'Too new to rank', ''],
    ]);
    expect(screen.getAllByRole('row')[3]).toHaveClass('you');
    expect(screen.getByText('3 of 6')).toBeInTheDocument();
    expect(screen.queryByText(/This is your club/)).not.toBeInTheDocument();
  });

  it('says when it is the trainer’s own club', async () => {
    setup(resolving(details({ trainer: { ...trainer, in_club: true } })), '123456789012');
    expect(await screen.findByText(/This is your club/)).toBeInTheDocument();
  });

  it('explains a club that is not loaded and retries', async () => {
    const load = vi
      .fn<typeof fetchClub>()
      .mockRejectedValueOnce(new ApiError('http', 'club not loaded yet; try again later', 404))
      .mockResolvedValueOnce(details());
    setup(load);

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent("This club isn't loaded yet");
    await fireEvent.click(within(alert).getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('table')).toBeInTheDocument();
    expect(load).toHaveBeenCalledTimes(2);
  });

  it('offers no retry for an unknown trainer', async () => {
    setup(
      vi.fn<typeof fetchClub>(() => Promise.reject(new ApiError('http', 'trainer not found', 404))),
      '123456789012',
    );
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent("We couldn't find that trainer");
    expect(within(alert).queryByRole('button')).not.toBeInTheDocument();
  });

  it('links back to the search', () => {
    setup(resolving(details()));
    expect(screen.getByRole('link', { name: '← Back to search' })).toHaveAttribute('href', '/');
  });
});
