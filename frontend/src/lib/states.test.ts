// Search message tests. All data is made up.

import { describe, expect, it } from 'vitest';

import { ApiError } from './api';
import { describeError, emptyHints, monthNotice, pendingNotice, resultNotices } from './states';
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
    expect(describeError(http(500, 'internal error'))).toEqual({
      title: 'Something went wrong',
      detail: 'internal error',
      canRetry: true,
    });
    expect(describeError(new Error('boom'))).toMatchObject({ canRetry: true });
  });
});

describe('emptyHints', () => {
  const options = (balance_weight: number | null, include_closed: boolean) => ({
    balance_weight,
    include_closed,
    include_full: false,
    limit: 100,
  });

  it('suggests what could widen the search', () => {
    expect(emptyHints(options(0.5, false))).toHaveLength(2);
    expect(emptyHints(options(null, false))).toEqual([
      "Include closed clubs to see clubs that aren't recruiting right now.",
    ]);
    expect(emptyHints(options(0.5, true))).toEqual([
      'Lower the balance strength or turn balance scoring off.',
    ]);
    expect(emptyHints(options(0, true))).toEqual([]);
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
    const at = '2026-03-02T15:00:00Z';
    expect(monthNotice({ updated_at: at, month: '2026-03' })).toBeNull();
    expect(monthNotice({ updated_at: at, month: null })).toBeNull();
    expect(monthNotice({ updated_at: at, month: '2026-02' })).toBe(
      "These results are based on last month's data (February 2026), because the new game month has only just started.",
    );
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
