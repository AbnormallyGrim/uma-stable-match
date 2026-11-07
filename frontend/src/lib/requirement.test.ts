// Requirement tests. All numbers are made up.

import { afterEach, describe, expect, it } from 'vitest';

import {
  daysInMonth,
  parseRequirement,
  perDay,
  rememberedRequirement,
  rememberRequirement,
} from './requirement';

describe('requirement', () => {
  it('spreads a requirement over its interval', () => {
    expect(perDay({ fans: 8_000_000, per: 'day' }, '2026-11')).toBe(8_000_000);
    expect(perDay({ fans: 7_000_000, per: 'week' }, '2026-11')).toBe(1_000_000);
    expect(perDay({ fans: 60_000_000, per: 'month' }, '2026-11')).toBe(2_000_000);
    expect(perDay({ fans: 62_000_000, per: 'month' }, '2026-12')).toBe(2_000_000);
  });

  it('knows the days in a month, and assumes 30 without one', () => {
    expect(daysInMonth('2026-02')).toBe(28);
    expect(daysInMonth('2028-02')).toBe(29);
    expect(daysInMonth(null)).toBe(30);
    expect(daysInMonth('soon')).toBe(30);
  });

  it('reads an address, and drops anything unusable', () => {
    expect(parseRequirement('8000000', 'week')).toEqual({ fans: 8_000_000, per: 'week' });
    for (const [need, per] of [
      [null, 'week'],
      ['8000000', null],
      ['0', 'day'],
      ['-5', 'day'],
      ['8e6', 'day'],
      ['8000000', 'year'],
    ] as const) {
      expect(parseRequirement(need, per)).toBeNull();
    }
  });
});

describe('remembered requirement', () => {
  afterEach(() => localStorage.clear());

  it('is kept for each club and forgotten on request', () => {
    const requirement = { fans: 8_000_000, per: 'week' as const };
    rememberRequirement(localStorage, '7', requirement);
    expect(rememberedRequirement(localStorage, '7')).toEqual(requirement);
    expect(rememberedRequirement(localStorage, '8')).toBeNull();
    rememberRequirement(localStorage, '7', null);
    expect(rememberedRequirement(localStorage, '7')).toBeNull();
  });

  it('ignores anything unreadable, and works without storage', () => {
    localStorage.setItem('uma-stable-match:requirement:7', 'not json');
    expect(rememberedRequirement(localStorage, '7')).toBeNull();
    localStorage.setItem('uma-stable-match:requirement:7', '{"fans":-1,"per":"week"}');
    expect(rememberedRequirement(localStorage, '7')).toBeNull();
    expect(rememberedRequirement(undefined, '7')).toBeNull();
    expect(() => rememberRequirement(undefined, '7', null)).not.toThrow();
  });
});
