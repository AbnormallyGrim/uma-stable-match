// axe-core accessibility checks for the main views and states. All data is made up.

import { render, screen, fireEvent } from '@testing-library/svelte';
import axe from 'axe-core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import AdPanel from './lib/AdPanel.svelte';
import AdsPage from './lib/AdsPage.svelte';
import TrainerAdsTable from './lib/TrainerAdsTable.svelte';
import App from './App.svelte';
import type { ClubDetails, Recommendations } from './lib/api';
import ClubPage from './lib/ClubPage.svelte';
import HowToUse from './lib/HowToUse.svelte';
import { t } from './lib/i18n';
import ApplicationsPage from './lib/ApplicationsPage.svelte';
import ResultsTable from './lib/ResultsTable.svelte';
import SearchForm from './lib/SearchForm.svelte';
import SearchStatus from './lib/SearchStatus.svelte';
import ApplicationsTable from './lib/ApplicationsTable.svelte';
import ClubApplicationsPage from './lib/ClubApplicationsPage.svelte';
import { DEFAULT_SEARCH } from './lib/search';
import { club, recommendations } from './lib/test-data';
import ThemeToggle from './lib/ThemeToggle.svelte';
import TrainerApplicationsPage from './lib/TrainerApplicationsPage.svelte';
import TrainerApplicationsTable from './lib/TrainerApplicationsTable.svelte';
import { BOARD_TOUR_KEY, TOUR_KEY } from './lib/tour';

// axe's colour-contrast rule needs real rendering, so contrast.test.ts covers it instead.
// The landmark (region) rule only applies to whole-page checks.
async function violations(node: Element, wholePage = false) {
  const result = await axe.run(node, {
    rules: {
      'color-contrast': { enabled: false },
      region: { enabled: wholePage },
    },
  });
  return result.violations.map(
    (v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`,
  );
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

function stubApi(recommendationsResponse: () => Response) {
  vi.stubGlobal(
    'fetch',
    vi.fn(() => Promise.resolve(recommendationsResponse())),
  );
}

// Cumulative fans for five days at a made-up daily pace.
const fansAt = (pace: number) => Array.from({ length: 5 }, (_, day) => 100_000_000 + pace * day);

const balancedClubs = [1, 2, 3].map((id) =>
  club({
    circle_id: id,
    name: `Club ${id}`,
    url: `https://uma.moe/circles/${id}`,
    join_style: id === 3 ? 'closed' : 'open',
    balance: {
      score: 80,
      gini: 0.2,
      passengers: 1,
      counted: 20,
      bottom_quarter_per_day: 3_000_000,
      top3_share: 0.3,
    },
  }),
);

const withResults: Recommendations = recommendations({
  total: 3,
  clubs: balancedClubs,
  options: {
    balance_weight: 0.5,
    include_closed: true,
    include_full: false,
    join_styles: null,
    playstyles: null,
    has_discord: false,
    quick_apply: false,
    limit: 100,
  },
});

beforeEach(() => {
  localStorage.setItem(TOUR_KEY, '1');
  localStorage.setItem(BOARD_TOUR_KEY, '1');
});

afterEach(() => {
  vi.unstubAllGlobals();
  window.history.replaceState(null, '', '/');
  localStorage.clear();
});

