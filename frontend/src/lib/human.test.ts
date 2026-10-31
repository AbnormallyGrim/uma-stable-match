// Human check tests with a stubbed fetch and Turnstile. All keys and tokens are made up.

import { afterEach, describe, expect, it, vi } from 'vitest';

import { ApiError, fetchRecommendations } from './api';
import { HUMAN_UNTIL_KEY, HumanCheck, type Turnstile } from './human';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

// A Turnstile that answers "made-up-response" at once, unless told to fail.
function turnstile(outcome: 'pass' | 'fail' = 'pass') {
  const widget = {
    render: vi.fn<Turnstile['render']>((_container, options) => {
      if (outcome === 'pass') options.callback('made-up-response');
      else options['error-callback']();
      return 'widget-1';
    }),
    remove: vi.fn<Turnstile['remove']>(),
  };
  return widget;
}

function server(handler: (url: string, init?: RequestInit) => Response) {
  return vi.fn<typeof globalThis.fetch>((input, init) =>
    Promise.resolve(handler(String(input), init)),
  );
}

const on = (token = 'made-up-token') =>
  server((url, init) =>
    init?.method === 'POST'
      ? json({ token, expires_in: 600 })
      : json({ site_key: 'made-up-site-key' }),
  );

afterEach(() => {
  document.body.replaceChildren();
});

