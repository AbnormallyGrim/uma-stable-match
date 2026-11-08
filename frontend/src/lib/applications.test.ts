// Applications API client tests with a stubbed fetch. All data is made up.

import { describe, expect, it, vi } from 'vitest';

import {
  activeApplications,
  applyUrl,
  checkTrainerLink,
  deleteAccount,
  discardVerificationKey,
  expiresAt,
  fetchClubApplications,
  fetchLinkedTrainers,
  fetchMyApplications,
  fetchMyClub,
  fetchSettings,
  hasInvite,
  loginUrl,
  optInUrl,
  saveSettings,
  signedInUser,
  requestVerificationKey,
  signOut,
  unlinkClub,
  unlinkTrainer,
  type Settings,
} from './applications';
import { ACCOUNT_KEY, saveAccount, sessionMarker, SIGNED_IN_COOKIE } from './session';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const settings: Settings = {
  guild_id: '5000',
  category: null,
  auto_join: true,
  open: true,
  problem: null,
  categories: [{ id: '10', name: 'Applications' }],
};

describe('addresses', () => {
  it('sends the visitor back to where they were', () => {
    expect(loginUrl()).toBe('/api/v1/auth/discord/login?next=%2Fapplications');
    expect(loginUrl('/applications/club/7')).toBe(
      '/api/v1/auth/discord/login?next=%2Fapplications%2Fclub%2F7',
    );
  });
});

describe('adding the bot', () => {
  it('starts the opt-in and comes back to the club page', () => {
    expect(optInUrl('7')).toBe(
      '/api/v1/discord/opt-in?club=7&next=%2Fapplications%2Fclub%2F7%3Flinked%3D1',
    );
  });

  it('reads a redirect as an invite and a 422 as none, without following it', async () => {
    const redirect = vi.fn<typeof fetch>(async () => new Response(null, { status: 307 }));
    await expect(hasInvite('7', { fetch: redirect })).resolves.toBe(true);
    expect(redirect.mock.calls[0]?.[1]).toMatchObject({ redirect: 'manual' });
    const none = vi.fn<typeof fetch>(async () => json({ error: 'no invite' }, 422));
    await expect(hasInvite('7', { fetch: none })).resolves.toBe(false);
  });

  it('fails on other answers, such as being signed out', async () => {
    const signedOut = vi.fn<typeof fetch>(async () => json({ error: 'sign in' }, 401));
    await expect(hasInvite('7', { fetch: signedOut })).rejects.toMatchObject({ status: 401 });
  });
});

describe('an ended session', () => {
  it('is forgotten on any 401, so later pages stop counting on it', async () => {
    document.cookie = `${SIGNED_IN_COOKIE}=1794170000; Path=/`;
    saveAccount('1794170000', { discordId: '42', name: null }, localStorage);
    const signedOut = vi.fn<typeof fetch>(async () => json({ error: 'sign in' }, 401));
    await expect(fetchMyClub({ fetch: signedOut })).rejects.toMatchObject({ status: 401 });
    expect(sessionMarker()).toBeNull();
    expect(localStorage.getItem(ACCOUNT_KEY)).toBeNull();
  });
});

describe('fetchMyClub', () => {
  it('gives the linked trainer and its club, and fails when signed out', async () => {
    const body = { trainer: '123456789', club: { id: '7', name: 'Example Club', leader: true } };
    const ok = vi.fn<typeof fetch>(async () => json(body));
    expect(await fetchMyClub({ fetch: ok })).toEqual(body);
    expect(ok.mock.calls[0]?.[0]).toBe('/api/v1/discord/my-club');
    const signedOut = vi.fn<typeof fetch>(async () => json({ error: 'sign in' }, 401));
    await expect(fetchMyClub({ fetch: signedOut })).rejects.toMatchObject({ status: 401 });
  });
});

describe('signedInUser', () => {
  it('gives the Discord ID and name, or null when signed out', async () => {
    const ok = vi.fn(() => Promise.resolve(json({ discord_id: '42', name: 'Made Up' })));
    expect(await signedInUser({ fetch: ok })).toEqual({ discordId: '42', name: 'Made Up' });
    const unnamed = vi.fn(() => Promise.resolve(json({ discord_id: '42', name: null })));
    expect(await signedInUser({ fetch: unnamed })).toEqual({ discordId: '42', name: null });
    const out = vi.fn(() => Promise.resolve(json({ error: 'not signed in' }, 401)));
    expect(await signedInUser({ fetch: out })).toBeNull();
  });

  it('fails on other errors', async () => {
    const down = vi.fn(() => Promise.resolve(json({ error: 'down' }, 503)));
    await expect(signedInUser({ fetch: down })).rejects.toMatchObject({ status: 503 });
    const offline = vi.fn(() => Promise.reject(new TypeError('offline')));
    await expect(signedInUser({ fetch: offline })).rejects.toMatchObject({ kind: 'network' });
  });
});