describe('accessibility', () => {
  it('has no violations on the empty page', async () => {
    stubApi(() => json(withResults));
    render(App);
    await screen.findByRole('heading', { name: 'Uma Stable Match club finder' });
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations with the tour open', async () => {
    localStorage.removeItem(TOUR_KEY);
    stubApi(() => json(withResults));
    render(App);
    await screen.findByRole('dialog', { name: 'Welcome to Uma Stable Match' });
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations with the release notes open', async () => {
    stubApi(() => json(withResults));
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: "What's new" }));
    await screen.findByRole('dialog', { name: "What's new" });
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations with results, cards and the table', async () => {
    window.history.replaceState(null, '', '/?pace=7600000&balance_weight=0.5&include_closed=true');
    stubApi(() => json(withResults));
    render(App);
    await screen.findByRole('table');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations with coverage and last-month notices', async () => {
    window.history.replaceState(null, '', '/?pace=7600000');
    stubApi(() =>
      json({
        ...withResults,
        updated_at: '2026-03-02T15:00:00Z',
        month: '2026-02',
        coverage: { considered: 340, shortlisted: 9, scored: 3, pending: 6 },
      }),
    );
    render(App);
    await screen.findAllByTestId('result-notice');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations on the club page with the trainer placed', async () => {
    const details: ClubDetails = {
      club: club(),
      month: '2026-03',
      previous: null,
      updated_at: '2026-03-19T15:00:00Z',
      median_per_day: 6_400_000,
      members: [9_000_000, 5_000_000, 4_000_000].map((pace, i) => ({
        name: i === 2 ? null : `Made-up Trainer ${i + 1}`,
        pace_per_day: pace,
        daily_fans: fansAt(pace),
      })),
      trainer: {
        trainer_id: 123456789012,
        pace_per_day: 7_600_000,
        daily_fans: fansAt(7_600_000),
        in_club: true,
        your_place: 2,
        club_size: 4,
      },
    };
    stubApi(() => json(details));
    render(ClubPage, { route: { circleId: '1', trainerId: '123456789012' } });
    await screen.findByRole('slider');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations when the club page fails', async () => {
    stubApi(() => json({ error: 'club not loaded yet; try again later' }, 404));
    render(ClubPage, { route: { circleId: '1', trainerId: null } });
    await screen.findByRole('alert');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations when a search fails', async () => {
    window.history.replaceState(null, '', '/?trainer_id=123456789');
    stubApi(() => json({ error: 'trainer not found' }, 404));
    render(App);
    await screen.findByRole('alert');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations in the form with a validation error', async () => {
    const { container } = render(SearchForm, {
      initial: { ...DEFAULT_SEARCH, trainerId: '123' },
      remember: false,
      onsearch: vi.fn(),
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the pace form with balance on', async () => {
    const { container } = render(SearchForm, {
      initial: { ...DEFAULT_SEARCH, mode: 'pace', paceMillions: '7.6', balance: true },
      remember: false,
      onsearch: vi.fn(),
    });
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the sorted table', async () => {
    const { container } = render(ResultsTable, {
      clubs: balancedClubs,
      balanceOn: true,
      includeClosed: true,
    });
    await fireEvent.click(screen.getByRole('button', { name: /^Median/ }));
    expect(await violations(container)).toEqual([]);
  });

  it.each([
    { phase: 'loading' as const },
    { phase: 'waiting-for-scan' as const, retryIn: 5 },
    { phase: 'rate-limited' as const, retryIn: 3 },
    {
      phase: 'error' as const,
      message: { title: "Couldn't reach uma.moe", detail: 'Try again.', canRetry: true },
    },
  ])('has no violations in the $phase status', async (status) => {
    const { container } = render(SearchStatus, { status, onretry: vi.fn() });
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the open instructions', async () => {
    const { container } = render(HowToUse);
    container.querySelector('details')!.open = true;
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the theme switch', async () => {
    const { container } = render(ThemeToggle, { storage: localStorage });
    expect(await violations(container)).toEqual([]);
  });

  describe('the club applications pages', () => {
    const leader = () =>
      json({ trainer: '123456789', club: { id: '7', name: 'Example Club', leader: true } });
    const answer = (routes: Record<string, () => Response>) =>
      vi.stubGlobal(
        'fetch',
        vi.fn((input: string, init?: RequestInit) => {
          const all: Record<string, () => Response> = {
            'GET /api/v1/discord/my-club': leader,
            ...routes,
          };
          const route = all[`${init?.method ?? 'GET'} ${input.split('?')[0]}`];
          return Promise.resolve(route ? route() : json({ error: 'not found' }, 404));
        }),
      );
    const settings = {
      guild_id: '5000',
      category: '10',
      auto_join: true,
      open: true,
      problem: 'made-up problem',
      ticket_name: { mode: 'default', value: null },
      categories: [{ id: '10', name: 'Applications' }],
    };

    it('has no violations when signed out', async () => {
      answer({ 'GET /api/v1/auth/session': () => json({ error: 'no' }, 401) });
      const { container } = render(ApplicationsPage, { error: 'wrong_server' });
      expect(await violations(container)).toEqual([]);
    });

    it('has no violations when signed in', async () => {
      answer({ 'GET /api/v1/auth/session': () => json({ discord_id: '42' }) });
      const { container } = render(ApplicationsPage);
      await screen.findByText(t('applications.role.leader.title'));
      expect(await violations(container)).toEqual([]);
    });

    it('has no violations on a linked club, and when removing the bot', async () => {
      answer({ 'GET /api/v1/discord/clubs/7/settings': () => json(settings) });
      const { container } = render(ClubApplicationsPage, { clubId: '7', linked: true });
      await screen.findByRole('button', { name: t('leader.settings.save') });
      expect(await violations(container)).toEqual([]);
      await fireEvent.click(screen.getByRole('button', { name: t('leader.remove.button') }));
      expect(await violations(container)).toEqual([]);
    });

    it('has no violations while the bot is not set up', async () => {
      answer({ 'GET /api/v1/discord/opt-in': () => json({ error: 'no invite' }, 422) });
      const { container } = render(ClubApplicationsPage, { clubId: '7' });
      await screen.findByRole('button', { name: t('leader.steps.checkAgain') });
      expect(await violations(container)).toEqual([]);
    });

    it('has no violations before a trainer is linked', async () => {
      answer({ 'GET /api/v1/discord/my-club': () => json({ trainer: null, club: null }) });
      const { container } = render(ClubApplicationsPage);
      await screen.findByRole('link', { name: t('leader.steps.openSettings') });
      expect(await violations(container)).toEqual([]);
    });

    it.each([401, 403, 500])('has no violations when the settings answer %s', async (status) => {
      answer({ 'GET /api/v1/discord/clubs/7/settings': () => json({ error: 'no' }, status) });
      const { container } = render(ClubApplicationsPage, { clubId: '7' });
      await screen.findByText(/./, { selector: 'p.card, p[role="alert"], section p' });
      expect(await violations(container)).toEqual([]);
    });

    it("has no violations on a trainer's applications page and table", async () => {
      const page = render(TrainerApplicationsPage);
      expect(await violations(page.container)).toEqual([]);
      page.unmount();
      const { container } = render(TrainerApplicationsTable, {
        applications: [
          {
            id: 'a1',
            clubId: '100000001',
            clubName: 'Example Club',
            trainerId: '100000000011',
            status: 'pending',
            submittedAt: '2026-11-01T10:00:00Z',
            ticketUrl: 'https://discord.com/channels/1/2',
          },
        ],
        now: new Date('2026-11-05T00:00:00Z'),
      });
      expect(await violations(container)).toEqual([]);
    });

    it("has no violations in the ad panel and a trainer's ad table", async () => {
      const mine = {
        trainerName: 'Example Trainer',
        trainerId: '100000000011',
        currentClub: 'Example Club',
        pace: 6_400_000,
        fansThisMonth: 48_000_000,
        rankMin: 3,
        rankMax: 8,
        discord: 'required' as const,
        language: 'en' as const,
        postedAt: '2026-11-01T00:00:00Z',
        visibleUntil: '2026-11-08T00:00:00Z',
        expiresAt: '2026-12-01T00:00:00Z',
        onBoard: true,
        invites: [
          { clubId: '100000001', clubName: 'Example Club', sentAt: '2026-11-02T00:00:00Z' },
        ],
      };
      for (const ad of [null, mine]) {
        const panel = render(AdPanel, { ad });
        expect(await violations(panel.container)).toEqual([]);
        panel.unmount();
        const table = render(TrainerAdsTable, { ad });
        expect(await violations(table.container)).toEqual([]);
        table.unmount();
      }
    });

    function stubBoard() {
      const board = [
        {
          id: 'a1',
          trainerName: 'Trainer Alpha',
          trainerId: '100000000011',
          currentClub: null,
          pace: 1_420_000,
          fansThisMonth: 18_300_000,
          rankMin: 1,
          rankMax: 11,
          discord: 'required',
          language: 'ja',
          postedAt: '2026-11-01T00:00:00Z',
          visibleUntil: '2026-11-08T00:00:00Z',
          invited: false,
        },
      ];
      vi.stubGlobal(
        'fetch',
        vi.fn((uri: string) => {
          const path = uri.split('?')[0];
          return Promise.resolve(
            json(
              path === '/api/v1/discord/ads'
                ? { ads: board, page: 1, pages: 1, total: board.length }
                : path === '/api/v1/discord/my-club'
                  ? { trainer: '100000000011', club: { id: '100000001', name: 'X', leader: true } }
                  : path === '/api/v1/discord/ads/me'
                    ? null
                    : {},
            ),
          );
        }),
      );
    }

    it('has no violations on the board of ads, and in its dialog', async () => {
      stubBoard();
      const { container } = render(AdsPage);
      await screen.findByText('Trainer Alpha');
      expect(await violations(container)).toEqual([]);
      await fireEvent.click(screen.getByRole('button', { name: t('ads.post') }));
      expect(await violations(screen.getByRole('dialog'))).toEqual([]);
    });

    it('has no violations on the board with its tour open', async () => {
      localStorage.removeItem(BOARD_TOUR_KEY);
      stubBoard();
      render(AdsPage);
      await screen.findByRole('dialog', { name: 'Get found by clubs' });
      expect(await violations(document.body, true)).toEqual([]);
    });

    it('has no violations in the applications table and an open application', async () => {
      const application = {
        id: 'a1',
        trainerName: 'Trainer Alpha',
        trainerId: '100000000011',
        discordName: 'alpha_applicant',
        pace: 1_420_000,
        fansThisMonth: 18_300_000,
        currentClub: null,
        status: 'pending' as const,
        submittedAt: '2026-11-01T10:00:00Z',
        ticketUrl: 'https://discord.com/channels/1/2',
      };
      const { container } = render(ApplicationsTable, {
        applications: [application],
        clubName: 'Example Club',
        clubId: '7',
      });
      expect(await violations(container)).toEqual([]);
      await fireEvent.click(
        screen.getByRole('button', { name: t('leader.table.viewFor', { name: 'Trainer Alpha' }) }),
      );
      expect(await violations(document.body)).toEqual([]);
    });
  });
});
