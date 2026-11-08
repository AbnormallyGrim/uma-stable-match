// Tests for working out who is signed in, with a stubbed fetch. All IDs and names are made up.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ACCOUNT_KEY, saveAccount, sessionMarker, SIGNED_IN_COOKIE } from './session';

const marker = '1794170000';
const person = { discordId: '100000000000000001', name: 'Made Up' };
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status });

let fetch: ReturnType<typeof vi.fn<typeof globalThis.fetch>>;

/** A fresh copy of the module, since it asks at most once per page. */
async function load() {
  vi.resetModules();
  return import('./account.svelte');
}

beforeEach(() => {
  fetch = vi.fn<typeof globalThis.fetch>(() =>
    Promise.resolve(json({ discord_id: person.discordId, name: person.name })),
  );
  vi.stubGlobal('fetch', fetch);
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
  document.cookie = `${SIGNED_IN_COOKIE}=; Path=/; Max-Age=0`;
});

describe('loadAccount', () => {
  it("doesn't ask the server without the signed-in marker", async () => {
    const { account, loadAccount } = await load();
    await expect(loadAccount()).resolves.toBeNull();
    expect(fetch).not.toHaveBeenCalled();
    expect(account).toEqual({ current: null, known: true });
  });

  it('asks once after signing in, then reuses the saved person', async () => {
    document.cookie = `${SIGNED_IN_COOKIE}=${marker}; Path=/`;
    let { account, loadAccount } = await load();
    await expect(loadAccount()).resolves.toEqual(person);
    expect(fetch).toHaveBeenCalledWith('/api/v1/auth/session', expect.anything());
    expect(account.current).toEqual(person);

    // The next page.
    ({ account, loadAccount } = await load());
    await expect(loadAccount()).resolves.toEqual(person);
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(account.current).toEqual(person);
  });

  it('asks again for a new session', async () => {
    saveAccount('1794100000', { ...person, name: 'Old Name' }, localStorage);
    document.cookie = `${SIGNED_IN_COOKIE}=${marker}; Path=/`;
    const { loadAccount } = await load();
    await expect(loadAccount()).resolves.toEqual(person);
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('forgets the marker when the server says the session has ended', async () => {
    fetch.mockResolvedValue(json({ error: 'not signed in' }, 401));
    document.cookie = `${SIGNED_IN_COOKIE}=${marker}; Path=/`;
    const { loadAccount } = await load();
    await expect(loadAccount()).resolves.toBeNull();
    expect(sessionMarker()).toBeNull();
  });
});

describe('clearAccount', () => {
  it('forgets the saved person and the marker', async () => {
    document.cookie = `${SIGNED_IN_COOKIE}=${marker}; Path=/`;
    saveAccount(marker, person, localStorage);
    const { account, clearAccount } = await load();
    clearAccount();
    expect(localStorage.getItem(ACCOUNT_KEY)).toBeNull();
    expect(sessionMarker()).toBeNull();
    expect(account).toEqual({ current: null, known: true });
  });

  it('follows a sign-out in another tab', async () => {
    document.cookie = `${SIGNED_IN_COOKIE}=${marker}; Path=/`;
    saveAccount(marker, person, localStorage);
    const { account, loadAccount } = await load();
    await loadAccount();
    expect(account.current).toEqual(person);
    window.dispatchEvent(new StorageEvent('storage', { key: ACCOUNT_KEY, newValue: null }));
    expect(account.current).toBeNull();
  });
});
