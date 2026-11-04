// ApplicationsPage component tests with a stubbed fetch. All data is made up.

import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import ApplicationsPage from './ApplicationsPage.svelte';
import { t } from './i18n';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

// Answers by address: the session, a club's name and the opt-in check.
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
    expect(screen.queryByRole('link', { name: t('applications.session.signIn') })).not.toBeInTheDocument();
    expect(screen.queryByText(t('applications.session.signedOut'))).not.toBeInTheDocument();
    expect(screen.queryByLabelText(t('applications.pick.label'))).not.toBeInTheDocument();
  });

  it('says so when it cannot tell whether you are signed in', async () => {
    stub({ '/api/v1/applications/me': () => json({ error: 'down' }, 503) });
    vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    render(ApplicationsPage);
    expect(await screen.findByRole('alert')).toHaveTextContent(t('applications.session.error'));
  });

  it('picks the club, confirms it and sends the leader to add the bot', async () => {
    const assign = vi.fn();
    vi.stubGlobal('location', {
      ...window.location,
      assign,
      pathname: '/applications',
      search: '',
    });
    const fetchMock = stub({
      ...signedIn,
      '/api/v1/clubs/777': () => json({ club: { name: 'Made-up Club' }, members: [], month: null }),
      '/api/v1/discord/opt-in': () => ({ type: 'opaqueredirect', status: 0 }) as Response,
    });
    render(ApplicationsPage);

    const field = await screen.findByLabelText(t('applications.pick.label'));
    await fireEvent.input(field, { target: { value: 'https://uma.moe/circles/777' } });
    await fireEvent.click(screen.getByRole('button', { name: t('applications.pick.next') }));
    expect(await screen.findByText('Made-up Club')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: t('applications.confirm.add') }));
    await waitFor(() => expect(assign).toHaveBeenCalledOnce());
    expect(assign).toHaveBeenCalledWith(
      '/api/v1/discord/opt-in?club=777&next=%2Fapplications%2Fclub%2F777%3Flinked%3D1',
    );
    expect(fetchMock).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/discord/opt-in'),
      expect.objectContaining({ redirect: 'manual' }),
    );
  });

  it('refuses something that is not a club', async () => {
    stub(signedIn);
    render(ApplicationsPage);
    const field = await screen.findByLabelText(t('applications.pick.label'));
    await fireEvent.input(field, { target: { value: 'my club' } });
    await fireEvent.click(screen.getByRole('button', { name: t('applications.pick.next') }));
    expect(screen.getByRole('alert')).toHaveTextContent(t('applications.pick.invalid'));
    expect(field).toHaveAttribute('aria-invalid', 'true');
  });

  it.each([
    [404, 'applications.error.notFound'],
    [422, 'applications.error.noInvite'],
    [502, 'applications.error.unavailable'],
    [500, 'applications.error.generic'],
  ] as const)(
    'explains a %s from the opt-in check and stays on the page',
    async (status, message) => {
      const assign = vi.fn();
      vi.stubGlobal('location', { ...window.location, assign });
      stub({
        ...signedIn,
        '/api/v1/discord/opt-in': () => json({ error: 'server wording' }, status),
      });
      render(ApplicationsPage);
      await fireEvent.input(await screen.findByLabelText(t('applications.pick.label')), {
        target: { value: '777' },
      });
      await fireEvent.click(screen.getByRole('button', { name: t('applications.pick.next') }));
      await fireEvent.click(screen.getByRole('button', { name: t('applications.confirm.add') }));
      expect(await screen.findByRole('alert')).toHaveTextContent(t(message));
      expect(assign).not.toHaveBeenCalled();
      expect(screen.getByRole('button', { name: t('applications.confirm.add') })).toBeEnabled();
    },
  );

  it('asks for a sign-in again when the session ended before adding the bot', async () => {
    stub({
      ...signedIn,
      '/api/v1/discord/opt-in': () => json({ error: 'sign in with Discord first' }, 401),
    });
    render(ApplicationsPage);
    await fireEvent.input(await screen.findByLabelText(t('applications.pick.label')), {
      target: { value: '777' },
    });
    await fireEvent.click(screen.getByRole('button', { name: t('applications.pick.next') }));
    await fireEvent.click(screen.getByRole('button', { name: t('applications.confirm.add') }));
    expect(await screen.findByRole('alert')).toHaveTextContent(t('applications.error.signIn'));
    expect(
      screen.getByRole('link', { name: t('applications.session.signIn') }),
    ).toBeInTheDocument();
  });

  it('lets the leader choose another club', async () => {
    stub(signedIn);
    render(ApplicationsPage);
    await fireEvent.input(await screen.findByLabelText(t('applications.pick.label')), {
      target: { value: '777' },
    });
    await fireEvent.click(screen.getByRole('button', { name: t('applications.pick.next') }));
    await fireEvent.click(screen.getByRole('button', { name: t('applications.confirm.back') }));
    expect(screen.getByLabelText(t('applications.pick.label'))).toBeInTheDocument();
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

  it.each([
    ['wrong_server', 'applications.error.wrong_server'],
    ['denied', 'applications.error.denied'],
    ['invalid_state', 'applications.error.invalid_state'],
    ['no_server', 'applications.error.no_server'],
    ['unheard_of', 'applications.error.generic'],
  ] as const)('shows why a sign-in or opt-in failed: %s', (label, key) => {
    stub(signedOut);
    render(ApplicationsPage, { error: label });
    expect(screen.getByRole('alert')).toHaveTextContent(t(key));
  });

  it('shows no error when there was none', () => {
    stub(signedOut);
    render(ApplicationsPage);
    expect(screen.queryByText(t('applications.error.heading'))).not.toBeInTheDocument();
  });
});
