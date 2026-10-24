// NavBar component tests.

import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import NavBar from './NavBar.svelte';

describe('NavBar', () => {
  it('links the brand back to the website', () => {
    render(NavBar);
    const nav = screen.getByRole('navigation', { name: 'Main' });
    const brand = within(nav).getByRole('link', { name: 'Uma Stable Match' });
    expect(brand).toHaveAttribute('href', 'https://umastable.com');
  });

  it('holds the theme switch', () => {
    render(NavBar);
    const nav = screen.getByRole('navigation', { name: 'Main' });
    expect(within(nav).getByRole('group', { name: 'Theme' })).toBeInTheDocument();
  });
});
