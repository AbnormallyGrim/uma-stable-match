// Message lookup tests.

import { afterEach, describe, expect, it } from 'vitest';

import { hasMessage, locale, t } from './i18n';
import { language } from './language.svelte';
import { en } from './messages/en';
import { ja } from './messages/ja';

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

  it('shows Japanese once chosen and enabled, and English while it is off', () => {
    language.choose('ja');
    expect(t('form.find')).toBe('Find clubs');
    language.japaneseEnabled = true;
    expect(t('form.find')).toBe('サークルを探す');
    expect(t('format.place', { place: 8, size: 30 })).toBe('30人中8位');
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

// Values for every message that takes them.
const SAMPLE = {
  n: 2,
  total: 3,
  shown: 1,
  place: 4,
  size: 30,
  percent: 46,
  year: '2026',
  month: 2,
  value: '0.50',
  wait: '5',
  rank: 3,
  tier: 'A',
  score: 'x',
  name: 'Example',
  own: '6.0M/day',
  pace: '7.6M/day',
  balanced: true,
  rankings: 'https://example.com',
} as never;

function text(message: unknown): string {
  return typeof message === 'function' ? message(SAMPLE) : String(message);
}

// The words a visitor would read, with any markup parsed away rather than pattern-matched out.
function visibleText(message: unknown): string {
  return new DOMParser().parseFromString(text(message), 'text/html').body.textContent ?? '';
}

describe('the Japanese catalog', () => {
  it('has exactly the English messages', () => {
    expect(Object.keys(ja).sort()).toEqual(Object.keys(en).sort());
  });

  it('uses the Japanese term for a club, not the English word', () => {
    for (const [key, message] of Object.entries(ja)) {
      expect(visibleText(message), key).not.toMatch(/club/i);
    }
  });

  it('produces text for every message that takes values', () => {
    for (const [key, message] of Object.entries(ja)) {
      expect(text(message), key).not.toBe('');
      expect(text(message), key).not.toContain('undefined');
    }
  });
});
