// NavLinks component tests.

import { render, screen, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';

import NavLinks from './NavLinks.svelte';
import { flags } from './flags.svelte';
import { currentPage, visiblePages, websiteUrl } from './pages';

afterEach(() => {
  window.history.pushState({}, '', '/');
  flags.applications = false;
});

describe('NavLinks', () => {
  it('links to the home page, the club finder and the wishlist page', () => {
    render(NavLinks);
    const nav = screen.getByRole('navigation', { name: 'Pages' });
    expect(
      within(nav)
        .getAllByRole('link')
        .map((link) => [link.textContent?.trim(), link.getAttribute('href')]),
    ).toEqual([
      ['Home', `http://site.localhost:${window.location.port}`],
      ['Club finder', '/'],
      ['Wishlist', '/wishlist'],
    ]);
  });

  it('adds the applications link once the server has it on', async () => {
    render(NavLinks);
    const nav = screen.getByRole('navigation', { name: 'Pages' });
    expect(within(nav).queryByRole('link', { name: 'Applications' })).toBeNull();
    flags.applications = true;
    await tick();
    expect(within(nav).getByRole('link', { name: 'Applications' })).toHaveAttribute(
      'href',
      '/applications',
    );
  });

  it('keeps the applications page out of the navbar unless its flag is on', () => {
    expect(visiblePages(false).map((page) => page.id)).toEqual(['home', 'finder', 'wishlist']);
    expect(visiblePages(true).map((page) => page.id)).toEqual([
      'home',
      'finder',
      'wishlist',
      'applications',
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
    expect(currentPage('/applications')).toBe('applications');
    expect(currentPage('/applications/club/7')).toBe('applications');
    expect(currentPage('/login')).toBe('applications');
  });
});

describe('websiteUrl', () => {
  it('is umastable.com everywhere but locally', () => {
    for (const hostname of ['app.umastable.com', 'example.com']) {
      expect(websiteUrl({ protocol: 'https:', hostname, port: '' })).toBe('https://umastable.com');
    }
  });

  it('is site.localhost on the same server locally', () => {
    expect(websiteUrl({ protocol: 'http:', hostname: 'localhost', port: '8080' })).toBe(
      'http://site.localhost:8080',
    );
    expect(websiteUrl({ protocol: 'http:', hostname: '127.0.0.1', port: '' })).toBe(
      'http://site.localhost',
    );
    // Without SITE_HOST, the server sends the app to site.localhost too.
    expect(websiteUrl({ protocol: 'http:', hostname: 'site.localhost', port: '8080' })).toBe(
      'http://site.localhost:8080',
    );
  });
});
