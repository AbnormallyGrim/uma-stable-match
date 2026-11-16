// NavBar component tests.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import NavBar from './NavBar.svelte';
import { navTools } from './navTools.svelte';
import { PAGES } from './pages';

const props = { onhelp: () => {}, onreleases: () => {}, unseen: false };

describe('NavBar', () => {
  it('links the brand back to the website', () => {
    render(NavBar, { ...props });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    const brand = within(nav).getByRole('link', { name: 'Uma Stable Match' });
    // jsdom runs on localhost, so the link is the local website.
    expect(brand).toHaveAttribute('href', `http://site.localhost:${window.location.port}`);
  });

  it('holds the pages', () => {
    render(NavBar, { ...props });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    const pages = within(nav).getByRole('navigation', { name: 'Pages' });
    expect(within(pages).getAllByRole('link')).toHaveLength(PAGES.length);
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

describe('NavBar on narrow screens', () => {
  afterEach(() => {
    navTools.open = false;
    vi.unstubAllGlobals();
  });

  function narrow() {
    vi.stubGlobal('matchMedia', (query: string) => ({ matches: query === '(width < 768px)' }));
  }

  it('opens the folded tools, shrinking the pages pill, and swaps back from it', async () => {
    narrow();
    render(NavBar, { ...props });
    const fold = screen.getByRole('button', { name: 'More options' });
    const pages = screen.getByRole('navigation', { name: 'Pages' });
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(pages).not.toHaveClass('collapsed');

    await fireEvent.click(fold);
    expect(fold).toHaveAttribute('aria-expanded', 'true');
    expect(pages).toHaveClass('collapsed');
    expect(screen.getByRole('button', { name: "What's new" })).toHaveFocus();

    // jsdom is at /, the club finder, so the pill keeps that page.
    const kept = within(pages).getByRole('link', { name: 'Club finder' });
    expect(kept).toHaveClass('kept');
    const click = new MouseEvent('click', { bubbles: true, cancelable: true });
    kept.dispatchEvent(click);
    expect(click.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(fold).toHaveFocus());
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(pages).not.toHaveClass('collapsed');
  });

  it('closes the folded tools with Escape', async () => {
    narrow();
    render(NavBar, { ...props });
    const fold = screen.getByRole('button', { name: 'More options' });
    await fireEvent.click(fold);
    await fireEvent.keyDown(screen.getByRole('button', { name: "What's new" }), { key: 'Escape' });
    await vi.waitFor(() => expect(fold).toHaveFocus());
    expect(fold).toHaveAttribute('aria-expanded', 'false');
  });

  it('marks the fold button while there are unseen release notes', () => {
    render(NavBar, { ...props, unseen: true });
    const fold = screen.getByRole('button', { name: 'More options' });
    expect(fold.querySelector('.dot')).not.toBeNull();
  });
});
