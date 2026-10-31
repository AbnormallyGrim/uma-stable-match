// Client tests with a stubbed fetch. All data is made up.

import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  ApiError,
  DEFAULT_TIMEOUT_MS,
  fetchClub,
  fetchFeatures,
  fetchRecommendations,
  recommendationParams,
  type Recommendations,
} from './api';

const recommendations: Recommendations = {
  trainer: { trainer_id: 123456789012, pace_per_day: 7_600_000 },
  options: {
    balance_weight: null,
    include_closed: false,
    include_full: false,
    join_styles: null,
    playstyles: null,
    has_discord: false,
    limit: 50,
  },
  updated_at: '2026-03-18T09:00:00Z',
  month: '2026-03',
  coverage: { considered: 340, shortlisted: 1, scored: 1, pending: 0 },
  total: 1,
  clubs: [
    {
      circle_id: 1,
      name: 'Example Club',
      url: 'https://uma.moe/circles/1',
      tier: 'A+',
      join_style: 'open',
      policy: 2,
      member_count: 25,
      open_spots: 5,
      monthly_rank: 120,
      last_month_rank: 150,
      comment: null,
      has_discord: false,
      your_place: 9,
      club_size: 26,
      ranked_members: 24,
      median_per_day: 6_400_000,
      fit: 0.0562,
      score: 0.0562,
      balance: null,
    },
  ],
  your_club: null,
};

function json(body: unknown, init: ResponseInit = {}): Response {
  return new Response(JSON.stringify(body), {
    ...init,
    headers: { 'Content-Type': 'application/json', ...init.headers },
  });
}

function fetchReturning(response: Response | Promise<Response>) {
  return vi.fn<typeof fetch>(() => Promise.resolve(response));
}

async function rejection(promise: Promise<unknown>): Promise<ApiError> {
  try {
    await promise;
  } catch (error) {
    expect(error).toBeInstanceOf(ApiError);
    return error as ApiError;
  }
  throw new Error('expected the request to fail');
}

afterEach(() => {
  vi.useRealTimers();
});

describe('recommendationParams', () => {
  it('uses trainer_id for a trainer', () => {
    expect(recommendationParams({ source: { trainerId: '123456789012' } }).toString()).toBe(
      'trainer_id=123456789012',
    );
  });

  it('uses pace and every option', () => {
    const params = recommendationParams({
      source: { pace: 7_600_000 },
      balanceWeight: 0.5,
      joinStyle: 'closed',
      playstyle: 12,
      includeFull: true,
      limit: 10,
    });
    expect(params.toString()).toBe(
      'pace=7600000&balance_weight=0.5&join_style=closed&playstyle=12&include_full=true&limit=10',
    );
  });

  it('leaves out unset and default options', () => {
    const params = recommendationParams({
      source: { pace: 5_000_000 },
      balanceWeight: null,
      joinStyle: null,
      playstyle: null,
    });
    expect(params.toString()).toBe('pace=5000000');
  });

  it('keeps a zero balance weight', () => {
    const params = recommendationParams({ source: { pace: 1 }, balanceWeight: 0 });
    expect(params.get('balance_weight')).toBe('0');
  });
});

describe('fetchRecommendations', () => {
  it('requests and parses recommendations', async () => {
    const fetch = fetchReturning(json(recommendations));
    const result = await fetchRecommendations(
      { source: { trainerId: '123456789012' }, balanceWeight: 0.5 },
      { fetch },
    );
    expect(result).toEqual(recommendations);
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/recommendations?trainer_id=123456789012&balance_weight=0.5',
      expect.objectContaining({ headers: { Accept: 'application/json' } }),
    );
  });

  it('uses the server error message', async () => {
    const fetch = fetchReturning(json({ error: 'trainer not found' }, { status: 404 }));
    const error = await rejection(fetchRecommendations({ source: { trainerId: '1' } }, { fetch }));
    expect(error.kind).toBe('http');
    expect(error.status).toBe(404);
    expect(error.message).toBe('trainer not found');
    expect(error.retryAfter).toBeNull();
  });

  it('reads Retry-After when rate limited', async () => {
    const fetch = fetchReturning(
      json(
        { error: 'too many requests; try again in 20 seconds' },
        { status: 429, headers: { 'Retry-After': '20' } },
      ),
    );
    const error = await rejection(fetchRecommendations({ source: { pace: 1 } }, { fetch }));
    expect(error.status).toBe(429);
    expect(error.retryAfter).toBe(20);
  });

  it('ignores a Retry-After that is not whole seconds', async () => {
    const fetch = fetchReturning(
      json({ error: 'busy' }, { status: 503, headers: { 'Retry-After': 'soon' } }),
    );
    const error = await rejection(fetchRecommendations({ source: { pace: 1 } }, { fetch }));
    expect(error.retryAfter).toBeNull();
  });

  it('falls back to a generic message without a JSON error', async () => {
    const fetch = fetchReturning(new Response('Bad Gateway', { status: 502 }));
    const error = await rejection(fetchRecommendations({ source: { pace: 1 } }, { fetch }));
    expect(error.status).toBe(502);
    expect(error.message).toBe('Request failed with status 502.');
  });

  it('reports invalid JSON', async () => {
    const fetch = fetchReturning(new Response('not json', { status: 200 }));
    const error = await rejection(fetchRecommendations({ source: { pace: 1 } }, { fetch }));
    expect(error.kind).toBe('invalid-response');
  });

  it('reports network failures', async () => {
    const fetchFn = vi.fn<typeof fetch>(() => Promise.reject(new TypeError('Failed to fetch')));
    const error = await rejection(
      fetchRecommendations({ source: { pace: 1 } }, { fetch: fetchFn }),
    );
    expect(error.kind).toBe('network');
    expect(error.status).toBeNull();
  });
});

