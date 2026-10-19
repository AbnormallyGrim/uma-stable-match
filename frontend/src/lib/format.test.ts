// Display helper tests.

import { describe, expect, it } from 'vitest';

import {
  formatMillions,
  formatPercent,
  formatPerDay,
  formatPlace,
  formatScore,
  formatUpdated,
  joinStyleLabel,
  playstyleLabel,
  rankTrend,
  tierIndex,
} from './format';

describe('numbers', () => {
  it('formats fans in millions', () => {
    expect(formatMillions(6_400_000)).toBe('6.4M');
    expect(formatMillions(12_345_678)).toBe('12.3M');
    expect(formatMillions(850_000)).toBe('0.85M');
    expect(formatMillions(5_000)).toBe('<0.01M');
    expect(formatMillions(0)).toBe('0M');
  });

  it('formats a daily pace', () => {
    expect(formatPerDay(7_620_000)).toBe('7.6M/day');
  });

  it('formats places, percentages and scores', () => {
    expect(formatPlace(8, 30)).toBe('8 of 30');
    expect(formatPercent(0.344)).toBe('34%');
    expect(formatPercent(1)).toBe('100%');
    expect(formatScore(0.10449)).toBe('0.104');
  });
});

describe('rankTrend', () => {
  it('shows climbing clubs', () => {
    expect(rankTrend(111, 127)).toEqual({
      kind: 'up',
      change: 16,
      label: '▲16',
      description: 'Up 16 places since last month',
    });
    expect(rankTrend(10, 11).description).toBe('Up 1 place since last month');
  });

  it('shows falling clubs', () => {
    expect(rankTrend(130, 126)).toEqual({
      kind: 'down',
      change: -4,
      label: '▼4',
      description: 'Down 4 places since last month',
    });
    expect(rankTrend(12, 11).description).toBe('Down 1 place since last month');
  });

  it('handles same, new and unknown ranks', () => {
    expect(rankTrend(50, 50)).toMatchObject({ kind: 'same', label: '–' });
    expect(rankTrend(50, null)).toMatchObject({ kind: 'new', label: 'new' });
    expect(rankTrend(null, 50)).toMatchObject({ kind: 'unknown', label: '' });
  });
});

describe('labels', () => {
  it('labels join styles', () => {
    expect(joinStyleLabel('open')).toBe('Open');
    expect(joinStyleLabel('approval')).toBe('Approval');
    expect(joinStyleLabel('closed')).toBe('Closed');
    expect(joinStyleLabel(null)).toBeNull();
  });

  it('labels playstyles by policy code', () => {
    expect(playstyleLabel(2)).toBe('Laid-back');
    expect(playstyleLabel(12)).toBe('Log in Daily');
    expect(playstyleLabel(17)).toBe('Active at Night');
    expect(playstyleLabel(99)).toBeNull();
    expect(playstyleLabel(null)).toBeNull();
  });
});

describe('formatUpdated', () => {
  const now = new Date('2026-03-18T12:00:00Z');

  it('describes how long ago data was updated', () => {
    expect(formatUpdated('2026-03-18T11:59:30Z', now)).toBe('just now');
    expect(formatUpdated('2026-03-18T11:55:00Z', now)).toBe('5 minutes ago');
    expect(formatUpdated('2026-03-18T11:00:00Z', now)).toBe('1 hour ago');
    expect(formatUpdated('2026-03-18T09:00:00Z', now)).toBe('3 hours ago');
    expect(formatUpdated('2026-03-17T12:00:00Z', now)).toBe('yesterday');
    expect(formatUpdated('2026-03-14T12:00:00Z', now)).toBe('4 days ago');
  });

  it('treats future times as just now and bad input as unknown', () => {
    expect(formatUpdated('2026-03-18T12:05:00Z', now)).toBe('just now');
    expect(formatUpdated('not a date', now)).toBe('unknown');
  });
});

describe('tierIndex', () => {
  it('orders tiers from D to SS', () => {
    expect(tierIndex('D')).toBe(0);
    expect(tierIndex('A')).toBe(6);
    expect(tierIndex('SS')).toBe(10);
    expect(tierIndex('Z')).toBe(-1);
    expect(tierIndex(null)).toBe(-1);
  });
});
