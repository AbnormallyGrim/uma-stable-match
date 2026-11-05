// ApplicationsPage component tests with a stubbed fetch. All data is made up.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import ApplicationsPage from './ApplicationsPage.svelte';
import { t } from './i18n';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

// Answers by address: the session and sign-out state.
function stub(routes: Record<string, () => Response | Promise<Response>>) {
  const fetchMock = vi.fn((input: string) => {
    const path = input.split('?')[0]!;
    const route = routes[path];
    return route ? Promise.resolve(route()) : Promise.resolve(json({ error: 'not found' }, 404));
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

const signedIn = { '/api/v1/applications/me': () => json({ discord_id: '42' }) };
const signedOut = { '/api/v1/applications/me': () => json({ error: 'not signed in' }, 401) };

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('ApplicationsPage', () => {
  it('shows its heading and intro, and links to the privacy page', () => {
    stub(signedOut);
    render(ApplicationsPage);
    expect(
      screen.getByRole('heading', { level: 1, name: t('applications.heading') }),
    ).toBeInTheDocument();
    expect(screen.getByText(t('applications.intro'))).toBeInTheDocument();
    const links = screen.getAllByRole('link');
    expect(links.some((link) => /\/privacy\/$/.test(link.getAttribute('href') ?? ''))).toBe(true);
  });

  it('asks a signed-out visitor to sign in with Discord', async () => {
    stub(signedOut);
    render(ApplicationsPage);
    expect(
      screen.queryByRole('link', { name: t('applications.session.signIn') }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(t('applications.session.signedOut'))).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/role/i)).not.toBeInTheDocument();
  });

  it('says so when it cannot tell whether you are signed in', async () => {
    stub({ '/api/v1/applications/me': () => json({ error: 'down' }, 503) });
    vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    render(ApplicationsPage);
    expect(await screen.findByRole('alert')).toHaveTextContent(t('applications.session.error'));
  });

  it('signs out', async () => {
    const fetchMock = stub({
      ...signedIn,
      '/api/v1/auth/sign-out': () => new Response(null, { status: 204 }),
    });
    render(ApplicationsPage);
    await fireEvent.click(
      await screen.findByRole('button', { name: t('applications.session.signOut') }),
    );
    expect(
      await screen.findByRole('link', { name: t('applications.session.signIn') }),
    ).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/v1/auth/sign-out',
      expect.objectContaining({ method: 'POST' }),
    );
  });

  it('shows no error when there was none', () => {
    stub(signedOut);
    render(ApplicationsPage);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
