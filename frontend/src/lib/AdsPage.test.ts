// AdsPage tests. All data is made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { BoardAd, MyAd } from './ads';
import AdsPage from './AdsPage.svelte';
import { t } from './i18n';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

function ad(id: string, over: Partial<BoardAd> = {}): BoardAd {
  return {
    id,
    trainerName: `Trainer ${id}`,
    trainerId: `10000000${id}`,
    currentClub: 'Other Club',
    pace: 1_000_000,
    fansThisMonth: 10_000_000,
    rankMin: 1,
    rankMax: 11,
    discord: 'any',
    language: 'any',
    postedAt: `2026-11-0${id}T00:00:00Z`,
    visibleUntil: '2026-11-10T00:00:00Z',
    invited: false,
    ...over,
  };
}

const board = [
  ad('1', { pace: 2_000_000, rankMin: 5, rankMax: 8, language: 'en' }),
  ad('2', { pace: 5_000_000, discord: 'required' }),
  ad('3', { pace: null }),
  ad('4', { invited: true }),
];

const mine: MyAd = {
  trainerName: 'Example Trainer',
  trainerId: '123456789',
  currentClub: null,
  pace: 1_000_000,
  fansThisMonth: 10_000_000,
  rankMin: 1,
  rankMax: 11,
  discord: 'any',
  language: 'any',
  postedAt: '2026-11-01T00:00:00Z',
  visibleUntil: '2026-11-08T00:00:00Z',
  expiresAt: '2026-12-01T00:00:00Z',
  onBoard: true,
  invites: [],
};

interface Setup {
  /** The signed-in trainer's own ad. */
  mine?: MyAd | null;
  /** The club they lead, or `null` for none. */
  leads?: string | null;
  /** The ads on the server's board; it pages them 20 at a time, fastest first, unfiltered. */
  board?: BoardAd[];
  boardStatus?: number;
}

