// AccountMenu and its place in the navbar. All names are made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { account } from './account.svelte';
import AccountMenu from './AccountMenu.svelte';
import NavBar from './NavBar.svelte';

afterEach(() => {
  account.current = null;
  account.known = false;
  vi.unstubAllGlobals();
});

function signIn(name: string | null = 'Made Up') {
  account.current = { discordId: '100000000000000001', name };
  account.known = true;
}

describe('AccountMenu', () => {
  it('shows nothing until the server has said who is signed in', () => {
    render(AccountMenu);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('shows a sign-in silhouette while signed out, coming back to the page', () => {
    account.known = true;
    render(AccountMenu);
    const link = screen.getByRole('link', { name: 'Sign in' });
    expect(link).toHaveAttribute('href', `/login?next=${encodeURIComponent('/')}`);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('leaves the silhouette out on the login page', () => {
    vi.stubGlobal('location', { ...window.location, pathname: '/login', search: '' });
    account.known = true;
    render(AccountMenu);
    expect(screen.queryByRole('link', { name: 'Sign in' })).not.toBeInTheDocument();
  });

  it('shows the Discord name, and opens the menu', async () => {
    signIn();
    render(AccountMenu);
    const button = screen.getByRole('button', { name: 'Made Up' });
    expect(button).toHaveAttribute('aria-expanded', 'false');
    await fireEvent.click(button);
    expect(button).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('Signed in as Made Up')).toBeInTheDocument();
    const menu = screen.getByRole('menu');
    expect(within(menu).getByRole('menuitem', { name: 'Settings' })).toHaveAttribute(
      'href',
      '/settings',
    );
    expect(within(menu).getByRole('menuitem', { name: 'Sign Out' })).toBeInTheDocument();
  });

  it('falls back to a generic name without one', () => {
    signIn(null);
    render(AccountMenu);
    expect(screen.getByRole('button', { name: 'Discord account' })).toBeInTheDocument();
  });

  it('opens from the keyboard on the first item and closes with Escape', async () => {
    signIn();
    render(AccountMenu);
    const button = screen.getByRole('button', { name: 'Made Up' });
    await fireEvent.keyDown(button, { key: 'ArrowDown' });
    const settings = await screen.findByRole('menuitem', { name: 'Settings' });
    expect(settings).toHaveFocus();
    await fireEvent.keyDown(settings, { key: 'ArrowDown' });
    expect(screen.getByRole('menuitem', { name: 'Sign Out' })).toHaveFocus();
    await fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(button).toHaveFocus();
  });

  it('closes on a click outside', async () => {
    signIn();
    render(AccountMenu);
    await fireEvent.click(screen.getByRole('button', { name: 'Made Up' }));
    await fireEvent.click(document.body);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('signs out and shows the silhouette in its place', async () => {
    const fetch = vi.fn(() => Promise.resolve(new Response(null, { status: 204 })));
    vi.stubGlobal('fetch', fetch);
    signIn();
    render(AccountMenu);
    await fireEvent.click(screen.getByRole('button', { name: 'Made Up' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Sign Out' }));
    await vi.waitFor(() => expect(account.current).toBeNull());
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/auth/sign-out',
      expect.objectContaining({ method: 'POST' }),
    );
    expect(screen.queryByRole('button', { name: 'Made Up' })).not.toBeInTheDocument();
    expect(await screen.findByRole('link', { name: 'Sign in' })).toBeInTheDocument();
  });

  it('says so when signing out fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.resolve(new Response('{}', { status: 503 }))),
    );
    signIn();
    render(AccountMenu);
    await fireEvent.click(screen.getByRole('button', { name: 'Made Up' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Sign Out' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('couldn’t sign you out');
    expect(account.current).not.toBeNull();
  });
});

describe('NavBar', () => {
  it('puts the sign-in silhouette last while signed out', () => {
    account.known = true;
    render(NavBar, { onhelp: () => {}, onreleases: () => {}, unseen: false });
    const link = screen.getByRole('link', { name: 'Sign in' });
    expect(link.closest('.tools')?.lastElementChild).toBe(link);
  });

  it('puts the account menu last, at the far right', () => {
    signIn();
    render(NavBar, { onhelp: () => {}, onreleases: () => {}, unseen: false });
    const tools = screen.getByRole('button', { name: 'Made Up' }).closest('.tools');
    const last = tools?.lastElementChild;
    expect(last).toContainElement(screen.getByRole('button', { name: 'Made Up' }));
  });
});
