// Human check tests with a stubbed fetch and Turnstile. All keys and tokens are made up.

import { afterEach, describe, expect, it, vi } from 'vitest';

import { ApiError, fetchRecommendations } from './api';
import { HumanCheck, type Turnstile } from './human';

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

  it('asks for a new token after invalidate', async () => {
    const fetch = on();
    const human = new HumanCheck({ fetch, loadTurnstile: () => Promise.resolve(turnstile()) });
    await human.token();
    human.invalidate();
    await human.token();
    expect(fetch).toHaveBeenCalledTimes(4);
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

  it('sends the token as a bearer header', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json(recommendations)));
    const human = { token: () => Promise.resolve('made-up-token'), invalidate: vi.fn() };
    await fetchRecommendations(query, { fetch, human });
    expect(fetch.mock.calls[0]![1]!.headers).toMatchObject({
      Authorization: 'Bearer made-up-token',
    });
  });

  it('sends no Authorization header when the server needs none', async () => {
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json(recommendations)));
    const human = { token: () => Promise.resolve(null), invalidate: vi.fn() };
    await fetchRecommendations(query, { fetch, human });
    expect(fetch.mock.calls[0]![1]!.headers).not.toHaveProperty('Authorization');
  });

  it('gets a new token and retries once after a 401', async () => {
    const tokens = ['made-up-old', 'made-up-new'];
    const human = { token: vi.fn(() => Promise.resolve(tokens.shift()!)), invalidate: vi.fn() };
    const fetch = vi.fn<typeof globalThis.fetch>((_url, init) =>
      Promise.resolve(
        (init!.headers as Record<string, string>).Authorization === 'Bearer made-up-new'
          ? json(recommendations)
          : json({ error: 'a human check token is required' }, 401),
      ),
    );
    await fetchRecommendations(query, { fetch, human });
    expect(human.invalidate).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('gives up after a second 401', async () => {
    const human = { token: () => Promise.resolve('made-up-token'), invalidate: vi.fn() };
    const fetch = vi.fn<typeof globalThis.fetch>(() => Promise.resolve(json({ error: 'no' }, 401)));
    await expect(fetchRecommendations(query, { fetch, human })).rejects.toMatchObject({
      status: 401,
    });
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('does not retry other errors', async () => {
    const human = { token: () => Promise.resolve('made-up-token'), invalidate: vi.fn() };
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
