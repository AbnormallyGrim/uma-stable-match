import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { Status } from './api';
import DataStatus, { FAST_POLL_MS, POLL_MS } from './DataStatus.svelte';

const NOW = new Date('2026-03-18T12:00:00Z');

const ready = (finished_at = '2026-03-18T09:00:00Z'): Status => ({
  scanning: false,
  last_scan: { finished_at, clubs: 80, candidates: 200, failed: 0 },
  last_error: null,
});

function setup(responses: (Status | Error)[], onscanready = vi.fn()) {
  const load = vi.fn(() => {
    const next = responses.length > 1 ? responses.shift()! : responses[0]!;
    return next instanceof Error ? Promise.reject(next) : Promise.resolve(next);
  });
  render(DataStatus, { load, onscanready });
  return { load, onscanready };
}

const text = () => screen.queryByTestId('data-status')?.textContent?.replace(/\s+/g, ' ').trim();

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
});

describe('DataStatus', () => {
  it('shows when club data was last updated', async () => {
    setup([ready()]);
    await vi.advanceTimersByTimeAsync(0);
    expect(text()).toBe('Club data updated 3 hours ago.');
    expect(screen.getByText('3 hours ago')).toHaveAttribute('datetime', '2026-03-18T09:00:00Z');
  });

  it('shows the first scan and a refresh in progress', async () => {
    setup([{ scanning: true, last_scan: null, last_error: null }]);
    await vi.advanceTimersByTimeAsync(0);
    expect(text()).toBe('Loading club data for the first time…');
  });

  it('shows a refresh running alongside the last update', async () => {
    setup([{ ...ready(), scanning: true }]);
    await vi.advanceTimersByTimeAsync(0);
    expect(text()).toBe('Club data updated 3 hours ago; refreshing now…');
  });

  it('shows a failed refresh', async () => {
    setup([
      {
        ...ready(),
        last_error: {
          at: '2026-03-18T11:00:00Z',
          message: 'listing clubs failed: could not reach uma.moe',
        },
      },
    ]);
    await vi.advanceTimersByTimeAsync(0);
    expect(text()).toContain(
      'The last refresh failed: listing clubs failed: could not reach uma.moe.',
    );
    expect(screen.getByTestId('data-status')).toHaveClass('warn');
  });

  it('shows nothing until the status has loaded, or if it fails', async () => {
    setup([new Error('offline')]);
    await vi.advanceTimersByTimeAsync(0);
    expect(screen.queryByTestId('data-status')).not.toBeInTheDocument();
  });

  it('polls quickly while scanning and slowly once data is ready', async () => {
    const { load } = setup([{ scanning: true, last_scan: null, last_error: null }, ready()]);
    await vi.advanceTimersByTimeAsync(0);
    expect(load).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(FAST_POLL_MS);
    expect(load).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(FAST_POLL_MS);
    expect(load).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(POLL_MS);
    expect(load).toHaveBeenCalledTimes(3);
  });

  it('reports each newly finished scan', async () => {
    const { onscanready } = setup([
      { scanning: true, last_scan: null, last_error: null },
      ready('2026-03-18T11:59:00Z'),
      ready('2026-03-18T11:59:00Z'),
      ready('2026-03-18T12:01:00Z'),
    ]);
    await vi.advanceTimersByTimeAsync(0);
    expect(onscanready).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(FAST_POLL_MS);
    expect(onscanready).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(POLL_MS);
    expect(onscanready).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(POLL_MS);
    expect(onscanready).toHaveBeenCalledTimes(2);
  });

  it('skips polling while the tab is hidden', async () => {
    const hidden = vi.spyOn(document, 'hidden', 'get').mockReturnValue(true);
    const { load } = setup([ready()]);
    await vi.advanceTimersByTimeAsync(POLL_MS * 3);
    expect(load).not.toHaveBeenCalled();

    hidden.mockReturnValue(false);
    document.dispatchEvent(new Event('visibilitychange'));
    await vi.advanceTimersByTimeAsync(0);
    expect(load).toHaveBeenCalledTimes(1);
    hidden.mockRestore();
  });
});
