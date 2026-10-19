// theme tests.

import { afterEach, describe, expect, it } from 'vitest';

import { THEME_KEY, applyTheme, isThemeChoice, loadTheme, saveTheme } from './theme';

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});

describe('theme', () => {
  it('recognises theme choices', () => {
    expect(isThemeChoice('system')).toBe(true);
    expect(isThemeChoice('dark')).toBe(true);
    expect(isThemeChoice('blue')).toBe(false);
    expect(isThemeChoice(null)).toBe(false);
  });

  it('defaults to system and remembers light or dark', () => {
    expect(loadTheme(localStorage)).toBe('system');
    saveTheme(localStorage, 'dark');
    expect(localStorage.getItem(THEME_KEY)).toBe('dark');
    expect(loadTheme(localStorage)).toBe('dark');
    saveTheme(localStorage, 'system');
    expect(localStorage.getItem(THEME_KEY)).toBeNull();
  });

  it('ignores bad stored values, missing storage and storage errors', () => {
    localStorage.setItem(THEME_KEY, 'neon');
    expect(loadTheme(localStorage)).toBe('system');
    expect(loadTheme(undefined)).toBe('system');
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
    expect(loadTheme(broken)).toBe('system');
    expect(() => saveTheme(broken, 'dark')).not.toThrow();
  });

  it('sets or clears data-theme on the root element', () => {
    applyTheme('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
    applyTheme('light');
    expect(document.documentElement.dataset.theme).toBe('light');
    applyTheme('system');
    expect(document.documentElement).not.toHaveAttribute('data-theme');
  });
});
