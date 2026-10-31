// Feature switch tests with a stubbed fetch. All tokens are made up.

import { afterEach, describe, expect, it, vi } from 'vitest';

import { BEACON_SRC } from './analytics';
import { applyFeatures } from './features';
import { language } from './language.svelte';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const beacon = () => document.head.querySelector(`script[src="${BEACON_SRC}"]`);

afterEach(() => {
  language.japaneseEnabled = false;
  beacon()?.remove();
});

describe('applyFeatures', () => {
  it('offers Japanese and loads the beacon when the server has them on', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(json({ japanese: true, web_analytics: 'madeuptoken123' })),
    );
    await applyFeatures({ fetch });
    expect(fetch).toHaveBeenCalledWith('/api/v1/features', expect.anything());
    expect(language.japaneseEnabled).toBe(true);
    expect(beacon()).not.toBeNull();
  });

  it('leaves both off when the request fails', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(json({ error: 'down' }, 500)),
    );
    await expect(applyFeatures({ fetch })).resolves.toBeUndefined();
    expect(language.japaneseEnabled).toBe(false);
    expect(beacon()).toBeNull();
  });
});
