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
    expect(screen.queryByRole('group')).not.toBeInTheDocument();
  });

  it('starts on English and switches to Japanese, remembering the choice', async () => {
    language.japaneseEnabled = true;
    render(LanguageToggle);
    expect(screen.getByRole('group', { name: 'Language' })).toBeInTheDocument();
    expect(screen.getByLabelText('English')).toBeChecked();

    await fireEvent.click(screen.getByLabelText('日本語'));
    expect(language.current).toBe('ja');
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe('ja');
    expect(screen.getByRole('group', { name: '言語' })).toBeInTheDocument();
    expect(screen.getByLabelText('日本語')).toBeChecked();

    await fireEvent.click(screen.getByLabelText('English'));
    expect(language.current).toBe('en');
    expect(localStorage.getItem(LANGUAGE_KEY)).toBe('en');
  });

  it('shows the remembered language', () => {
    language.japaneseEnabled = true;
    language.choose('ja');
    render(LanguageToggle);
    expect(screen.getByLabelText('日本語')).toBeChecked();
  });
});
