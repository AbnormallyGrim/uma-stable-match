// LanguageToggle tests.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { LANGUAGE_KEY } from './language';
import { language } from './language.svelte';
import LanguageToggle from './LanguageToggle.svelte';

afterEach(() => {
  localStorage.clear();
  language.choose('en');
  language.japaneseEnabled = false;
});

describe('LanguageToggle', () => {
  it('is hidden while Japanese is off', () => {
    render(LanguageToggle);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('names the language it switches to, and flips between the two', async () => {
    language.japaneseEnabled = true;
    render(LanguageToggle);
    await fireEvent.click(screen.getByRole('button', { name: 'Switch to Japanese (日本語)' }));
    expect(language.current).toBe('ja');
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe('ja');

    await fireEvent.click(screen.getByRole('button', { name: '英語に切り替え (English)' }));
    expect(language.current).toBe('en');
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe('en');
  });

  it('shows the remembered language', () => {
    language.japaneseEnabled = true;
    language.choose('ja');
    render(LanguageToggle);
    expect(screen.getByRole('button', { name: '英語に切り替え (English)' })).toBeInTheDocument();
  });
});
