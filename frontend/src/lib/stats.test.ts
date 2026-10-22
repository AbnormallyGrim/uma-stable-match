// Results summary statistics tests. All data is made up.

import { describe, expect, it } from 'vitest';

import { comfortableFits, formatTopShare, typicalPlace } from './stats';
import { club } from './test-data';

const at = (your_place: number, club_size: number) => club({ your_place, club_size });

describe('comfortableFits', () => {
  it('counts clubs where you would sit 25% to 60% of the way down', () => {
    const clubs = [at(5, 20), at(12, 20), at(13, 20), at(4, 20), at(1, 20)];
    expect(comfortableFits(clubs)).toBe(2);
  });

  it('is zero without clubs', () => {
    expect(comfortableFits([])).toBe(0);
  });
});

describe('typicalPlace', () => {
  it('is the middle place share across the clubs', () => {
    expect(typicalPlace([at(2, 20), at(10, 20), at(18, 20)])).toBe(0.5);
    expect(typicalPlace([at(4, 20), at(8, 20)])).toBeCloseTo(0.3);
  });

  it('is null without clubs', () => {
    expect(typicalPlace([])).toBeNull();
  });
});

describe('formatTopShare', () => {
  it('shows a share as top N percent', () => {
    expect(formatTopShare(0.456)).toBe('Top 46%');
    expect(formatTopShare(0.001)).toBe('Top 1%');
    expect(formatTopShare(null)).toBe('–');
  });
});
