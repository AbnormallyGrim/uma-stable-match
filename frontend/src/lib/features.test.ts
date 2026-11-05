// Feature switch tests with a stubbed fetch. All tokens are made up.

import { afterEach, describe, expect, it, vi } from 'vitest';

import { BEACON_SRC } from './analytics';
import { applyFeatures } from './features';
import { flags } from './flags.svelte';
import { language } from './language.svelte';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const beacon = () => document.head.querySelector(`script[src="${BEACON_SRC}"]`);

afterEach(() => {
  language.japaneseEnabled = false;
  flags.applications = false;
  beacon()?.remove();
});

describe('applyFeatures', () => {
  it('switches on what the server has on', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(
        json({ japanese: true, applications: true, web_analytics: 'madeuptoken123' }),
      ),
    );
    await applyFeatures({ fetch });
    expect(fetch).toHaveBeenCalledWith('/api/v1/features', expect.anything());
    expect(language.japaneseEnabled).toBe(true);
    expect(flags.applications).toBe(true);
    expect(beacon()).not.toBeNull();
  });

  it('leaves them all off when the request fails', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(json({ error: 'down' }, 500)),
    );
    await expect(applyFeatures({ fetch })).resolves.toBeUndefined();
    expect(language.japaneseEnabled).toBe(false);
    expect(flags.applications).toBe(false);
    expect(beacon()).toBeNull();
  });
});
