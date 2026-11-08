// ApplicationsPage component tests with a stubbed fetch. All data is made up.

import { render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import ApplicationsPage from './ApplicationsPage.svelte';
import { t } from './i18n';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

// Answers by address: the session state.
function stub(routes: Record<string, () => Response | Promise<Response>>) {
  const fetchMock = vi.fn((input: string) => {
    const path = input.split('?')[0]!;
    const route = routes[path];
    return route ? Promise.resolve(route()) : Promise.resolve(json({ error: 'not found' }, 404));
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

const signedIn = { '/api/v1/auth/session': () => json({ discord_id: '42' }) };
const signedOut = { '/api/v1/auth/session': () => json({ error: 'not signed in' }, 401) };

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('ApplicationsPage', () => {
  it('shows its heading and intro, with no privacy link outside the footer', () => {
    stub(signedOut);
    render(ApplicationsPage);
    expect(
      screen.getByRole('heading', { level: 1, name: t('applications.heading') }),
    ).toBeInTheDocument();
    expect(screen.getByText(t('applications.intro'))).toBeInTheDocument();
    const links = within(screen.getByRole('main')).queryAllByRole('link');
    expect(links.some((link) => /\/privacy\/$/.test(link.getAttribute('href') ?? ''))).toBe(false);
  });

  it('offers no sign-in link to a signed-out visitor', async () => {
    stub(signedOut);
    render(ApplicationsPage);
    expect(
      screen.queryByRole('link', { name: t('applications.session.signIn') }),
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText(/role/i)).not.toBeInTheDocument();
  });

  it("doesn't check the session itself, since the page only opens once someone is signed in", () => {
    const fetchMock = stub(signedIn);
    render(ApplicationsPage);
    expect(fetchMock).not.toHaveBeenCalledWith('/api/v1/auth/session', expect.anything());
  });

  it('has no sign-out button, signed in or not', async () => {
    stub(signedIn);
    render(ApplicationsPage);
    await screen.findByText(t('applications.role.leader.title'));
    expect(screen.queryByRole('button', { name: /sign out/i })).not.toBeInTheDocument();
  });

  it('shows no error when there was none', () => {
    stub(signedOut);
    render(ApplicationsPage);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