describe('settings', () => {
  it('reads the settings with the cookie', async () => {
    const read = vi.fn<typeof fetch>(() => Promise.resolve(json(settings)));
    expect(await fetchSettings('7', { fetch: read })).toEqual(settings);
    expect(read).toHaveBeenCalledWith(
      '/api/v1/discord/clubs/7/settings',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });

  it('surfaces who may not read them', async () => {
    for (const status of [401, 403, 404]) {
      const refused = vi.fn(() => Promise.resolve(json({ error: 'no' }, status)));
      await expect(fetchSettings('7', { fetch: refused })).rejects.toMatchObject({ status });
    }
  });

  it('saves a change as JSON and gives the new settings', async () => {
    const patch = vi.fn<typeof fetch>(() => Promise.resolve(json({ ...settings, open: false })));
    const saved = await saveSettings('7', { open: false, category: null }, { fetch: patch });
    expect(saved.open).toBe(false);
    const init = patch.mock.calls[0]![1]!;
    expect(init.method).toBe('PATCH');
    expect(init.body).toBe('{"open":false,"category":null}');
    expect(init.headers).toMatchObject({ 'Content-Type': 'application/json' });
  });
});

describe('unlinkClub and signOut', () => {
  it('send a DELETE and a POST and read the empty answers', async () => {
    const empty = vi.fn<typeof fetch>(() => Promise.resolve(new Response(null, { status: 204 })));
    await unlinkClub('7', { fetch: empty });
    await signOut({ fetch: empty });
    expect(empty.mock.calls.map(([path, init]) => [path, init?.method])).toEqual([
      ['/api/v1/discord/clubs/7', 'DELETE'],
      ['/api/v1/auth/sign-out', 'POST'],
    ]);
  });
});

describe('account', () => {
  it('deletes the account with a DELETE', async () => {
    const remove = vi.fn<typeof fetch>(() => Promise.resolve(new Response(null, { status: 204 })));
    await deleteAccount({ fetch: remove });
    expect(remove).toHaveBeenCalledWith(
      '/api/v1/auth/account',
      expect.objectContaining({ method: 'DELETE', credentials: 'same-origin' }),
    );
  });
});

describe('trainer links', () => {
  it('lists, starts, checks and unlinks with the trainer ID in the path', async () => {
    const calls: string[] = [];
    const fetch = vi.fn<typeof globalThis.fetch>((input, init) => {
      calls.push(`${init?.method ?? 'GET'} ${String(input)}`);
      if (String(input).endsWith('/verification')) {
        return Promise.resolve(json({ key: 'uma-ABCDEFGH', expires_in: 86400 }));
      }
      if (String(input).endsWith('/link')) {
        return Promise.resolve(json({ linked: true, name: 'Example Trainer' }));
      }
      if (init?.method === 'DELETE') {
        return Promise.resolve(new Response(null, { status: 204 }));
      }
      return Promise.resolve(json(list));
    });
    const list = {
      linked: [{ id: '100000000001', name: 'Example Trainer' }],
      max: 5,
      pending: [],
    };
    expect(await fetchLinkedTrainers({ fetch })).toEqual(list);
    expect(await requestVerificationKey('100000000002', { fetch })).toEqual({
      key: 'uma-ABCDEFGH',
      expires_in: 86400,
    });
    expect(await checkTrainerLink('100000000002', { fetch })).toEqual({
      linked: true,
      name: 'Example Trainer',
    });
    await discardVerificationKey('100000000002', { fetch });
    await unlinkTrainer('100000000002', { fetch });
    expect(calls).toEqual([
      'GET /api/v1/discord/trainers',
      'POST /api/v1/discord/trainers/100000000002/verification',
      'POST /api/v1/discord/trainers/100000000002/link',
      'DELETE /api/v1/discord/trainers/100000000002/verification',
      'DELETE /api/v1/discord/trainers/100000000002',
    ]);
  });

  it('gives a running check when the key is not in the comment yet', async () => {
    const later = vi.fn<typeof fetch>(() =>
      Promise.resolve(json({ checking: true, seconds_left: 300 }, 202)),
    );
    expect(await checkTrainerLink('100000000002', { fetch: later })).toEqual({
      checking: true,
      seconds_left: 300,
    });
  });
});

describe('application expiry', () => {
  it('expires one calendar month after sending, clamped to the month’s last day', () => {
    expect(expiresAt('2026-10-05T10:00:00Z').toISOString()).toBe('2026-11-05T10:00:00.000Z');
    expect(expiresAt('2026-12-20T08:30:00Z').toISOString()).toBe('2027-01-20T08:30:00.000Z');
    expect(expiresAt('2026-01-31T00:00:00Z').toISOString()).toBe('2026-02-28T00:00:00.000Z');
  });

  it('keeps only the applications that have not expired', () => {
    const sent = (submittedAt: string) => ({ submittedAt });
    const now = new Date('2026-11-05T12:00:00Z');
    expect(
      activeApplications(
        [sent('2026-10-05T11:59:00Z'), sent('2026-10-05T12:00:00Z'), sent('2026-11-01T00:00:00Z')],
        now,
      ),
    ).toEqual([sent('2026-11-01T00:00:00Z')]);
  });
});

describe('applications', () => {
  it('addresses applying to a club', () => {
    expect(applyUrl(7)).toBe('/api/v1/discord/apply?club=7');
    expect(applyUrl('12')).toBe('/api/v1/discord/apply?club=12');
  });

  it("reads the signed-in person's applications", async () => {
    const list = [{ id: 'a', clubId: '7', clubName: 'Example Club', status: 'pending' }];
    const fetch = vi.fn(() => Promise.resolve(json(list)));
    await expect(fetchMyApplications({ fetch })).resolves.toEqual(list);
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/discord/applications',
      expect.objectContaining({ credentials: 'same-origin' }),
    );
  });

  it("reads a club's applications", async () => {
    const fetch = vi.fn(() => Promise.resolve(json([])));
    await expect(fetchClubApplications('7', { fetch })).resolves.toEqual([]);
    expect(fetch).toHaveBeenCalledWith('/api/v1/discord/clubs/7/applications', expect.anything());
  });
});
