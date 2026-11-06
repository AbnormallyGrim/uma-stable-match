// ClubApplicationsPage tests with a stubbed fetch. All data is made up.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { Settings } from './applications';
import ClubApplicationsPage from './ClubApplicationsPage.svelte';
import { t } from './i18n';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const settings = (change: Partial<Settings> = {}): Settings => ({
  guild_id: '5000',
  category: '10',
  auto_join: true,
  open: true,
  problem: null,
  categories: [{ id: '10', name: 'Applications' }],
  ...change,
});

const TRAINER = '123456789';

/** The leader of club 7, unless `routes` says otherwise. */
const leader = () =>
  json({ trainer: TRAINER, club: { id: '7', name: 'Example Club', leader: true } });

/**
 * Answers `METHOD /path` from `routes`, with the signed-in person leading club 7 by default, and
 * 404 for anything else.
 */
function answer(routes: Record<string, (init?: RequestInit) => Response>) {
  const all: Record<string, (init?: RequestInit) => Response> = {
    'GET /api/v1/discord/my-club': leader,
    ...routes,
  };
  const fetch = vi.fn((input: string, init?: RequestInit) => {
    const route = all[`${init?.method ?? 'GET'} ${input.split('?')[0]}`];
    return Promise.resolve(route ? route(init) : json({ error: 'not found' }, 404));
  });
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

afterEach(() => {
  vi.unstubAllGlobals();
  history.replaceState(null, '', '/');
});

describe('ClubApplicationsPage', () => {
  describe('before the bot is added', () => {
    it('asks for a linked account first, without checking the invite', async () => {
      const fetch = answer({
        'GET /api/v1/discord/my-club': () => json({ trainer: null, club: null }),
      });
      render(ClubApplicationsPage);
      expect(await screen.findByText(t('leader.steps.accountHow'))).toBeInTheDocument();
      expect(screen.getByRole('link', { name: t('leader.steps.openSettings') })).toHaveAttribute(
        'href',
        '/settings',
      );
      expect(screen.queryAllByRole('img', { name: t('leader.steps.done') })).toHaveLength(0);
      expect(screen.getByRole('button', { name: t('leader.addBot') })).toBeDisabled();
      expect(fetch.mock.calls.filter(([url]) => url.includes('opt-in'))).toHaveLength(0);
    });

    it('says when the linked trainer leads no club', async () => {
      const fetch = answer({
        'GET /api/v1/discord/my-club': () =>
          json({ trainer: TRAINER, club: { id: '8', name: 'Other Club', leader: false } }),
      });
      render(ClubApplicationsPage, { clubId: '8' });
      expect(await screen.findByText(t('leader.steps.notLeader'))).toBeInTheDocument();
      expect(screen.getByText(t('leader.steps.accountDone', { id: TRAINER }))).toBeInTheDocument();
      expect(screen.getByRole('button', { name: t('leader.addBot') })).toBeDisabled();
      expect(fetch.mock.calls.filter(([url]) => url.includes('opt-in'))).toHaveLength(0);
    });

    it('opens the club the leader leads', async () => {
      history.replaceState(null, '', '/applications/club');
      answer({ 'GET /api/v1/discord/opt-in': () => new Response(null, { status: 307 }) });
      render(ClubApplicationsPage);
      expect(await screen.findByText(t('leader.steps.inviteFound'))).toBeInTheDocument();
      expect(window.location.pathname).toBe('/applications/club/7');
      expect(screen.getByRole('heading', { level: 1, name: 'Example Club' })).toBeInTheDocument();
    });

    it('ticks the account and invite steps and links to adding the bot', async () => {
      answer({ 'GET /api/v1/discord/opt-in': () => new Response(null, { status: 307 }) });
      render(ClubApplicationsPage, { clubId: '7' });
      expect(await screen.findByText(t('leader.steps.inviteFound'))).toBeInTheDocument();
      expect(screen.getAllByRole('img', { name: t('leader.steps.done') })).toHaveLength(2);
      expect(screen.getByRole('link', { name: t('leader.addBot') })).toHaveAttribute(
        'href',
        '/api/v1/discord/opt-in?club=7&next=%2Fapplications%2Fclub%2F7%3Flinked%3D1',
      );
      expect(screen.getByText(t('leader.state.off'))).toBeInTheDocument();
    });

    it('holds the button back until an invite is found, and checks again', async () => {
      let found = false;
      const fetch = answer({
        'GET /api/v1/discord/opt-in': () =>
          found ? new Response(null, { status: 307 }) : json({ error: 'no invite' }, 422),
      });
      render(ClubApplicationsPage, { clubId: '7' });
      expect(await screen.findByText(t('leader.steps.inviteMissing'))).toBeInTheDocument();
      expect(screen.getByRole('button', { name: t('leader.addBot') })).toBeDisabled();
      found = true;
      await fireEvent.click(screen.getByRole('button', { name: t('leader.steps.checkAgain') }));
      expect(await screen.findByRole('link', { name: t('leader.addBot') })).toBeInTheDocument();
      expect(fetch.mock.calls.filter(([url]) => url.includes('opt-in'))).toHaveLength(2);
    });
  });

  describe('once the bot is added', () => {
    it('shows the status, the settings and the empty applications table', async () => {
      answer({ 'GET /api/v1/discord/clubs/7/settings': () => json(settings()) });
      render(ClubApplicationsPage, { clubId: '7', linked: true });
      expect(
        await screen.findByText(t('leader.state.open'), { selector: '.status' }),
      ).toBeInTheDocument();
      expect(screen.getByText(t('leader.linked'))).toBeInTheDocument();
      expect(screen.getByText(t('leader.table.empty'))).toBeInTheDocument();
    });

    it('lists the applications the club has had', async () => {
      answer({
        'GET /api/v1/discord/clubs/7/settings': () => json(settings()),
        'GET /api/v1/discord/clubs/7/applications': () =>
          json([
            {
              id: 'a',
              trainerName: 'Example Trainer',
              trainerId: '123456789',
              discordName: 'example_player',
              pace: 6_400_000,
              fansThisMonth: 48_000_000,
              currentClub: null,
              status: 'pending',
              submittedAt: new Date().toISOString(),
              ticketUrl: 'https://discord.com/channels/1/2',
            },
          ]),
      });
      render(ClubApplicationsPage, { clubId: '7', linked: true });
      expect(await screen.findByText('Example Trainer')).toBeInTheDocument();
      expect(screen.queryByText(t('leader.table.empty'))).not.toBeInTheDocument();
    });

    it('flags a setup problem', async () => {
      answer({
        'GET /api/v1/discord/clubs/7/settings': () =>
          json(settings({ problem: 'made-up problem' })),
      });
      render(ClubApplicationsPage, { clubId: '7' });
      expect(await screen.findByText(t('leader.state.problem'))).toBeInTheDocument();
      expect(screen.getByText(t('leader.callout.problem'))).toBeInTheDocument();
    });

    it('resumes paused applications', async () => {
      const fetch = answer({
        'GET /api/v1/discord/clubs/7/settings': () => json(settings({ open: false })),
        'PATCH /api/v1/discord/clubs/7/settings': () => json(settings({ open: true })),
      });
      render(ClubApplicationsPage, { clubId: '7' });
      await fireEvent.click(
        await screen.findByRole('button', { name: t('leader.callout.resume') }),
      );
      expect(
        await screen.findByText(t('leader.state.open'), { selector: '.status' }),
      ).toBeInTheDocument();
      const patch = fetch.mock.calls.find(([, init]) => init?.method === 'PATCH');
      expect(JSON.parse(String(patch?.[1]?.body))).toEqual({ open: true });
    });

    it('saves the settings', async () => {
      const fetch = answer({
        'GET /api/v1/discord/clubs/7/settings': () => json(settings()),
        'PATCH /api/v1/discord/clubs/7/settings': () => json(settings({ auto_join: false })),
      });
      render(ClubApplicationsPage, { clubId: '7' });
      await fireEvent.click(await screen.findByRole('switch', { name: /Add applicants/ }));
      await fireEvent.click(screen.getByRole('button', { name: t('leader.settings.save') }));
      expect(await screen.findByText(t('leader.settings.saved'))).toBeInTheDocument();
      const patch = fetch.mock.calls.find(([, init]) => init?.method === 'PATCH');
      expect(JSON.parse(String(patch?.[1]?.body))).toEqual({
        open: true,
        auto_join: false,
      });
    });

    it('removes the bot after asking, and goes back to the setup steps', async () => {
      answer({
        'GET /api/v1/discord/clubs/7/settings': () => json(settings()),
        'DELETE /api/v1/discord/clubs/7': () => new Response(null, { status: 204 }),
        'GET /api/v1/discord/opt-in': () => json({ error: 'no invite' }, 422),
      });
      render(ClubApplicationsPage, { clubId: '7' });
      await fireEvent.click(await screen.findByRole('button', { name: t('leader.remove.button') }));
      await fireEvent.click(screen.getByRole('button', { name: t('leader.remove.yes') }));
      expect(await screen.findByText(t('leader.steps.heading'))).toBeInTheDocument();
      expect(screen.getByText(t('leader.state.off'))).toBeInTheDocument();
    });
  });

  it.each([
    [401, 'leader.signIn'],
    [403, 'leader.forbidden'],
    [500, 'leader.loadError'],
  ] as const)('explains a %s from the server', async (status, key) => {
    answer({ 'GET /api/v1/discord/clubs/7/settings': () => json({ error: 'no' }, status) });
    render(ClubApplicationsPage, { clubId: '7' });
    expect(await screen.findByText(t(key))).toBeInTheDocument();
  });

  it('asks a signed-out visitor to sign in', async () => {
    answer({ 'GET /api/v1/discord/my-club': () => json({ error: 'no' }, 401) });
    render(ClubApplicationsPage);
    expect(await screen.findByText(t('leader.signIn'))).toBeInTheDocument();
  });
});
