// LoginPage component tests with a stubbed fetch.

import { render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import LoginPage from './LoginPage.svelte';
import { websiteUrl } from './pages';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('LoginPage', () => {
  it('leaves the privacy link to the page footer', () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.resolve(new Response('{}'))),
    );
    render(LoginPage);
    const link = screen.getByRole('link', { name: 'Privacy Policy' });
    expect(link).toHaveAttribute('href', `${websiteUrl(window.location)}/privacy/`);
    expect(screen.getByRole('contentinfo')).toContainElement(link);
  });

  it('links to the applications page in the navbar', () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.resolve(new Response('{}'))),
    );
    render(LoginPage);
    const nav = screen.getByRole('navigation', { name: 'Pages' });
    expect(within(nav).getByRole('link', { name: 'Applications' })).toHaveAttribute(
      'href',
      '/applications',
    );
  });
});
