// Search message tests. All data is made up.

import { describe, expect, it } from 'vitest';

import { ApiError, type JoinStyle } from './api';
import {
  describeError,
  emptyHints,
  filterNotice,
  monthNotice,
  pendingNotice,
  resultNotices,
} from './states';
import { recommendations } from './test-data';

const http = (status: number, message = 'server message') =>
  new ApiError('http', message, status, null);

describe('describeError', () => {
  it('explains trainer lookup problems without offering a retry', () => {
    expect(describeError(http(404))).toMatchObject({
      title: "We couldn't find that trainer",
      canRetry: false,
    });
    expect(describeError(http(422))).toMatchObject({
      title: 'Not enough fan data yet',
      canRetry: false,
    });
    expect(describeError(http(400, 'pace must be a number'))).toEqual({
      title: "That search didn't work",
      detail: 'pace must be a number',
      canRetry: false,
    });
  });

  it('offers a retry for temporary problems', () => {
    expect(describeError(http(502))).toMatchObject({
      title: "Couldn't reach uma.moe",
      canRetry: true,
    });
    expect(describeError(new ApiError('network', 'x'))).toMatchObject({
      title: "Couldn't reach the server",
      canRetry: true,
    });
    expect(describeError(new ApiError('timeout', 'x'))).toMatchObject({
      title: 'The server took too long to answer',
      canRetry: true,
    });
    expect(describeError(new ApiError('invalid-response', 'x'))).toMatchObject({
      canRetry: true,
    });
    expect(describeError(new ApiError('human-check', 'The check failed. Try again.'))).toEqual({
      title: "We couldn't confirm you're a person",
      detail: 'The check failed. Try again.',
      canRetry: true,
    });
    expect(describeError(http(500, 'internal error'))).toEqual({
      title: 'Something went wrong',
      detail: 'internal error',
      canRetry: true,
    });
    expect(describeError(new Error('boom'))).toMatchObject({ canRetry: true });
  });
});

describe('emptyHints', () => {
  const options = (
    balance_weight: number | null,
    include_closed: boolean,
    playstyles: number[] | null = null,
  ) => ({
    balance_weight,
    include_closed,
    include_full: false,
    join_styles: null,
    playstyles,
    has_discord: false,
    limit: 100,
  });

  it('suggests turning off Has Discord', () => {
    expect(emptyHints({ ...options(null, true), has_discord: true })).toEqual([
      'Turn off Has Discord.',
    ]);
  });

  it('suggests what could widen the search', () => {
    expect(emptyHints(options(0.5, false))).toHaveLength(2);
    expect(emptyHints(options(null, false))).toEqual([
      "Pick Closed under join style to see clubs that aren't recruiting right now.",
    ]);
    expect(emptyHints(options(null, true, [2]))).toEqual(['Set playstyle back to any.']);
    expect(emptyHints(options(0.5, true))).toEqual([
      'Lower the balance strength or turn balance scoring off.',
    ]);
    expect(emptyHints(options(0, true))).toEqual([]);
  });
});

describe('filter notice', () => {
  const options = (join_styles: JoinStyle[] | null, playstyles: number[] | null) => ({
    balance_weight: null,
    include_closed: false,
    include_full: false,
    join_styles,
    playstyles,
    has_discord: false,
    limit: 100,
  });

  it('warns only when filters narrow the search', () => {
    expect(filterNotice(options(null, null))).toBeNull();
    expect(filterNotice(options(['open', 'approval', 'closed'], null))).toBeNull();
    expect(filterNotice(options(['open'], null))).toMatch(/filtered by join style or playstyle/);
    expect(filterNotice(options(null, [12]))).toMatch(/further from your pace/);
  });
});

describe('result notices', () => {
  it('count clubs still loading', () => {
    expect(pendingNotice({ considered: 50, shortlisted: 5, scored: 5, pending: 0 })).toBeNull();
    expect(pendingNotice({ considered: 50, shortlisted: 5, scored: 4, pending: 1 })).toBe(
      '1 more club is still loading. Search again in a minute for complete results.',
    );
    expect(pendingNotice({ considered: 50, shortlisted: 5, scored: 2, pending: 3 })).toBe(
      '3 more clubs are still loading. Search again in a minute for complete results.',
    );
  });

  it("mention last month's data only when it's used", () => {
    const at = '2026-03-19T15:00:00Z';
    expect(monthNotice({ updated_at: at, month: '2026-03' })).toBeNull();
    expect(monthNotice({ updated_at: at, month: null })).toBeNull();
    expect(monthNotice({ updated_at: '2026-03-02T15:00:00Z', month: '2026-02' })).toBe(
      "These results are based on last month's data (February 2026), because the new game month has only just started.",
    );
  });

  it("mention last month's blended data in the first days of a month", () => {
    const blended =
      "The game month has only just started, so these results combine this month's data with last month's. Last month's counts for less each day.";
    expect(monthNotice({ updated_at: '2026-03-01T15:00:00Z', month: '2026-03' })).toBe(blended);
    expect(monthNotice({ updated_at: '2026-03-05T15:00:00Z', month: '2026-03' })).toBe(blended);
    expect(monthNotice({ updated_at: '2026-03-06T15:00:00Z', month: '2026-03' })).toBeNull();
  });

  it('put the month first', () => {
    const notices = resultNotices(
      recommendations({
        updated_at: '2026-03-02T15:00:00Z',
        month: '2026-02',
        coverage: { considered: 340, shortlisted: 3, scored: 1, pending: 2 },
      }),
    );
    expect(notices).toHaveLength(2);
    expect(notices[0]).toContain("last month's data");
    expect(notices[1]).toContain('2 more clubs');
  });
});
