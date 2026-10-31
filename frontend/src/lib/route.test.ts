// Address tests. All IDs are made up.

import { describe, expect, it } from 'vitest';

import { clubPath, clubRoute } from './route';

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
