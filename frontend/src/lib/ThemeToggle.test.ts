// ThemeToggle tests.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { THEME_KEY } from './theme';
import ThemeToggle from './ThemeToggle.svelte';

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});

describe('ThemeToggle', () => {
  it('starts on System with no theme forced', () => {
    render(ThemeToggle, { storage: localStorage });
    expect(screen.getByRole('group', { name: 'Theme' })).toBeInTheDocument();
    expect(screen.getByLabelText('System')).toBeChecked();
    expect(document.documentElement).not.toHaveAttribute('data-theme');
  });

  it('forces and remembers a theme', async () => {
    render(ThemeToggle, { storage: localStorage });
    await fireEvent.click(screen.getByLabelText('Dark'));
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(localStorage.getItem(THEME_KEY)).toBe('dark');

    await fireEvent.click(screen.getByLabelText('Light'));
    expect(document.documentElement.dataset.theme).toBe('light');
    expect(localStorage.getItem(THEME_KEY)).toBe('light');
  });

  it('goes back to following the system', async () => {
    localStorage.setItem(THEME_KEY, 'dark');
    render(ThemeToggle, { storage: localStorage });
    expect(screen.getByLabelText('Dark')).toBeChecked();
    expect(document.documentElement.dataset.theme).toBe('dark');

    await fireEvent.click(screen.getByLabelText('System'));
    expect(document.documentElement).not.toHaveAttribute('data-theme');
    expect(localStorage.getItem(THEME_KEY)).toBeNull();
  });

  it('can be used with the keyboard', async () => {
    render(ThemeToggle, { storage: localStorage });
    const radios = screen.getAllByRole('radio');
    expect(radios.map((r) => r.getAttribute('name'))).toEqual(['theme', 'theme', 'theme']);
    radios[2]!.focus();
    await fireEvent.click(radios[2]!);
    expect(screen.getByLabelText('Dark')).toBeChecked();
  });
});
