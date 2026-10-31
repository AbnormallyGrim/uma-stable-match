// Address tests. All IDs are made up.

import { describe, expect, it } from 'vitest';

import {
  clubPath,
  clubRoute,
  isVoteRoute,
  rememberedSearch,
  rememberSearch,
  VOTE_PATH,
} from './route';

describe('clubRoute', () => {
  it('reads the club and trainer', () => {
    expect(clubRoute('/club/100000001', '?trainer=123456789012')).toEqual({
      circleId: '100000001',
      trainerId: '123456789012',
    });
    expect(clubRoute('/club/7/', '')).toEqual({ circleId: '7', trainerId: null });
  });

  it('tidies the trainer ID and drops a broken one', () => {
    expect(clubRoute('/club/7', '?trainer=1234%205678%209012')?.trainerId).toBe('123456789012');
    expect(clubRoute('/club/7', '?trainer=me')).toEqual({ circleId: '7', trainerId: null });
  });

  it('leaves other pages alone', () => {
    for (const path of ['/', '/club', '/club/', '/club/abc', '/club/7/members', '/clubs/7']) {
      expect(clubRoute(path, ''), path).toBeNull();
    }
  });
});

describe('isVoteRoute', () => {
  it('matches the vote page, with or without a trailing slash', () => {
    expect(VOTE_PATH).toBe('/vote');
    expect(isVoteRoute('/vote')).toBe(true);
    expect(isVoteRoute('/vote/')).toBe(true);
  });

  it('leaves other pages alone', () => {
    for (const path of ['/', '/votes', '/vote/1', '/club/7']) {
      expect(isVoteRoute(path), path).toBe(false);
    }
  });
});

describe('clubPath', () => {
  it('builds the address back', () => {
    expect(clubPath(7)).toBe('/club/7');
    expect(clubPath('7', '123456789012')).toBe('/club/7?trainer=123456789012');
    const path = clubPath(7, '123456789012');
    expect(clubRoute(path.split('?')[0]!, `?${path.split('?')[1]}`)).toEqual({
      circleId: '7',
      trainerId: '123456789012',
    });
  });
});

describe('rememberSearch', () => {
  it('keeps the search address for the club page', () => {
    rememberSearch(sessionStorage, 'http://localhost:3000/?trainer_id=123456789012');
    expect(rememberedSearch(sessionStorage)).toBe('http://localhost:3000/?trainer_id=123456789012');
    sessionStorage.clear();
    expect(rememberedSearch(sessionStorage)).toBeNull();
  });

  it('copes without storage', () => {
    const broken = {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
    } as unknown as Storage;
    expect(() => rememberSearch(broken, '/')).not.toThrow();
    expect(rememberedSearch(broken)).toBeNull();
    expect(rememberedSearch(undefined)).toBeNull();
  });
});
