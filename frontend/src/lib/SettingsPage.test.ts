// SettingsPage tests with a stubbed fetch. All IDs and names are made up.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { account } from './account.svelte';
import SettingsPage from './SettingsPage.svelte';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const empty = () => new Response(null, { status: 204 });

/** Answers `METHOD /path` from `routes`, and 404 for anything else. */
function answer(routes: Record<string, () => Response>) {
  const fetch = vi.fn((input: string, init?: RequestInit) => {
    const route = routes[`${init?.method ?? 'GET'} ${input.split('?')[0]}`];
    return Promise.resolve(route ? route() : json({ error: 'not found' }, 404));
  });
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

const assign = vi.fn();

beforeEach(() => {
  account.current = { discordId: '100000000000000001', name: 'Made Up' };
  vi.stubGlobal('location', { ...window.location, assign, pathname: '/settings' });
});

afterEach(() => {
  account.current = null;
  assign.mockReset();
  vi.unstubAllGlobals();
});

const TRAINERS = 'GET /api/v1/discord/trainers';
const T2 = '/api/v1/discord/trainers/100000000002';

interface Pending {
  trainer: string;
  key: string;
  expires_in: number;
  check: { state: string; seconds_left?: number } | null;
}

/** The list endpoint's answer. */
const list = (linked: { id: string; name: string | null }[] = [], pending: Pending[] = []) =>
  json({ linked, max: 5, pending });

const pendingKey = (check: Pending['check'] = null): Pending => ({
  trainer: '100000000002',
  key: 'uma-ABCDEFGH',
  expires_in: 86000,
  check,
});

describe('SettingsPage', () => {
  it('lists the linked trainers', async () => {
    answer({ [TRAINERS]: () => list([{ id: '100000000001', name: 'Example Trainer' }]) });
    render(SettingsPage);
    expect(await screen.findByText('100000000001')).toBeInTheDocument();
    expect(screen.getByText('Example Trainer')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Unlink trainer 100000000001' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Link Account' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign Out' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Delete Account' })).toBeInTheDocument();
  });

  it('refuses an ID that is not 9 to 12 digits without asking the server', async () => {
    const fetch = answer({ [TRAINERS]: () => list() });
    render(SettingsPage);
    await fireEvent.input(screen.getByLabelText('Trainer ID'), { target: { value: '1234' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Link Account' }));
    expect(await screen.findByText('Enter a trainer ID of 9 to 12 digits.')).toBeInTheDocument();
    expect(fetch.mock.calls.some(([url]) => url.includes('verification'))).toBe(false);
  });

  it('shows the key, and links at once when it is already in the comment', async () => {
    let linked: { id: string; name: string }[] = [];
    answer({
      [TRAINERS]: () => list(linked),
      [`POST ${T2}/verification`]: () => json({ key: 'uma-ABCDEFGH', expires_in: 86400 }),
      [`POST ${T2}/link`]: () => {
        linked = [{ id: '100000000002', name: 'Example Trainer' }];
        return json({ linked: true, name: 'Example Trainer' });
      },
    });
    render(SettingsPage);
    await fireEvent.input(screen.getByLabelText('Trainer ID'), {
      target: { value: '100 000 000 002' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Link Account' }));
    expect(await screen.findByTestId('verification-key')).toHaveTextContent('uma-ABCDEFGH');
    await fireEvent.click(screen.getByRole('button', { name: 'Check' }));
    expect(await screen.findByText(/Linked Example Trainer/)).toBeInTheDocument();
    expect(await screen.findByText('100000000002')).toBeInTheDocument();
    expect(screen.queryByTestId('verification-key')).not.toBeInTheDocument();
  });

  it('shows Verifying while the server keeps checking, then the link', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let found = false;
    const fetch = answer({});
    // Before the first Check, nothing is pending on the server.
    let started = false;
    fetch.mockImplementation((input: string, init?: RequestInit) => {
      const route = `${init?.method ?? 'GET'} ${input}`;
      if (route === TRAINERS && !started && !found) {
        return Promise.resolve(list());
      }
      if (route === `POST ${T2}/link`) {
        started = true;
        return Promise.resolve(json({ checking: true, seconds_left: 300 }, 202));
      }
      if (route === `POST ${T2}/verification`) {
        return Promise.resolve(json({ key: 'uma-ABCDEFGH', expires_in: 86400 }));
      }
      return Promise.resolve(
        found
          ? list([{ id: '100000000002', name: 'Example Trainer' }])
          : list([], [pendingKey({ state: 'checking', seconds_left: 300 })]),
      );
    });
    render(SettingsPage);
    await fireEvent.input(screen.getByLabelText('Trainer ID'), {
      target: { value: '100000000002' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Link Account' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Check' }));

    const button = await screen.findByRole('button', { name: 'Verifying' });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute('aria-busy', 'true');
    expect(button.querySelector('.spinner')).not.toBeNull();

    // The page only asks the server how it's going; the server found the key meanwhile.
    found = true;
    await vi.advanceTimersByTimeAsync(5000);
    expect(await screen.findByText(/Linked Example Trainer/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Verifying' })).not.toBeInTheDocument();
    const links = fetch.mock.calls.filter(([url, init]) => url.endsWith('/link') && init?.method);
    expect(links).toHaveLength(1);
    vi.useRealTimers();
  });

  it('picks up a pending key and its running check when opened again', async () => {
    answer({ [TRAINERS]: () => list([], [pendingKey({ state: 'checking', seconds_left: 120 })]) });
    render(SettingsPage);
    expect(await screen.findByTestId('verification-key')).toHaveTextContent('uma-ABCDEFGH');
    expect(screen.getByRole('button', { name: 'Verifying' })).toBeDisabled();
  });

  it('says when the check gave up without finding the key', async () => {
    answer({ [TRAINERS]: () => list([], [pendingKey({ state: 'missing' })]) });
    render(SettingsPage);
    expect(await screen.findByRole('alert')).toHaveTextContent('after 5 minutes');
    expect(screen.getByRole('button', { name: 'Check' })).toBeEnabled();
  });

  it('forgets the key on the server when cancelled', async () => {
    const fetch = answer({
      [TRAINERS]: () => list([], [pendingKey()]),
      [`DELETE ${T2}/verification`]: empty,
    });
    render(SettingsPage);
    await fireEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
    expect(fetch).toHaveBeenCalledWith(
      `${T2}/verification`,
      expect.objectContaining({ method: 'DELETE' }),
    );
    expect(screen.queryByTestId('verification-key')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Link Account' })).toBeInTheDocument();
  });

  it('says when the trainer is already linked', async () => {
    answer({
      [TRAINERS]: () => list(),
      'POST /api/v1/discord/trainers/100000000002/verification': () =>
        json({ error: 'already' }, 409),
    });
    render(SettingsPage);
    await fireEvent.input(screen.getByLabelText('Trainer ID'), {
      target: { value: '100000000002' },
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Link Account' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('already linked');
  });

  it('unlinks a trainer', async () => {
    answer({
      [TRAINERS]: () => list([{ id: '100000000001', name: 'Example Trainer' }]),
      'DELETE /api/v1/discord/trainers/100000000001': empty,
    });
    render(SettingsPage);
    await fireEvent.click(
      await screen.findByRole('button', { name: 'Unlink trainer 100000000001' }),
    );
    expect(await screen.findByText('No trainers linked yet.')).toBeInTheDocument();
  });

  it('hides linking when the server does not offer it', async () => {
    answer({});
    render(SettingsPage);
    expect(
      await screen.findByText('Linking trainers isn’t available right now.'),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Link Account' })).not.toBeInTheDocument();
  });

  it('signs out and goes to the finder', async () => {
    const fetch = answer({
      [TRAINERS]: () => list(),
      'POST /api/v1/auth/sign-out': empty,
    });
    render(SettingsPage);
    await fireEvent.click(screen.getByRole('button', { name: 'Sign Out' }));
    await vi.waitFor(() => expect(assign).toHaveBeenCalledWith('/'));
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/auth/sign-out',
      expect.objectContaining({ method: 'POST' }),
    );
    expect(account.current).toBeNull();
  });

  it('deletes the account only after confirming', async () => {
    const fetch = answer({
      [TRAINERS]: () => list(),
      'DELETE /api/v1/auth/account': empty,
    });
    render(SettingsPage);
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Account' }));
    expect(screen.getByRole('alert')).toHaveTextContent('Delete your account?');
    await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(fetch.mock.calls.some(([url]) => url === '/api/v1/auth/account')).toBe(false);

    await fireEvent.click(screen.getByRole('button', { name: 'Delete Account' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await vi.waitFor(() => expect(assign).toHaveBeenCalledWith('/'));
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/auth/account',
      expect.objectContaining({ method: 'DELETE' }),
    );
    expect(account.current).toBeNull();
  });

  it('keeps the account when deleting fails', async () => {
    answer({
      [TRAINERS]: () => list(),
      'DELETE /api/v1/auth/account': () => json({ error: 'down' }, 503),
    });
    render(SettingsPage);
    await fireEvent.click(screen.getByRole('button', { name: 'Delete Account' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    expect(await screen.findByText(/couldn’t delete your account/)).toHaveAttribute(
      'role',
      'alert',
    );
    expect(assign).not.toHaveBeenCalled();
    expect(account.current).not.toBeNull();
  });
});
