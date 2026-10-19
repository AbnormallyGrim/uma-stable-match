// SearchRunner tests. All data is made up.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ApiError, type RecommendationQuery, type Recommendations } from './api';
import {
  DEFAULT_RATE_LIMIT_WAIT,
  SearchRunner,
  type RunnerOptions,
  type SearchState,
} from './runner';
import { SCAN_STILL_LOADING } from './states';
import { recommendations } from './test-data';

const query: RecommendationQuery = { source: { pace: 7_600_000 } };

function setup(responses: (() => Promise<Recommendations>)[], delays = [2, 3]) {
  const states: SearchState[] = [];
  const fetch = vi.fn<RunnerOptions['fetch']>(() => {
    const next = responses.shift();
    if (!next) throw new Error('no response queued');
    return next();
  });
  const runner = new SearchRunner({ fetch, onchange: (s) => states.push(s), delays });
  const last = () => states[states.length - 1]!;
  return { runner, fetch, states, last };
}

const ok =
  (total = 1) =>
  () =>
    Promise.resolve(recommendations({ total }));
const fail =
  (status: number, retryAfter: number | null = null) =>
  () =>
    Promise.reject(new ApiError('http', `status ${status}`, status, retryAfter));

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('SearchRunner', () => {
  it('loads then shows the result', async () => {
    const { runner, states, last } = setup([ok(3)]);
    runner.run(query);
    expect(states[0]?.status).toEqual({ phase: 'loading' });
    await vi.runAllTimersAsync();
    expect(last().status).toEqual({ phase: 'done' });
    expect(last().result?.total).toBe(3);
  });

  it('retries a 503 after counting down, then succeeds', async () => {
    const { runner, fetch, last } = setup([fail(503), ok()]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    expect(last().status).toEqual({ phase: 'waiting-for-scan', retryIn: 2 });

    await vi.advanceTimersByTimeAsync(1000);
    expect(last().status).toEqual({ phase: 'waiting-for-scan', retryIn: 1 });
    expect(fetch).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1000);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(last().status).toEqual({ phase: 'done' });
  });

  it('uses longer delays for later 503s and gives up when they run out', async () => {
    const { runner, fetch, last } = setup([fail(503), fail(503), fail(503)]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(2000);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(last().status).toEqual({ phase: 'waiting-for-scan', retryIn: 3 });
    await vi.advanceTimersByTimeAsync(3000);
    expect(fetch).toHaveBeenCalledTimes(3);
    expect(last().status).toEqual({ phase: 'error', message: SCAN_STILL_LOADING });
  });

  it('retries a 503 straight away with retryNow', async () => {
    const { runner, fetch, last } = setup([fail(503), ok()]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    runner.retryNow();
    await vi.advanceTimersByTimeAsync(0);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(last().status).toEqual({ phase: 'done' });
    await vi.advanceTimersByTimeAsync(10_000);
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('counts down a 429 without retrying on its own', async () => {
    const { runner, fetch, last } = setup([fail(429, 2), ok()]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    expect(last().status).toEqual({ phase: 'rate-limited', retryIn: 2 });

    runner.retryNow();
    expect(fetch).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(2000);
    expect(last().status).toEqual({ phase: 'rate-limited', retryIn: 0 });
    expect(fetch).toHaveBeenCalledTimes(1);

    runner.retryNow();
    await vi.advanceTimersByTimeAsync(0);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(last().status).toEqual({ phase: 'done' });
  });

  it('waits a default time when a 429 has no Retry-After', async () => {
    const { runner, last } = setup([fail(429)]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    expect(last().status).toEqual({ phase: 'rate-limited', retryIn: DEFAULT_RATE_LIMIT_WAIT });
  });

  it('describes other errors', async () => {
    const { runner, last } = setup([fail(404)]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    const status = last().status;
    expect(status.phase).toBe('error');
    if (status.phase === 'error') {
      expect(status.message.title).toBe("We couldn't find that trainer");
    }
  });

  it('keeps the previous result while a new search loads', async () => {
    const { runner, states } = setup([ok(3), ok(5)]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    runner.run(query);
    const loading = states[states.length - 1]!;
    expect(loading.status).toEqual({ phase: 'loading' });
    expect(loading.result?.total).toBe(3);
  });

  it('cancels a pending retry and the old request when a new search starts', async () => {
    let resolveOld: (value: Recommendations) => void = () => undefined;
    const slow = () => new Promise<Recommendations>((resolve) => (resolveOld = resolve));
    const { runner, fetch, last } = setup([fail(503), slow, ok(7)]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    runner.retryNow();
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    resolveOld(recommendations({ total: 99 }));
    await vi.advanceTimersByTimeAsync(10_000);
    expect(fetch).toHaveBeenCalledTimes(3);
    expect(last().result?.total).toBe(7);
  });

  it('stops counting down when disposed', async () => {
    const { runner, fetch } = setup([fail(503), ok()]);
    runner.run(query);
    await vi.advanceTimersByTimeAsync(0);
    runner.dispose();
    await vi.advanceTimersByTimeAsync(10_000);
    expect(fetch).toHaveBeenCalledTimes(1);
  });
});
