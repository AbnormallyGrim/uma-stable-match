// Language choice tests.

import { afterEach, describe, expect, it } from 'vitest';

import { LANGUAGE_KEY, isLanguage, loadLanguage, resolveLanguage, saveLanguage } from './language';
import { LanguageState } from './language.svelte';

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

  it('works without storage', () => {
    const state = new LanguageState(undefined);
    state.japaneseEnabled = true;
    state.choose('ja');
    expect(state.current).toBe('ja');
  });
});
