// Applications API client tests with a stubbed fetch. All data is made up.

import { describe, expect, it, vi } from 'vitest';

import { ApiError } from './api';
import {
  checkOptIn,
  fetchSettings,
  loginUrl,
  optInUrl,
  parseClubInput,
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

describe('parseClubInput', () => {
  it('reads a club ID from a direct frontend value', () => {
    expect(parseClubInput('123456789')).toBe('123456789');
    expect(parseClubInput('  123456789 ')).toBe('123456789');
  });

  it('refuses URLs and invalid values', () => {
    for (const input of [
      '',
      'abc',
      '12 34',
      '1'.repeat(19),
      'https://example.com/circles/123',
      'https://uma.moe/circles/123',
      'uma.moe/circles/123',
    ]) {
      expect(parseClubInput(input), input).toBeNull();
    }
  });
});

describe('addresses', () => {
  it('sends the visitor back to where they were', () => {
    expect(loginUrl()).toBe('/api/v1/auth/discord/login?next=%2Fapplications');
    expect(loginUrl('/applications/club/7')).toBe(
      '/api/v1/auth/discord/login?next=%2Fapplications%2Fclub%2F7',
    );
  });

  it("opts a club in and ends on the club's settings", () => {
    expect(optInUrl('7')).toBe(
      '/api/v1/discord/opt-in?club=7&next=%2Fapplications%2Fclub%2F7%3Flinked%3D1',
    );
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

describe('checkOptIn', () => {
  it('passes when the server redirects to Discord', async () => {
    const redirect = vi.fn(() =>
      Promise.resolve({ type: 'opaqueredirect', status: 0 } as Response),
    );
    await expect(checkOptIn('7', { fetch: redirect })).resolves.toBeUndefined();
    expect(redirect).toHaveBeenCalledWith(
      optInUrl('7'),
      expect.objectContaining({ redirect: 'manual' }),
    );
  });

  it("fails with the server's status when it can't start", async () => {
    const refused = vi.fn(() => Promise.resolve(json({ error: 'no invite' }, 422)));
    const error = await checkOptIn('7', { fetch: refused }).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(ApiError);
    expect(error).toMatchObject({ status: 422, message: 'no invite' });
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