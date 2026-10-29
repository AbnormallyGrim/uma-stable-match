// Message lookup tests.

import { afterEach, describe, expect, it } from 'vitest';

import { hasMessage, locale, t } from './i18n';
import { language } from './language.svelte';
import { en } from './messages/en';

afterEach(() => {
  language.choose('en');
  language.japaneseEnabled = false;
});

describe('t', () => {
  it('returns a plain message', () => {
    expect(t('form.find')).toBe('Find clubs');
  });

  it('fills in values and handles plurals', () => {
    expect(t('club.openSpots', { n: 1 })).toBe('1 open spot');
    expect(t('club.openSpots', { n: 5 })).toBe('5 open spots');
    expect(t('format.place', { place: 8, size: 30 })).toBe('8 of 30');
  });

  it('shows English for a language with no catalog yet', () => {
    language.japaneseEnabled = true;
    language.choose('ja');
    expect(language.current).toBe('ja');
    expect(t('form.find')).toBe('Find clubs');
    expect(locale()).toBe('ja');
  });
});

describe('hasMessage', () => {
  it('knows which keys exist', () => {
    expect(hasMessage('playstyle.5')).toBe(true);
    expect(hasMessage('playstyle.99')).toBe(false);
    expect(hasMessage('toString')).toBe(false);
  });
});

describe('the English catalog', () => {
  it('has a name for every playstyle code', () => {
    for (let code = 1; code <= 17; code++) {
      expect(en).toHaveProperty(`playstyle.${code}`);
    }
  });
});
