// NavBar component tests.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import NavBar from './NavBar.svelte';

const props = { onhelp: () => {}, onreleases: () => {}, unseen: false };

describe('NavBar', () => {
  it('links the brand back to the website', () => {
    render(NavBar, { ...props });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    const brand = within(nav).getByRole('link', { name: 'Uma Stable Match' });
    expect(brand).toHaveAttribute('href', 'https://umastable.com');
  });

  it('holds the theme switch', () => {
    render(NavBar, { ...props });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    expect(within(nav).getByRole('group', { name: 'Theme' })).toBeInTheDocument();
  });

  it('starts the tour from the help button', async () => {
    const onhelp = vi.fn();
    render(NavBar, { ...props, onhelp });
    const help = screen.getByRole('button', { name: 'Take the tour' });
    expect(help).toHaveAttribute('data-tooltip', 'Take the tour');
    await fireEvent.click(help);
    expect(onhelp).toHaveBeenCalledOnce();
  });

  it('has no help button without a tour', () => {
    render(NavBar, { onreleases: () => {}, unseen: false });
    expect(screen.queryByRole('button', { name: 'Take the tour' })).not.toBeInTheDocument();
  });

  it('opens the release notes from its button', async () => {
    const onreleases = vi.fn();
    render(NavBar, { ...props, onreleases });
    const button = screen.getByRole('button', { name: "What's new" });
    expect(button).toHaveAttribute('data-tooltip', "What's new");
    await fireEvent.click(button);
    expect(onreleases).toHaveBeenCalledOnce();
  });

  it('marks the release notes button only while there is something unseen', () => {
    const { unmount } = render(NavBar, { ...props, unseen: true });
    expect(screen.getByTestId('unseen-release')).toBeInTheDocument();
    unmount();
    render(NavBar, { ...props, unseen: false });
    expect(screen.queryByTestId('unseen-release')).not.toBeInTheDocument();
  });
});
