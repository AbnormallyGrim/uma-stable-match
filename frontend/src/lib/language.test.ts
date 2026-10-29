// Language choice tests.

import { afterEach, describe, expect, it } from 'vitest';

import {
  LANGUAGE_COOKIE,
  LANGUAGE_KEY,
  cookieDomain,
  isLanguage,
  loadLanguage,
  resolveLanguage,
  saveLanguage,
} from './language';
import { LanguageState } from './language.svelte';

/** A `document` stand-in: a cookie jar that also records each cookie written. */
function fakeCookies(hostname: string, protocol: string, initial = '') {
  const jar = new Map(
    initial ? initial.split('; ').map((pair) => pair.split('=') as [string, string]) : [],
  );
  const written: string[] = [];
  return {
    written,
    location: { hostname, protocol },
    get cookie() {
      return [...jar].map(([name, value]) => `${name}=${value}`).join('; ');
    },
    set cookie(cookie: string) {
      written.push(cookie);
      const [name = '', value = ''] = cookie.split(';', 1)[0]?.split('=') ?? [];
      jar.set(name, value);
    },
  };
}

afterEach(() => {
  localStorage.clear();
});

const blocked = {
  getItem: () => {
    throw new Error('blocked');
  },
  setItem: () => {
    throw new Error('blocked');
  },
} as unknown as Storage;

describe('language', () => {
  it('recognises supported languages', () => {
    expect(isLanguage('en')).toBe(true);
    expect(isLanguage('ja')).toBe(true);
    expect(isLanguage('fr')).toBe(false);
    expect(isLanguage(null)).toBe(false);
  });

  it('defaults to English and remembers a choice', () => {
    expect(loadLanguage(localStorage)).toBe('en');
    saveLanguage(localStorage, 'ja');
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe('ja');
    expect(loadLanguage(localStorage)).toBe('ja');
  });

  it('falls back to English for bad values, missing storage and storage errors', () => {
    localStorage.setItem(LANGUAGE_KEY, 'klingon');
    expect(loadLanguage(localStorage)).toBe('en');
    expect(loadLanguage(undefined)).toBe('en');
    expect(loadLanguage(blocked)).toBe('en');
    expect(() => saveLanguage(blocked, 'ja')).not.toThrow();
  });

  it('shares the choice on umastable.com and keeps it on other hosts', () => {
    expect(cookieDomain('umastable.com')).toBe('umastable.com');
    expect(cookieDomain('app.umastable.com')).toBe('umastable.com');
    expect(cookieDomain('notumastable.com')).toBeUndefined();
    expect(cookieDomain('localhost')).toBeUndefined();
  });

  it('writes the shared cookie and prefers it over local storage', () => {
    const cookies = fakeCookies('app.umastable.com', 'https:');
    saveLanguage(localStorage, 'ja', cookies);
    expect(cookies.written).toEqual([
      `${LANGUAGE_COOKIE}=ja; Path=/; Max-Age=31536000; SameSite=Lax; Domain=umastable.com; Secure`,
    ]);
    localStorage.setItem(LANGUAGE_KEY, 'en');
    expect(loadLanguage(localStorage, cookies)).toBe('ja');
  });

  it('falls back to local storage without a usable cookie', () => {
    localStorage.setItem(LANGUAGE_KEY, 'ja');
    expect(loadLanguage(localStorage, fakeCookies('localhost', 'http:'))).toBe('ja');
    const local = fakeCookies('localhost', 'http:', `other=1; ${LANGUAGE_COOKIE}=klingon`);
    expect(loadLanguage(localStorage, local)).toBe('ja');
    saveLanguage(localStorage, 'en', local);
    expect(loadLanguage(undefined, local)).toBe('en');
    expect(local.written).toEqual([
      `${LANGUAGE_COOKIE}=en; Path=/; Max-Age=31536000; SameSite=Lax`,
    ]);
  });

  it('shows the saved language only when Japanese is enabled', () => {
    expect(resolveLanguage(true, 'ja')).toBe('ja');
    expect(resolveLanguage(true, 'en')).toBe('en');
    expect(resolveLanguage(false, 'ja')).toBe('en');
  });
});

describe('LanguageState', () => {
  it('starts in English when nothing was saved', () => {
    const state = new LanguageState(localStorage);
    state.japaneseEnabled = true;
    expect(state.current).toBe('en');
  });

  it('restores the saved choice once Japanese is enabled', () => {
    saveLanguage(localStorage, 'ja');
    const state = new LanguageState(localStorage);
    expect(state.current).toBe('en');
    state.japaneseEnabled = true;
    expect(state.current).toBe('ja');
  });

  it('remembers a new choice and keeps it while Japanese is off', () => {
    const state = new LanguageState(localStorage);
    state.japaneseEnabled = true;
    state.choose('ja');
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe('ja');
    state.japaneseEnabled = false;
    expect(state.current).toBe('en');
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe('ja');
  });

  it('picks up a choice made on the other site', () => {
    const cookies = fakeCookies('app.umastable.com', 'https:', `${LANGUAGE_COOKIE}=ja`);
    const state = new LanguageState(localStorage, cookies);
    state.japaneseEnabled = true;
    expect(state.current).toBe('ja');
    state.choose('en');
    expect(cookies.cookie).toBe(`${LANGUAGE_COOKIE}=en`);
  });

  it('works without storage', () => {
    const state = new LanguageState(undefined);
    state.japaneseEnabled = true;
    state.choose('ja');
    expect(state.current).toBe('ja');
  });
});
