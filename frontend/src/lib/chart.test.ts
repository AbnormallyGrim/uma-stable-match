// Chart number tests. All data is made up.

import { describe, expect, it } from 'vitest';

import { lastDay, medianSeries, tickLabel, ticks, toSeries } from './chart';

describe('toSeries', () => {
  const fans = [100, 110, 0, 130, 160, 0, 0];

  it('gives daily gains, averaging over days without data', () => {
    expect(toSeries(fans, 'daily')).toEqual([null, 10, null, 10, 30, null, null]);
  });

  it('gives fans gained since the first day with data', () => {
    expect(toSeries(fans, 'month')).toEqual([0, 10, null, 30, 60, null, null]);
    expect(toSeries([0, 0, 50, 80], 'month')).toEqual([null, null, 0, 30]);
  });

  it('never shows a negative gain', () => {
    expect(toSeries([100, 90], 'daily')).toEqual([null, 0]);
  });
});

describe('medianSeries', () => {
  it('takes each day’s median where enough members have a value', () => {
    const series = [
      [1, 4, null],
      [2, 6, 5],
      [3, 8, 7],
      [null, 10, null],
    ];
    expect(medianSeries(series, 4)).toEqual([2, 7, null, null]);
  });
});

describe('lastDay', () => {
  it('finds the last day any series has a value', () => {
    expect(
      lastDay([
        [1, null, null],
        [null, 2, null],
      ]),
    ).toBe(1);
    expect(lastDay([[null], []])).toBe(-1);
  });
});

describe('ticks', () => {
  it('rounds to clean steps from zero', () => {
    expect(ticks(9_300_000)).toEqual([0, 2_500_000, 5_000_000, 7_500_000, 10_000_000]);
    expect(ticks(180_000_000)).toEqual([0, 50_000_000, 100_000_000, 150_000_000, 200_000_000]);
    expect(ticks(0)).toEqual([0]);
  });

  it('labels ticks compactly', () => {
    expect([0, 500, 2500, 500_000, 2_000_000, 2_500_000].map(tickLabel)).toEqual([
      '0',
      '500',
      '2.5K',
      '500K',
      '2M',
      '2.5M',
    ]);
  });
});
