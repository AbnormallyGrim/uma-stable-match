// ClubComparePage tests with a stubbed loader and fetch. All data is made up.

import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { ClubCompare, fetchClubCompare } from './api';
import ClubComparePage from './ClubComparePage.svelte';
import { t } from './i18n';
import { club } from './test-data';

const M = 1_000_000;
const ALPHA = '100000000011';
const BRAVO = '100000000022';
const CHARLIE = '100000000033';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const application = (id: string, name: string, submittedAt = new Date().toISOString()) => ({
  id: name,
  trainerName: name,
  trainerId: id,
  discordName: `${name}_discord`,
  pace: M,
  fansThisMonth: M,
  currentClub: null,
  status: 'pending',
  submittedAt,
  ticketUrl: null,
});

function answer(mine: unknown, applications: unknown[] = []) {
  vi.stubGlobal(
    'fetch',
    vi.fn((input: string) => {
      const path = input.split('?')[0];
      if (path === '/api/v1/discord/my-club') return Promise.resolve(mine as Response);
      if (path === '/api/v1/discord/clubs/7/applications') {
        return Promise.resolve(json(applications));
      }
      return Promise.resolve(json({ error: 'not found' }, 404));
    }),
  );
}

const leader = () =>
  json({ trainer: '123456789', club: { id: '7', name: 'Page Club', leader: true } });

const fans = (pace: number) => Array.from({ length: 5 }, (_, d) => 100 * M + pace * d);

function compare(ids: string[]): ClubCompare {
  return {
    club: club({ name: 'Page Club' }),
    month: '2026-03',
    updated_at: '2026-03-19T15:00:00Z',
    median_per_day: 6.5 * M,
    members: [9 * M, 7 * M, 4 * M].map((pace, i) => ({
      name: `Made-up Member ${'ABC'[i]}`,
      pace_per_day: pace,
      daily_fans: fans(pace),
    })),
    trainers: ids.map((id, i) => ({
      trainer_id: Number(id),
      pace_per_day: (5 + i) * M,
      daily_fans: fans((5 + i) * M),
      in_club: false,
      your_place: 3 - i,
      club_size: 4,
    })),
    previous: null,
  };
}

const loader = () => vi.fn<typeof fetchClubCompare>((_club, ids) => Promise.resolve(compare(ids)));

afterEach(() => {
  vi.unstubAllGlobals();
  history.replaceState(null, '', '/');
});

describe('ClubComparePage', () => {
  it('places the applicant from the address among the members', async () => {
    answer(leader(), [application(ALPHA, 'Applicant Alpha')]);
    const load = loader();
    render(ClubComparePage, { clubId: '7', trainerIds: [ALPHA], load });

    expect(await screen.findByRole('heading', { name: 'Page Club' })).toBeInTheDocument();
    expect(load).toHaveBeenCalledWith('7', [ALPHA], expect.anything());
    const card = (await screen.findByText('Applicant Alpha', { selector: '.name' })).closest('li')!;
    expect(within(card).getByText('5.0M/day')).toBeInTheDocument();
    expect(within(card).getByText('3 of 4')).toBeInTheDocument();
    expect(document.querySelectorAll('.roster li')).toHaveLength(3);
  });

  it('is for the leader of the club only', async () => {
    answer(json({ trainer: '123456789', club: { id: '8', name: 'Other', leader: true } }));
    const load = loader();
    render(ClubComparePage, { clubId: '7', trainerIds: [ALPHA], load });

    expect(await screen.findByText(t('leader.forbidden'), { exact: false })).toBeInTheDocument();
    expect(load).not.toHaveBeenCalled();
  });

  it('asks a signed-out visitor to sign in', async () => {
    answer(json({ error: 'sign in' }, 401));
    render(ClubComparePage, { clubId: '7', trainerIds: [ALPHA], load: loader() });
    expect(await screen.findByText(t('leader.signIn'))).toBeInTheDocument();
  });

  it('adds an applicant from the list, and keeps the address in step', async () => {
    answer(leader(), [
      application(ALPHA, 'Applicant Alpha'),
      application(BRAVO, 'Applicant Bravo'),
    ]);
    const load = loader();
    render(ClubComparePage, { clubId: '7', trainerIds: [ALPHA], load });

    await screen.findByRole('heading', { name: 'Page Club' });
    const select = await screen.findByLabelText(t('compare.add.applicant'));
    await waitFor(() => expect(within(select).getAllByRole('option')).toHaveLength(2));
    // Alpha is already shown, so only Bravo is offered.
    await fireEvent.change(select, { target: { value: BRAVO } });
    await fireEvent.click(screen.getAllByRole('button', { name: t('compare.add.button') })[0]!);

    await waitFor(() =>
      expect(load).toHaveBeenLastCalledWith('7', [ALPHA, BRAVO], expect.anything()),
    );
    expect(window.location.pathname + window.location.search).toBe(
      `/applications/club/7/compare?trainers=${ALPHA}%2C${BRAVO}`,
    );
    expect(await screen.findByText('Applicant Bravo', { selector: '.name' })).toBeInTheDocument();
  });

  it('adds a trainer by ID, and rejects one that is not an ID', async () => {
    answer(leader());
    const load = loader();
    render(ClubComparePage, { clubId: '7', trainerIds: [ALPHA], load });

    await screen.findByRole('heading', { name: 'Page Club' });
    const input = screen.getByLabelText(t('compare.add.byId'));
    await fireEvent.input(input, { target: { value: 'me' } });
    await fireEvent.submit(input.closest('form')!);
    expect(await screen.findByText(t('compare.add.invalid'))).toBeInTheDocument();

    await fireEvent.input(input, { target: { value: '1000 0000 0033' } });
    await fireEvent.submit(input.closest('form')!);
    await waitFor(() =>
      expect(load).toHaveBeenLastCalledWith('7', [ALPHA, CHARLIE], expect.anything()),
    );
  });

  it('stops at five trainers, and removes one on request', async () => {
    const five = Array.from({ length: 5 }, (_, i) => `10000000001${i}`);
    answer(leader());
    const load = loader();
    render(ClubComparePage, { clubId: '7', trainerIds: five, load });

    await screen.findByRole('heading', { name: 'Page Club' });
    expect(screen.getByText(t('compare.add.full', { max: 5 }))).toBeInTheDocument();
    expect(screen.queryByLabelText(t('compare.add.byId'))).toBeNull();

    await fireEvent.click(
      screen.getByRole('button', {
        name: t('compare.remove', { name: t('compare.unnamed', { id: five[0]! }) }),
      }),
    );
    await waitFor(() =>
      expect(load).toHaveBeenLastCalledWith('7', five.slice(1), expect.anything()),
    );
    expect(screen.queryByText(t('compare.add.full', { max: 5 }))).toBeNull();
  });

  it('keeps the last trainer', async () => {
    answer(leader());
    render(ClubComparePage, { clubId: '7', trainerIds: [ALPHA], load: loader() });
    await screen.findByRole('heading', { name: 'Page Club' });
    expect(
      screen.getByRole('button', {
        name: t('compare.remove', { name: t('compare.unnamed', { id: ALPHA }) }),
      }),
    ).toBeDisabled();
  });

  it('asks for an applicant when the address has none, without loading', async () => {
    answer(leader());
    const load = loader();
    render(ClubComparePage, { clubId: '7', trainerIds: [], load });
    expect(await screen.findByText(t('compare.empty'))).toBeInTheDocument();
    expect(load).not.toHaveBeenCalled();
  });
});
