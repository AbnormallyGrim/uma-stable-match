import { describe, expect, it } from 'vitest';

import { PROGRESS_CAP, PROGRESS_TAU_MS, progressAt } from './progress';

describe('progressAt', () => {
  it('starts at zero, and stays there for a time that is not positive or not a number', () => {
    expect(progressAt(0)).toBe(0);
    expect(progressAt(-500)).toBe(0);
    expect(progressAt(Number.NaN)).toBe(0);
  });

  it('is about 63% of the way to the cap after one time constant', () => {
    expect(progressAt(PROGRESS_TAU_MS)).toBeCloseTo(PROGRESS_CAP * (1 - 1 / Math.E), 10);
  });

  it('only ever increases, and never reaches the cap', () => {
    let last = 0;
    for (const ms of [100, 500, 1000, 4000, 10_000, 60_000, 3_600_000]) {
      const now = progressAt(ms);
      expect(now).toBeGreaterThan(last);
      expect(now).toBeLessThanOrEqual(PROGRESS_CAP);
      last = now;
    }
    expect(progressAt(60_000)).toBeLessThan(PROGRESS_CAP);
  });

  it('takes a custom time constant', () => {
    expect(progressAt(1000, 1000)).toBeCloseTo(progressAt(PROGRESS_TAU_MS), 10);
    expect(progressAt(1000, 500)).toBeGreaterThan(progressAt(1000, 2000));
  });
});