describe('HumanCheck', () => {
  it('needs no token, and asks only once, when the server has the check off', async () => {
    const fetch = server(() => json({ error: 'the human check is off' }, 404));
    const widget = turnstile();
    const human = new HumanCheck({ fetch, loadTurnstile: () => Promise.resolve(widget) });
    expect(await human.token()).toBeNull();
    expect(await human.token()).toBeNull();
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(widget.render).not.toHaveBeenCalled();
  });

  it('runs the challenge and trades the response for a token', async () => {
    const fetch = on();
    const widget = turnstile();
    const human = new HumanCheck({ fetch, loadTurnstile: () => Promise.resolve(widget) });
    expect(await human.token()).toBe('made-up-token');
    expect(widget.render.mock.calls[0]![1].sitekey).toBe('made-up-site-key');
    expect(widget.remove).toHaveBeenCalledWith('widget-1');
    expect(document.querySelector('.human-check')).toBeNull();
    const post = fetch.mock.calls[1]!;
    expect(post[0]).toBe('/api/v1/challenge');
    expect(JSON.parse(String(post[1]!.body))).toEqual({ response: 'made-up-response' });
  });

  it('reuses the token until shortly before it expires, and runs one challenge at a time', async () => {
    const fetch = on();
    let time = 0;
    const human = new HumanCheck({
      fetch,
      loadTurnstile: () => Promise.resolve(turnstile()),
      now: () => time,
    });
    const [a, b] = await Promise.all([human.token(), human.token()]);
    expect(a).toBe(b);
    expect(fetch).toHaveBeenCalledTimes(2);
    time = 569_000;
    await human.token();
    expect(fetch).toHaveBeenCalledTimes(2);
    time = 571_000;
    await human.token();
    expect(fetch).toHaveBeenCalledTimes(4);
  });

  it('has a cached token only after a check, until shortly before it expires', async () => {
    let time = 0;
    const human = new HumanCheck({
      fetch: on(),
      loadTurnstile: () => Promise.resolve(turnstile()),
      now: () => time,
    });
    expect(human.cached()).toBeNull();
    await human.token();
    expect(human.cached()).toBe('made-up-token');
    time = 571_000;
    expect(human.cached()).toBeNull();
  });

  it('asks for a new token after invalidate', async () => {
    const fetch = on();
    const human = new HumanCheck({ fetch, loadTurnstile: () => Promise.resolve(turnstile()) });
    await human.token();
    human.invalidate();
    await human.token();
    expect(fetch).toHaveBeenCalledTimes(4);
  });

  it('remembers when a check passed, so other pages can skip it', async () => {
    const storage = window.localStorage;
    storage.clear();
    let time = 0;
    const options = { loadTurnstile: () => Promise.resolve(turnstile()), now: () => time, storage };
    const first = new HumanCheck({ ...options, fetch: on() });
    expect(first.verified()).toBe(false);
    await first.token();
    expect(storage.getItem(HUMAN_UNTIL_KEY)).toBe('600000');
    // Another tab or page load has no token in memory, only the stored expiry.
    const second = new HumanCheck({ ...options, fetch: on() });
    expect(second.cached()).toBeNull();
    expect(second.verified()).toBe(true);
    time = 571_000;
    expect(second.verified()).toBe(false);
    first.invalidate();
    expect(storage.getItem(HUMAN_UNTIL_KEY)).toBeNull();
    storage.clear();
  });

  it('is not verified when storage is blocked', async () => {
    const storage = {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
      removeItem: () => {
        throw new Error('blocked');
      },
    };
    const human = new HumanCheck({
      fetch: on(),
      loadTurnstile: () => Promise.resolve(turnstile()),
      storage,
    });
    expect(human.verified()).toBe(false);
    expect(await human.token()).toBe('made-up-token');
    expect(human.verified()).toBe(true);
    expect(() => human.invalidate()).not.toThrow();
  });

  it('warms up in the background, once, and leaves a failure for the next request', async () => {
    const fetch = on();
    const widget = turnstile();
    const human = new HumanCheck({
      fetch,
      loadTurnstile: () => Promise.resolve(widget),
      storage: undefined,
    });
    human.warm();
    human.warm();
    expect(await human.token()).toBe('made-up-token');
    expect(widget.render).toHaveBeenCalledTimes(1);
    human.warm();
    expect(fetch).toHaveBeenCalledTimes(2);

    const failing = new HumanCheck({
      fetch: on(),
      loadTurnstile: () => Promise.resolve(turnstile('fail')),
      storage: undefined,
    });
    expect(() => failing.warm()).not.toThrow();
    await expect(failing.token()).rejects.toMatchObject({ kind: 'human-check' });
  });

  it('does not warm up when the server has the check off', async () => {
    const fetch = server(() => json({ error: 'off' }, 404));
    const human = new HumanCheck({ fetch, storage: undefined });
    await human.token();
    human.warm();
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('fails with a human-check error when the challenge or the server refuses', async () => {
    const failing = new HumanCheck({
      fetch: on(),
      loadTurnstile: () => Promise.resolve(turnstile('fail')),
    });
    await expect(failing.token()).rejects.toMatchObject({ kind: 'human-check' });
    expect(document.querySelector('.human-check')).toBeNull();

    const refused = new HumanCheck({
      fetch: server((_url, init) =>
        init?.method === 'POST'
          ? json({ error: 'the human check failed; try again' }, 403)
          : json({ site_key: 'made-up-site-key' }),
      ),
      loadTurnstile: () => Promise.resolve(turnstile()),
    });
    await expect(refused.token()).rejects.toMatchObject({ kind: 'human-check', status: 403 });
    // A later attempt tries again rather than staying failed.
    await expect(refused.token()).rejects.toBeInstanceOf(ApiError);
  });
});

describe('fetchRecommendations with a human check', () => {
  const recommendations = { total: 0, clubs: [] };
  const query = { source: { pace: 5000 } };
  const authorization = (fetch: ReturnType<typeof vi.fn<typeof globalThis.fetch>>, call: number) =>
    (fetch.mock.calls[call]![1]!.headers as Record<string, string>).Authorization;

  function check(cached: string | null, tokens: (string | null)[] = [], verified = true) {
    return {
      cached: () => cached,
      verified: () => verified,
      token: vi.fn(() => Promise.resolve(tokens.shift() ?? null)),
      invalidate: vi.fn(),
    };
  }

  it('tries without a token first and runs no check when the request passes', async () => {
    // As with a valid token cookie, or the check off.
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json(recommendations)));
    const human = check(null);
    await fetchRecommendations(query, { fetch, human });
    expect(authorization(fetch, 0)).toBeUndefined();
    expect(human.token).not.toHaveBeenCalled();
  });

  it('runs the check first, with no probe request, when none has passed lately', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json(recommendations)));
    const human = check(null, ['made-up-new'], false);
    await fetchRecommendations(query, { fetch, human });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(authorization(fetch, 0)).toBe('Bearer made-up-new');
    expect(human.invalidate).not.toHaveBeenCalled();
  });

  it('sends no token when the check turns out to be off', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json(recommendations)));
    const human = check(null, [null], false);
    await fetchRecommendations(query, { fetch, human });
    expect(authorization(fetch, 0)).toBeUndefined();
  });

  it('sends a cached token as a bearer header', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json(recommendations)));
    const human = check('made-up-token');
    await fetchRecommendations(query, { fetch, human });
    expect(authorization(fetch, 0)).toBe('Bearer made-up-token');
    expect(human.token).not.toHaveBeenCalled();
  });

  it('runs the check and retries once with its token after a 401', async () => {
    const human = check(null, ['made-up-new']);
    const fetch = vi.fn<typeof globalThis.fetch>((_url, init) =>
      Promise.resolve(
        (init!.headers as Record<string, string>).Authorization === 'Bearer made-up-new'
          ? json(recommendations)
          : json({ error: 'a human check token is required' }, 401),
      ),
    );
    await fetchRecommendations(query, { fetch, human });
    expect(human.invalidate).toHaveBeenCalledTimes(1);
    expect(human.token).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('gives up after a second 401', async () => {
    const human = check('made-up-token', ['made-up-token']);
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json({ error: 'no' }, 401)));
    await expect(fetchRecommendations(query, { fetch, human })).rejects.toMatchObject({
      status: 401,
    });
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('does not retry other errors', async () => {
    const human = check(null);
    const fetch = vi.fn<typeof globalThis.fetch>(() =>
      Promise.resolve(json({ error: 'busy' }, 503)),
    );
    await expect(fetchRecommendations(query, { fetch, human })).rejects.toMatchObject({
      status: 503,
    });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(human.invalidate).not.toHaveBeenCalled();
  });
});
