import { describe, expect, it } from 'vitest';

import { ApiError } from './api';
import { describeError, emptyHints } from './states';

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
