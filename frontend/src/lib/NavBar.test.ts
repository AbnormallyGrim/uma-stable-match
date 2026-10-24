// NavBar component tests.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import NavBar from './NavBar.svelte';

describe('NavBar', () => {
  it('links the brand back to the website', () => {
    render(NavBar, { onhelp: () => {} });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    const brand = within(nav).getByRole('link', { name: 'Uma Stable Match' });
    expect(brand).toHaveAttribute('href', 'https://umastable.com');
  });

  it('holds the theme switch', () => {
    render(NavBar, { onhelp: () => {} });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    expect(within(nav).getByRole('group', { name: 'Theme' })).toBeInTheDocument();
  });

  it('starts the tour from the help button', async () => {
    const onhelp = vi.fn();
    render(NavBar, { onhelp });
    const help = screen.getByRole('button', { name: 'Take the tour' });
    expect(help).toHaveAttribute('data-tooltip', 'Take the tour');
    await fireEvent.click(help);
    expect(onhelp).toHaveBeenCalledOnce();
  });
});
