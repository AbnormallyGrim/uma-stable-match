// NavLinks component tests.

import { render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import NavLinks from './NavLinks.svelte';
import { currentPage } from './pages';

afterEach(() => window.history.pushState({}, '', '/'));

describe('NavLinks', () => {
  it('links to the home page, the club finder and the wishlist page', () => {
    render(NavLinks);
    const nav = screen.getByRole('navigation', { name: 'Pages' });
    expect(
      within(nav)
        .getAllByRole('link')
        .map((link) => [link.textContent?.trim(), link.getAttribute('href')]),
    ).toEqual([
      ['Home', 'https://umastable.com'],
      ['Club finder', '/'],
      ['Wishlist', '/wishlist'],
    ]);
  });

  it('marks the page being shown', () => {
    for (const [path, name] of [
      ['/', 'Club finder'],
      ['/club/100000001', 'Club finder'],
      ['/wishlist', 'Wishlist'],
    ] as const) {
      window.history.pushState({}, '', path);
      const { unmount } = render(NavLinks);
      const marked = screen
        .getAllByRole('link')
        .filter((link) => link.getAttribute('aria-current') === 'page');
      expect(
        marked.map((link) => link.textContent?.trim()),
        path,
      ).toEqual([name]);
      unmount();
    }
  });
});

describe('currentPage', () => {
  it('puts the search and club pages under the finder', () => {
    expect(currentPage('/')).toBe('finder');
    expect(currentPage('/club/7')).toBe('finder');
    expect(currentPage('/wishlist')).toBe('wishlist');
  });
});
