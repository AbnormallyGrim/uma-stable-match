// Saved session tests. All IDs and names are made up.

import { afterEach, describe, expect, it } from 'vitest';

import {
  ACCOUNT_KEY,
  forgetSession,
  saveAccount,
  savedAccount,
  sessionMarker,
  SIGNED_IN_COOKIE,
} from './session';

const account = { discordId: '100000000000000001', name: 'Made Up' };

afterEach(() => {
  localStorage.clear();
  document.cookie = `${SIGNED_IN_COOKIE}=; Path=/; Max-Age=0`;
});

describe('sessionMarker', () => {
  it('reads the marker among other cookies', () => {
    expect(sessionMarker('a=1; uma_signed_in=1794170000; b=2')).toBe('1794170000');
  });

  it('is null without it, or when it is empty', () => {
    expect(sessionMarker('')).toBeNull();
    expect(sessionMarker('a=1')).toBeNull();
    expect(sessionMarker('uma_signed_in=')).toBeNull();
  });
});

describe('savedAccount', () => {
  it('gives back the person saved for the same session', () => {
    saveAccount('1794170000', account, localStorage);
    expect(savedAccount('1794170000', localStorage)).toEqual(account);
  });

  it('ignores a person saved for another session', () => {
    saveAccount('1794170000', account, localStorage);
    expect(savedAccount('1794180000', localStorage)).toBeNull();
  });

  it('ignores something unreadable', () => {
    localStorage.setItem(ACCOUNT_KEY, '{not json');
    expect(savedAccount('1794170000', localStorage)).toBeNull();
  });

  it('works without storage', () => {
    saveAccount('1794170000', account, undefined);
    expect(savedAccount('1794170000', undefined)).toBeNull();
  });
});

describe('forgetSession', () => {
  it('drops the saved person and the marker', () => {
    document.cookie = `${SIGNED_IN_COOKIE}=1794170000; Path=/`;
    saveAccount('1794170000', account, localStorage);
    forgetSession(localStorage);
    expect(localStorage.getItem(ACCOUNT_KEY)).toBeNull();
    expect(sessionMarker()).toBeNull();
  });
});
