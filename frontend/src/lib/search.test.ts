// Search helper tests. All data is made up.

import { describe, expect, it } from 'vitest';

import type { JoinStyle } from './api';
import {
  DEFAULT_SEARCH,
  fromUrlParams,
  isTrainerId,
  loadRememberedTrainerId,
  parsePaceMillions,
  saveRememberedTrainerId,
  toUrlParams,
  validate,
  type SearchState,
} from './search';

const trainer = (trainerId: string, extra: Partial<SearchState> = {}): SearchState => ({
  ...DEFAULT_SEARCH,
  trainerId,
  ...extra,
});

const pace = (paceMillions: string, extra: Partial<SearchState> = {}): SearchState => ({
  ...DEFAULT_SEARCH,
  mode: 'pace',
  paceMillions,
  ...extra,
});

const TRAINER_ID_ERROR = 'Enter your trainer ID: 9 to 12 digits.';
const PACE_ERROR = 'Enter your daily fan gain in millions, such as 7.6.';

describe('trainer IDs', () => {
  it('accepts 9 to 12 digits, ignoring spaces and dashes', () => {
    for (const id of ['123456789', '123456789012', '1234 5678 9012', '1234-5678-9012']) {
      expect(isTrainerId(id), id).toBe(true);
    }
  });

  it('rejects anything else', () => {
    for (const id of ['', '12345678', '1234567890123', '12345678901a', '12345678.9']) {
      expect(isTrainerId(id), id).toBe(false);
    }
  });
});

describe('parsePaceMillions', () => {
  it('converts millions to fans per day', () => {
    expect(parsePaceMillions('7.6')).toBe(7_600_000);
    expect(parsePaceMillions(' 12 ')).toBe(12_000_000);
    expect(parsePaceMillions('0,5')).toBe(500_000);
  });

  it('rejects zero, negatives and text', () => {
    for (const value of ['', '0', '-1', 'abc', '1e6', '7.6M']) {
      expect(parsePaceMillions(value), value).toBeNull();
    }
  });
});

describe('validate', () => {
  it('builds a trainer query without balance', () => {
    expect(validate(trainer('1234 5678 9012'))).toEqual({
      ok: true,
      query: {
        source: { trainerId: '123456789012' },
        balanceWeight: null,
        joinStyle: null,
        playstyle: null,
        hasDiscord: false,
        includeFull: false,
      },
    });
  });

  it('builds a pace query with balance and filters', () => {
    expect(
      validate(
        pace('7.6', {
          balance: true,
          balanceWeight: 0.75,
          joinStyle: 'closed',
          playstyle: 12,
        }),
      ),
    ).toEqual({
      ok: true,
      query: {
        source: { pace: 7_600_000 },
        balanceWeight: 0.75,
        joinStyle: 'closed',
        playstyle: 12,
        hasDiscord: false,
        includeFull: false,
      },
    });
  });

  it('reports the field that is wrong', () => {
    expect(validate(trainer('123'))).toEqual({
      ok: false,
      errors: { trainerId: TRAINER_ID_ERROR },
    });
    expect(validate(pace('fast'))).toEqual({ ok: false, errors: { pace: PACE_ERROR } });
  });

  it('only checks the field for the current mode', () => {
    expect(validate(trainer('123456789', { paceMillions: 'not a number' })).ok).toBe(true);
  });
});

describe('URL state', () => {
  it('round-trips a trainer search', () => {
    const result = validate(trainer('123456789012', { balance: true, balanceWeight: 0.25 }));
    if (!result.ok) throw new Error('expected a valid search');
    const params = toUrlParams(result.query);
    expect(params.toString()).toBe('trainer_id=123456789012&balance_weight=0.25');
    expect(fromUrlParams(params)).toEqual(
      trainer('123456789012', { balance: true, balanceWeight: 0.25 }),
    );
  });

  it('round-trips a pace search with filters', () => {
    const filters = { joinStyle: 'approval' as JoinStyle, playstyle: 12, hasDiscord: true };
    const result = validate(pace('7.6', filters));
    if (!result.ok) throw new Error('expected a valid search');
    const params = toUrlParams(result.query);
    expect(params.toString()).toBe(
      'pace=7600000&join_style=approval&playstyle=12&has_discord=true',
    );
    expect(fromUrlParams(params)).toEqual(pace('7.6', filters));
  });

  it('ignores unknown or several join styles and playstyles in links', () => {
    for (const query of [
      'pace=7600000&join_style=locked&playstyle=99',
      'pace=7600000&join_style=open,closed&playstyle=2,12',
      'pace=7600000&include_closed=true&playstyle=x',
    ]) {
      const state = fromUrlParams(new URLSearchParams(query));
      expect(state?.joinStyle, query).toBeNull();
      expect(state?.playstyle, query).toBeNull();
    }
  });

  it('round-trips full clubs', () => {
    const result = validate(pace('7.6', { includeFull: true }));
    if (!result.ok) throw new Error('expected a valid search');
    const params = toUrlParams(result.query);
    expect(params.toString()).toBe('pace=7600000&include_full=true');
    expect(fromUrlParams(params)).toEqual(pace('7.6', { includeFull: true }));
  });

  it('ignores missing or invalid searches', () => {
    for (const query of ['', 'trainer_id=abc', 'pace=-1', 'pace=x', 'balance_weight=0.5']) {
      expect(fromUrlParams(new URLSearchParams(query)), query).toBeNull();
    }
  });

  it('clamps the balance weight and ignores a bad one', () => {
    expect(fromUrlParams(new URLSearchParams('pace=1000000&balance_weight=3'))).toMatchObject({
      balance: true,
      balanceWeight: 1,
    });
    expect(fromUrlParams(new URLSearchParams('pace=1000000&balance_weight=x'))).toMatchObject({
      balance: false,
    });
  });
});

describe('remembered trainer ID', () => {
  it('saves, loads and forgets the ID', () => {
    localStorage.clear();
    expect(loadRememberedTrainerId(localStorage)).toBeNull();
    saveRememberedTrainerId(localStorage, '1234 5678 9012');
    expect(loadRememberedTrainerId(localStorage)).toBe('123456789012');
    saveRememberedTrainerId(localStorage, null);
    expect(loadRememberedTrainerId(localStorage)).toBeNull();
  });

  it('ignores invalid stored values and missing storage', () => {
    localStorage.setItem('uma-stable-match:trainer-id', 'not an id');
    expect(loadRememberedTrainerId(localStorage)).toBeNull();
    expect(loadRememberedTrainerId(undefined)).toBeNull();
    expect(() => saveRememberedTrainerId(undefined, '123456789')).not.toThrow();
  });

  it('survives storage that throws', () => {
    const broken = {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
      removeItem: () => {
        throw new Error('blocked');
      },
    } as unknown as Storage;
    expect(loadRememberedTrainerId(broken)).toBeNull();
    expect(() => saveRememberedTrainerId(broken, '123456789')).not.toThrow();
  });
});
