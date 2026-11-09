// Ads module tests. All data is made up.

import { describe, expect, it, vi } from 'vitest';

import {
  dressIconUrl,
  deleteAd,
  dismissInvite,
  fetchBoard,
  fetchMyAd,
  inviteTrainer,
  NO_FILTERS,
  rankLabel,
  renewAd,
  saveAd,
  DEFAULT_CHOICES,
  type BoardAd,
} from './ads';
import { ApiError } from './api';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

function ad(id: string, over: Partial<BoardAd> = {}): BoardAd {
  return {
    id,
    trainerName: `Trainer ${id}`,
    trainerId: `10000000${id}`,
    currentClub: null,
    pace: 1_000_000,
    fansThisMonth: 10_000_000,
    leaderCharaDressId: null,
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

describe('dressIconUrl', () => {
  it('finds the icon for an outfit we have an image of', () => {
    expect(dressIconUrl(100102)).toMatch(/chara_stand_1001_100102.*\.webp$/);
  });

  it('is null for an outfit we don’t have', () => {
    expect(dressIconUrl(1)).toBeNull();
  });
});

describe('rankLabel', () => {
  it('names the tiers from D to SS', () => {
    expect(rankLabel(1)).toBe('D');
    expect(rankLabel(8)).toBe('A+');
    expect(rankLabel(11)).toBe('SS');
  });
});

describe('the ad calls', () => {
  it('reads the trainer’s ad, or none', async () => {
    const fetch = vi.fn(() => Promise.resolve(json(null)));
    expect(await fetchMyAd({ fetch })).toBeNull();
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/discord/ads/me',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });

  it('posts the choices', async () => {
    const fetch = vi.fn(() => Promise.resolve(json({ trainerName: 'Example' })));
    await saveAd(DEFAULT_CHOICES, { fetch });
    const [uri, init] = (fetch.mock.calls as unknown as [string, RequestInit][])[0]!;
    expect(uri).toBe('/api/v1/discord/ads/me');
    expect(init.method).toBe('PUT');
    expect(JSON.parse(init.body as string)).toEqual(DEFAULT_CHOICES);
  });

  it('renews, deletes, dismisses and invites with the right verbs and addresses', async () => {
    const fetch = vi.fn(() => Promise.resolve(new Response(null, { status: 204 })));
    await renewAd({ fetch }).catch(() => undefined);
    await deleteAd({ fetch });
    await dismissInvite('7', { fetch });
    await inviteTrainer('7', 'abc', { fetch });
    const calls = (fetch.mock.calls as unknown as [string, RequestInit][]).map(([uri, init]) => [
      init.method,
      uri,
    ]);
    expect(calls).toEqual([
      ['POST', '/api/v1/discord/ads/me/renew'],
      ['DELETE', '/api/v1/discord/ads/me'],
      ['DELETE', '/api/v1/discord/ads/me/invites/7'],
      ['POST', '/api/v1/discord/clubs/7/ads/abc/invite'],
    ]);
  });

  it('reads the board and passes on why it was refused', async () => {
    const page = { ads: [ad('1')], page: 2, pages: 3, total: 41 };
    const fetch = vi.fn(() => Promise.resolve(json(page)));
    expect(await fetchBoard(null, NO_FILTERS, 2, { fetch })).toEqual(page);
    await fetchBoard('7', { rankMin: 3, rankMax: 8, hasDiscord: false, language: 'ko' }, 1, {
      fetch,
    });
    const uris = (fetch.mock.calls as unknown as [string][]).map(([uri]) => uri);
    expect(uris).toEqual([
      '/api/v1/discord/ads?page=2',
      '/api/v1/discord/ads?page=1&rank_min=3&rank_max=8&has_discord=false&language=ko&club=7',
    ]);
    const refused = vi.fn(() => Promise.resolve(json({ error: 'no' }, 403)));
    await expect(fetchBoard('7', NO_FILTERS, 1, { fetch: refused })).rejects.toMatchObject({
      status: 403,
    } satisfies Partial<ApiError>);
  });
});