/** Answers each address the page reads, and remembers what was asked. */
function stub(setup: Setup = {}) {
  const fetch = vi.fn((uri: string, init?: RequestInit) => {
    const [path, search] = uri.split('?') as [string, string | undefined];
    if (path === '/api/v1/discord/ads/me') {
      return Promise.resolve(json(setup.mine ?? null));
    }
    if (path === '/api/v1/discord/my-club') {
      const club = setup.leads ? { id: setup.leads, name: 'Example Club', leader: true } : null;
      return Promise.resolve(json({ trainer: '123456789', club }));
    }
    if (path === '/api/v1/discord/ads' && (init?.method ?? 'GET') === 'GET') {
      if (setup.boardStatus) {
        return Promise.resolve(json({ error: 'down' }, setup.boardStatus));
      }
      const all = [...(setup.board ?? board)].sort((a, b) => (b.pace ?? -1) - (a.pace ?? -1));
      const pages = Math.max(1, Math.ceil(all.length / 20));
      const page = Math.min(Number(new URLSearchParams(search).get('page') ?? '1'), pages);
      const ads = all.slice((page - 1) * 20, page * 20);
      return Promise.resolve(json({ ads, page, pages, total: all.length }));
    }
    if (path.endsWith('/invite')) {
      return Promise.resolve(new Response(null, { status: 204 }));
    }
    return Promise.resolve(json({}));
  });
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

/** The query of the last request for the board. */
function lastBoardQuery(fetch: ReturnType<typeof vi.fn>): URLSearchParams {
  const uris = (fetch.mock.calls as unknown as [string][])
    .map(([uri]) => uri)
    .filter((uri) => uri.startsWith('/api/v1/discord/ads?'));
  return new URLSearchParams(uris.at(-1)!.split('?')[1]);
}

function asked(fetch: ReturnType<typeof vi.fn>): string[] {
  return (fetch.mock.calls as unknown as [string, RequestInit?][])
    .map(([uri, init]) => `${init?.method ?? 'GET'} ${uri}`)
    .filter((call) => !call.includes('/features'));
}

/** Opens a dropdown and picks one of its options, as a visitor would. */
async function choose(label: string, option: string) {
  await fireEvent.click(await screen.findByRole('combobox', { name: label }));
  await fireEvent.click(screen.getByRole('option', { name: option }));
}

function names(): string[] {
  return screen
    .getAllByRole('row')
    .slice(1)
    .map((row) => within(row).getAllByRole('link')[0]!.textContent?.trim() ?? '');
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('AdsPage', () => {
  it('has a heading, a description and a button to post an ad', async () => {
    stub();
    render(AdsPage);
    expect(screen.getByRole('heading', { level: 1, name: t('ads.heading') })).toBeInTheDocument();
    expect(screen.getByText(t('ads.lede'))).toBeInTheDocument();
    expect(await screen.findByRole('button', { name: t('ads.post') })).toBeInTheDocument();
    expect(document.title).toBe(t('ads.title'));
  });

  it('lists the ads fastest first with what each trainer wants', async () => {
    stub();
    render(AdsPage);
    expect(await screen.findByText('Trainer 2')).toBeInTheDocument();
    expect(names()).toEqual(['Trainer 2', 'Trainer 1', 'Trainer 4', 'Trainer 3']);
    expect(screen.getByText('5.0M/day')).toBeInTheDocument();
    expect(screen.getByText('B to A+')).toBeInTheDocument();
    expect(screen.getByText(t('ads.table.discord.required'))).toBeInTheDocument();
  });

  it('shows no invite buttons to people who don’t lead a club', async () => {
    const fetch = stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    expect(screen.queryByRole('button', { name: /Invite/ })).not.toBeInTheDocument();
    expect(lastBoardQuery(fetch).has('club')).toBe(false);
  });

  it('has only the club rank, Discord and language filters', async () => {
    stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    const filters = screen.getByRole('search', { name: t('ads.filter.label') });
    expect(
      within(filters)
        .getAllByRole('combobox')
        .map((box) => box.id),
    ).toEqual(['ads-rank-min', 'ads-rank-max', 'ads-discord', 'ads-language']);
    expect(within(filters).queryByRole('textbox')).not.toBeInTheDocument();
  });

  it('asks the server for the first page with no filters', async () => {
    const fetch = stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    // Every filter starts unset, so only the page is sent.
    expect(Object.fromEntries(lastBoardQuery(fetch))).toEqual({ page: '1' });
    expect(screen.getByText(t('ads.count', { n: 4 }))).toBeInTheDocument();
    for (const label of [
      'ads.filter.rankMin',
      'ads.filter.rankMax',
      'ads.filter.discord',
    ] as const) {
      expect(screen.getByRole('combobox', { name: t(label) })).toHaveTextContent(
        t('ads.filter.any'),
      );
    }
  });

  it('sends only the filters that are set, and unsets them again', async () => {
    const fetch = stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    await choose(t('ads.filter.rankMin'), 'A+');
    await vi.waitFor(() =>
      expect(Object.fromEntries(lastBoardQuery(fetch))).toEqual({ page: '1', rank_min: '8' }),
    );
    await choose(t('ads.filter.rankMin'), t('ads.filter.any'));
    await vi.waitFor(() =>
      expect(Object.fromEntries(lastBoardQuery(fetch))).toEqual({ page: '1' }),
    );
  });

  it('clears every filter at once', async () => {
    const fetch = stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    const clear = screen.getByRole('button', { name: t('ads.filter.clear') });
    expect(clear).toBeDisabled();
    await choose(t('ads.filter.rankMax'), 'B');
    await choose(t('ads.filter.discord'), t('ads.filter.discord.no'));
    await choose(t('ads.filter.language'), t('ad.language.ja'));
    expect(clear).toBeEnabled();
    await fireEvent.click(clear);
    await vi.waitFor(() =>
      expect(Object.fromEntries(lastBoardQuery(fetch))).toEqual({ page: '1' }),
    );
    expect(screen.getByRole('combobox', { name: t('ads.filter.language') })).toHaveTextContent(
      t('ads.filter.any'),
    );
    expect(clear).toBeDisabled();
  });

  it('links each trainer to their uma.moe profile', async () => {
    stub();
    render(AdsPage);
    const link = await screen.findByRole('link', { name: 'Trainer 2' });
    expect(link).toHaveAttribute('href', 'https://uma.moe/profile/100000002');
    expect(link).toHaveAttribute('target', '_blank');
  });

  it('opens the whole ad in a dialog from its View button or a click on its row', async () => {
    stub({ leads: '7' });
    render(AdsPage);
    await fireEvent.click(
      await screen.findByRole('button', { name: t('ads.viewAd', { name: 'Trainer 1' }) }),
    );
    let dialog = screen.getByRole('dialog', { name: 'Trainer 1' });
    expect(within(dialog).getByText('100000001')).toBeInTheDocument();
    expect(within(dialog).getByText('B to A+')).toBeInTheDocument();
    expect(within(dialog).getByText(t('ad.language.en'))).toBeInTheDocument();
    expect(within(dialog).getByRole('link', { name: t('ads.details.profile') })).toHaveAttribute(
      'href',
      'https://uma.moe/profile/100000001',
    );
    // A leader can invite from the dialog too.
    await fireEvent.click(
      within(dialog).getByRole('button', { name: t('ads.inviteTrainer', { name: 'Trainer 1' }) }),
    );
    expect(await within(dialog).findByText(t('ads.invited'))).toBeInTheDocument();
    await fireEvent.click(within(dialog).getByRole('button', { name: t('modal.close') }));

    const row = screen.getByRole('link', { name: 'Trainer 2' }).closest('tr')!;
    await fireEvent.click(within(row).getByText('5.0M/day'));
    dialog = screen.getByRole('dialog', { name: 'Trainer 2' });
    expect(within(dialog).getByText(t('ad.discord.required'))).toBeInTheDocument();
  });

  it('doesn’t open the dialog from a click on a link or button in the row', async () => {
    stub({ leads: '7' });
    render(AdsPage);
    await fireEvent.click(
      await screen.findByRole('button', { name: t('ads.inviteTrainer', { name: 'Trainer 2' }) }),
    );
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('sends each filter to the server', async () => {
    const fetch = stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    await choose(t('ads.filter.rankMin'), 'C');
    await choose(t('ads.filter.rankMax'), 'A+');
    await choose(t('ads.filter.discord'), t('ads.filter.discord.no'));
    await choose(t('ads.filter.language'), t('ad.language.ko'));
    await vi.waitFor(() =>
      expect(Object.fromEntries(lastBoardQuery(fetch))).toEqual({
        page: '1',
        rank_min: '3',
        rank_max: '8',
        has_discord: 'false',
        language: 'ko',
      }),
    );
  });

  it('says when no ads match the filters', async () => {
    const fetch = stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    fetch.mockImplementation((uri: string) =>
      Promise.resolve(
        uri.startsWith('/api/v1/discord/ads?')
          ? json({ ads: [], page: 1, pages: 1, total: 0 })
          : json({}),
      ),
    );
    await choose(t('ads.filter.language'), t('ad.language.ko'));
    expect(await screen.findByText(t('ads.noMatch'))).toBeInTheDocument();
  });

  it('asks the server for each page, and starts again when a filter changes', async () => {
    const many = Array.from({ length: 25 }, (_, index) =>
      ad(String(index + 10), {
        pace: 10_000_000 - index,
        postedAt: '2026-11-01T00:00:00Z',
      }),
    );
    const fetch = stub({ board: many });
    render(AdsPage);
    await screen.findByText('Trainer 10');
    expect(names()).toHaveLength(20);
    expect(screen.getByText(t('ads.count', { n: 25 }))).toBeInTheDocument();
    expect(screen.getByText(t('ads.page.status', { page: 1, pages: 2 }))).toBeInTheDocument();
    expect(screen.getByRole('button', { name: t('ads.page.prev') })).toBeDisabled();

    await fireEvent.click(screen.getByRole('button', { name: t('ads.page.next') }));
    await screen.findByText('Trainer 34');
    expect(names()).toHaveLength(5);
    expect(lastBoardQuery(fetch).get('page')).toBe('2');
    expect(screen.getByRole('button', { name: t('ads.page.next') })).toBeDisabled();

    await choose(t('ads.filter.discord'), t('ads.filter.discord.no'));
    await vi.waitFor(() => expect(lastBoardQuery(fetch).get('page')).toBe('1'));
    expect(lastBoardQuery(fetch).get('has_discord')).toBe('false');
  });

  it('keeps the ads it has when a later page can’t be read', async () => {
    const many = Array.from({ length: 25 }, (_, index) =>
      ad(String(index + 10), { postedAt: '2026-11-01T00:00:00Z' }),
    );
    const fetch = stub({ board: many });
    render(AdsPage);
    await screen.findByText('Trainer 10');
    fetch.mockImplementation(() => Promise.resolve(json({ error: 'down' }, 502)));
    await fireEvent.click(screen.getByRole('button', { name: t('ads.page.next') }));
    expect(await screen.findByText(t('ads.loadError'))).toBeInTheDocument();
    expect(names()).toHaveLength(20);
  });

  it('has no pager for a short board', async () => {
    stub();
    render(AdsPage);
    await screen.findByText('Trainer 2');
    expect(screen.queryByRole('navigation', { name: t('ads.page.label') })).not.toBeInTheDocument();
  });

  it('lets a leader invite a trainer and marks them invited', async () => {
    const fetch = stub({ leads: '7' });
    render(AdsPage);
    await fireEvent.click(
      await screen.findByRole('button', { name: t('ads.inviteTrainer', { name: 'Trainer 2' }) }),
    );
    expect(await screen.findAllByText(t('ads.invited'))).toHaveLength(2);
    expect(asked(fetch)).toContain('POST /api/v1/discord/clubs/7/ads/2/invite');
    expect(lastBoardQuery(fetch).get('club')).toBe('7');
  });

  it('says why an invitation wasn’t sent', async () => {
    const fetch = stub({ leads: '7' });
    render(AdsPage);
    const button = await screen.findByRole('button', {
      name: t('ads.inviteTrainer', { name: 'Trainer 2' }),
    });
    fetch.mockImplementationOnce(() => Promise.resolve(json({ error: 'full' }, 409)));
    await fireEvent.click(button);
    expect(await screen.findByText(t('ads.inviteRefused'))).toBeInTheDocument();
    expect(screen.getAllByText(t('ads.invited'))).toHaveLength(1);
  });

  it('opens a dialog to post an ad, and reads the board again once it’s posted', async () => {
    const fetch = stub();
    render(AdsPage);
    await fireEvent.click(await screen.findByRole('button', { name: t('ads.post') }));
    const dialog = screen.getByRole('dialog', { name: t('ad.heading') });
    expect(within(dialog).getByRole('button', { name: t('ad.form.post') })).toBeInTheDocument();

    fetch.mockImplementationOnce(() => Promise.resolve(json(mine)));
    await fireEvent.click(within(dialog).getByRole('button', { name: t('ad.form.post') }));
    expect(await within(dialog).findByText('Example Trainer')).toBeInTheDocument();
    const calls = asked(fetch);
    expect(calls).toContain('PUT /api/v1/discord/ads/me');
    await vi.waitFor(() =>
      expect(asked(fetch).at(-1)).toMatch(/^GET \/api\/v1\/discord\/ads\?page=1/),
    );

    await fireEvent.click(screen.getByRole('button', { name: t('modal.close') }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('labels the button “My ad” for a trainer who has one, and closes with Escape', async () => {
    stub({ mine });
    render(AdsPage);
    await fireEvent.click(await screen.findByRole('button', { name: t('ads.myAd') }));
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByText('Example Trainer')).toBeInTheDocument();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('opens the dialog straight away when asked to', async () => {
    stub();
    render(AdsPage, { post: true });
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
  });

  it('shows an empty board', async () => {
    stub({ board: [] });
    render(AdsPage);
    expect(await screen.findByText(t('ads.empty'))).toBeInTheDocument();
  });

  it('says when the board can’t be loaded', async () => {
    stub({ boardStatus: 502 });
    render(AdsPage);
    expect(await screen.findByText(t('ads.loadError'))).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: t('ads.post') })).not.toBeInTheDocument();
  });
});