function hangingFetch() {
  return vi.fn<typeof fetch>(
    (_input, init) =>
      new Promise((_resolve, reject) => {
        const abort = () => reject(new DOMException('aborted', 'AbortError'));
        if (init?.signal?.aborted) {
          abort();
        }
        init?.signal?.addEventListener('abort', abort);
      }),
  );
}

describe('timeouts and cancellation', () => {
  it('times out after the default timeout', async () => {
    vi.useFakeTimers();
    const pending = rejection(
      fetchRecommendations({ source: { pace: 5_000_000 } }, { fetch: hangingFetch() }),
    );
    await vi.advanceTimersByTimeAsync(DEFAULT_TIMEOUT_MS);
    const error = await pending;
    expect(error.kind).toBe('timeout');
  });

  it('honours a custom timeout', async () => {
    vi.useFakeTimers();
    const pending = rejection(
      fetchRecommendations(
        { source: { pace: 5_000_000 } },
        { fetch: hangingFetch(), timeoutMs: 100 },
      ),
    );
    await vi.advanceTimersByTimeAsync(100);
    expect((await pending).kind).toBe('timeout');
  });

  it('passes a caller abort through unchanged', async () => {
    const controller = new AbortController();
    const pending = fetchRecommendations(
      { source: { pace: 5_000_000 } },
      { fetch: hangingFetch(), signal: controller.signal },
    );
    controller.abort();
    await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
  });

  it('does not start when already aborted', async () => {
    const controller = new AbortController();
    controller.abort();
    const fetchFn = hangingFetch();
    await expect(
      fetchRecommendations(
        { source: { pace: 5_000_000 } },
        { fetch: fetchFn, signal: controller.signal },
      ),
    ).rejects.toMatchObject({
      name: 'AbortError',
    });
    expect(fetchFn).not.toHaveBeenCalled();
  });
});

describe('fetchFeatures', () => {
  const reply = (body: unknown, status = 200) =>
    vi.fn<typeof fetch>(async () => new Response(JSON.stringify(body), { status }));

  it('reads the switched-on features', async () => {
    const fetch = reply({ japanese: true, web_analytics: 'madeuptoken123' });
    await expect(fetchFeatures({ fetch })).resolves.toEqual({
      japanese: true,
      webAnalytics: 'madeuptoken123',
    });
    expect(fetch.mock.calls[0]?.[0]).toBe('/api/v1/features');
  });

  it('counts anything that is not exactly true as off', async () => {
    const off = { japanese: false, webAnalytics: null };
    await expect(fetchFeatures({ fetch: reply({ japanese: 'yes' }) })).resolves.toEqual(off);
    await expect(fetchFeatures({ fetch: reply({ web_analytics: 1 }) })).resolves.toEqual(off);
    await expect(fetchFeatures({ fetch: reply({ web_analytics: '' }) })).resolves.toEqual(off);
    await expect(fetchFeatures({ fetch: reply({}) })).resolves.toEqual(off);
    await expect(fetchFeatures({ fetch: reply(null) })).resolves.toEqual(off);
  });

  it('fails like any other request', async () => {
    await expect(fetchFeatures({ fetch: reply({ error: 'nope' }, 500) })).rejects.toMatchObject({
      kind: 'http',
      status: 500,
    });
  });
});

describe('fetchClub', () => {
  const details = {
    club: recommendations.clubs[0],
    month: '2026-03',
    updated_at: '2026-03-19T15:00:00Z',
    median_per_day: 6_400_000,
    members: [{ pace_per_day: 6_400_000, daily_fans: [100_000_000, 106_400_000] }],
    trainer: null,
  };

  it('requests a club alone', async () => {
    const fetch = fetchReturning(json(details));
    expect(await fetchClub('1', null, { fetch })).toEqual(details);
    expect(fetch).toHaveBeenCalledWith('/api/v1/clubs/1', expect.anything());
  });

  it('places a trainer', async () => {
    const fetch = fetchReturning(json(details));
    await fetchClub('1', '123456789012', { fetch });
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/clubs/1?trainer_id=123456789012',
      expect.anything(),
    );
  });

  it('runs the check and retries once after a 401', async () => {
    const human = {
      cached: () => 'made-up-old',
      token: vi.fn(() => Promise.resolve('made-up-new')),
      invalidate: vi.fn(),
    };
    const fetch = vi.fn<typeof globalThis.fetch>((_url, init) =>
      Promise.resolve(
        (init!.headers as Record<string, string>).Authorization === 'Bearer made-up-new'
          ? json(details)
          : json({ error: 'a human check token is required' }, { status: 401 }),
      ),
    );
    expect(await fetchClub('1', null, { fetch, human })).toEqual(details);
    expect(human.invalidate).toHaveBeenCalledOnce();
  });

  it('uses the server error message', async () => {
    const fetch = fetchReturning(
      json({ error: 'club not loaded yet; try again later' }, { status: 404 }),
    );
    const error = await rejection(fetchClub('1', null, { fetch }));
    expect(error.status).toBe(404);
    expect(error.message).toBe('club not loaded yet; try again later');
  });
});
