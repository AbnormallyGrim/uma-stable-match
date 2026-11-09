// NavLinks component tests.

import { render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import NavLinks from './NavLinks.svelte';
import { currentPage, hasSeenPage, markPageSeen, websiteUrl } from './pages';

afterEach(() => {
  window.history.pushState({}, '', '/');
  localStorage.clear();
});

describe('NavLinks', () => {
  it('links to the home page, the club finder, the wishlist page, applications and ads', () => {
    markPageSeen(localStorage, 'applications');
    markPageSeen(localStorage, 'board');
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
      ['Applications', '/applications'],
      ['Recruitment Board', '/recruitment-board'],
    ]);
  });

  it('marks the page being shown', () => {
    for (const [path, name] of [
      ['/', 'Club finder'],
      ['/club/100000001', 'Club finder'],
      ['/wishlist', 'Wishlist'],
      ['/recruitment-board', 'Recruitment Board'],
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
    expect(currentPage('/recruitment-board')).toBe('board');
    expect(currentPage('/settings')).toBeNull();
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

describe('New badges', () => {
  const badges = () =>
    screen
      .queryAllByText('New')
      .map((badge) => badge.closest('a')?.textContent?.replace(/\s+/g, ' ').trim());

  it('mark the applications and recruitment board links until they have been visited', () => {
    render(NavLinks);
    expect(badges()).toEqual(['Applications New', 'Recruitment Board New']);
  });

  it('clear on a visit, and stay cleared', () => {
    window.history.pushState({}, '', '/recruitment-board');
    const { unmount } = render(NavLinks);
    expect(hasSeenPage(localStorage, 'board')).toBe(true);
    expect(badges()).toEqual(['Applications New']);
    unmount();
    window.history.pushState({}, '', '/wishlist');
    render(NavLinks);
    expect(badges()).toEqual(['Applications New']);
  });

  it('don’t clear from the sign-in page, which belongs to applications', () => {
    window.history.pushState({}, '', '/login');
    render(NavLinks);
    expect(hasSeenPage(localStorage, 'applications')).toBe(false);
  });

  it('never show when storage is blocked', () => {
    expect(hasSeenPage(undefined, 'board')).toBe(true);
  });
});
