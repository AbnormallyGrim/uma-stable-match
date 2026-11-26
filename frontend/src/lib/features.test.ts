// Feature switch tests with a stubbed fetch. All tokens are made up.

import { afterEach, describe, expect, it, vi } from 'vitest';

import { BEACON_SRC } from './analytics';
import { applyFeatures } from './features';
import { language } from './language.svelte';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const beacon = () => document.head.querySelector(`script[src="${BEACON_SRC}"]`);

afterEach(() => {
  sessionStorage.clear();
  language.japaneseEnabled = false;
  beacon()?.remove();
});

describe('applyFeatures', () => {
  it('switches on what the server has on', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(json({ japanese: true, web_analytics: 'madeuptoken123' })),
    );
    await applyFeatures({ fetch });
    expect(fetch).toHaveBeenCalledWith('/api/v1/features', expect.anything());
    expect(language.japaneseEnabled).toBe(true);
    expect(beacon()).not.toBeNull();
  });

  it('leaves them all off when the request fails', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(json({ error: 'down' }, 500)),
    );
    await expect(applyFeatures({ fetch })).resolves.toBeUndefined();
    expect(language.japaneseEnabled).toBe(false);
    expect(beacon()).toBeNull();
  });

  it('asks the server once per session', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(json({ japanese: true, web_analytics: null })),
    );
    await applyFeatures({ fetch });
    language.japaneseEnabled = false;
    await applyFeatures({ fetch });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(language.japaneseEnabled).toBe(true);
  });

  it('asks again after a failed request', async () => {
    const fetch = vi
      .fn<typeof globalThis.fetch>()
      .mockResolvedValueOnce(json({ error: 'down' }, 500))
      .mockResolvedValueOnce(json({ japanese: true }));
    await applyFeatures({ fetch });
    await applyFeatures({ fetch });
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(language.japaneseEnabled).toBe(true);
  });

  it('ignores an unreadable saved answer', async () => {
    sessionStorage.setItem('uma-stable-match:features', '{"japanese":"yes"}');
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json({ japanese: true })));
    await applyFeatures({ fetch });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(language.japaneseEnabled).toBe(true);
  });

  it('asks every time without storage', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json({ japanese: true })));
    await applyFeatures({ fetch }, null);
    await applyFeatures({ fetch }, null);
    expect(fetch).toHaveBeenCalledTimes(2);
  });
});
