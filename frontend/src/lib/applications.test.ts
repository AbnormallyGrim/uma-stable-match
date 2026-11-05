// Applications API client tests with a stubbed fetch. All data is made up.

import { describe, expect, it, vi } from 'vitest';

import {
  fetchSettings,
  hasInvite,
  loginUrl,
  optInUrl,
  saveSettings,
  signedInUser,
  signOut,
  unlinkClub,
  type Settings,
} from './applications';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const settings: Settings = {
  guild_id: '5000',
  category: null,
  leader_roles: [],
  auto_join: true,
  open: true,
  problem: null,
  categories: [{ id: '10', name: 'Applications' }],
  roles: [{ id: '20', name: 'Officers' }],
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

describe('signedInUser', () => {
  it('gives the Discord ID, or null when signed out', async () => {
    const ok = vi.fn(() => Promise.resolve(json({ discord_id: '42' })));
    expect(await signedInUser({ fetch: ok })).toBe('42');
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
