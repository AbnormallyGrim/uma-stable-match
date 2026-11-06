// LoginPage component tests with a stubbed fetch.

import { render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { flags } from './flags.svelte';
import LoginPage from './LoginPage.svelte';
import { websiteUrl } from './pages';

afterEach(() => {
  vi.unstubAllGlobals();
  flags.applications = false;
});

describe('LoginPage', () => {
  it('links to the privacy page beneath the sign-in card', () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.resolve(new Response('{}'))),
    );
    render(LoginPage);
    const card = screen.getByRole('region', { name: 'Sign in to Uma Stable Match' });
    const link = screen.getByRole('link', { name: 'Privacy Policy' });
    expect(link).toHaveAttribute('href', `${websiteUrl(window.location)}/privacy/`);
    expect(card.compareDocumentPosition(link) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(card).not.toContainElement(link);
  });

  it('links to the applications page in the navbar once the server has it on', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.resolve(new Response(JSON.stringify({ applications: true })))),
    );
    render(LoginPage);
    const nav = screen.getByRole('navigation', { name: 'Pages' });
    expect(await within(nav).findByRole('link', { name: 'Applications' })).toHaveAttribute(
      'href',
      '/applications',
    );
  });
});
