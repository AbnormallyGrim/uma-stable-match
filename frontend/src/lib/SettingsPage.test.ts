// SettingsPage component tests with a stubbed fetch. All data is made up.

import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { Settings } from './applications';
import { t } from './i18n';
import SettingsPage from './SettingsPage.svelte';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const settings: Settings = {
  guild_id: '5000',
  category: null,
  leader_roles: ['20'],
  auto_join: true,
  open: true,
  problem: null,
  categories: [
    { id: '10', name: 'Applications' },
    { id: '11', name: 'Archive' },
  ],
  roles: [
    { id: '20', name: 'Officers' },
    { id: '21', name: 'Veterans' },
  ],
};

type Handler = (init?: RequestInit) => Response;

function stub(routes: Record<string, Handler>) {
  const fetchMock = vi.fn((input: string, init?: RequestInit) => {
    const handler = routes[`${init?.method ?? 'GET'} ${input}`];
    return Promise.resolve(handler ? handler(init) : json({ error: 'not found' }, 404));
  });
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

const SETTINGS = '/api/v1/discord/clubs/7/settings';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('SettingsPage', () => {
  it('shows the settings with the servers categories and roles', async () => {
    stub({
      [`GET ${SETTINGS}`]: () => json(settings),
      'GET /api/v1/clubs/7': () => json({ club: { name: 'Made-up Club' }, members: [] }),
    });
    render(SettingsPage, { clubId: '7' });

    expect(
      await screen.findByRole('heading', {
        level: 1,
        name: t('settings.heading', { name: 'Made-up Club' }),
      }),
    ).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: t('settings.open') })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: t('settings.autoJoin') })).toBeChecked();
    const category = screen.getByRole('combobox', { name: t('settings.category') });
    expect(
      within(category)
        .getAllByRole('option')
        .map((o) => o.textContent),
    ).toEqual([t('settings.categoryNone'), 'Applications', 'Archive']);
    expect(screen.getByRole('checkbox', { name: 'Officers' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: 'Veterans' })).not.toBeChecked();
  });

  it('says the club was just linked, and shows a setup problem', async () => {
    stub({ [`GET ${SETTINGS}`]: () => json({ ...settings, problem: 'made-up problem' }) });
    render(SettingsPage, { clubId: '7', linked: true });
    expect(await screen.findByText(t('settings.linked'))).toBeInTheDocument();
    expect(
      screen.getByText(t('settings.problem', { problem: 'made-up problem' })),
    ).toBeInTheDocument();
  });

  it('saves what was changed', async () => {
    const fetchMock = stub({
      [`GET ${SETTINGS}`]: () => json(settings),
      [`PATCH ${SETTINGS}`]: (init) => json({ ...settings, ...JSON.parse(String(init?.body)) }),
    });
    render(SettingsPage, { clubId: '7' });

    await fireEvent.click(await screen.findByRole('checkbox', { name: t('settings.open') }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Veterans' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Officers' }));
    await fireEvent.change(screen.getByRole('combobox', { name: t('settings.category') }), {
      target: { value: '11' },
    });
    await fireEvent.click(screen.getByRole('button', { name: t('settings.save') }));

    expect(await screen.findByText(t('settings.saved'))).toBeInTheDocument();
    const patch = fetchMock.mock.calls.find(([, init]) => init?.method === 'PATCH')!;
    expect(JSON.parse(String(patch[1]?.body))).toEqual({
      open: false,
      auto_join: true,
      category: '11',
      leader_roles: ['21'],
    });
    expect(screen.getByRole('combobox', { name: t('settings.category') })).toHaveValue('11');
  });

  it('clears the category with Not set', async () => {
    const fetchMock = stub({
      [`GET ${SETTINGS}`]: () => json({ ...settings, category: '10' }),
      [`PATCH ${SETTINGS}`]: () => json(settings),
    });
    render(SettingsPage, { clubId: '7' });
    await fireEvent.change(await screen.findByRole('combobox', { name: t('settings.category') }), {
      target: { value: '' },
    });
    await fireEvent.click(screen.getByRole('button', { name: t('settings.save') }));
    await screen.findByText(t('settings.saved'));
    const patch = fetchMock.mock.calls.find(([, init]) => init?.method === 'PATCH')!;
    expect(JSON.parse(String(patch[1]?.body)).category).toBeNull();
  });

  it('says when saving fails', async () => {
    stub({
      [`GET ${SETTINGS}`]: () => json(settings),
      [`PATCH ${SETTINGS}`]: () => json({ error: 'no' }, 500),
    });
    render(SettingsPage, { clubId: '7' });
    await fireEvent.click(await screen.findByRole('button', { name: t('settings.save') }));
    expect(await screen.findByRole('alert')).toHaveTextContent(t('settings.saveError'));
  });

  it('asks before unlinking, and can be cancelled', async () => {
    const fetchMock = stub({ [`GET ${SETTINGS}`]: () => json(settings) });
    render(SettingsPage, { clubId: '7' });
    await fireEvent.click(await screen.findByRole('button', { name: t('settings.unlink') }));
    expect(screen.getByRole('alert')).toHaveTextContent(t('settings.unlinkConfirm'));
    await fireEvent.click(screen.getByRole('button', { name: t('settings.cancel') }));
    expect(screen.queryByRole('button', { name: t('settings.unlinkYes') })).not.toBeInTheDocument();
    expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'DELETE')).toBe(false);
  });

  it('unlinks once confirmed', async () => {
    const fetchMock = stub({
      [`GET ${SETTINGS}`]: () => json(settings),
      'DELETE /api/v1/discord/clubs/7': () => new Response(null, { status: 204 }),
    });
    render(SettingsPage, { clubId: '7' });
    await fireEvent.click(await screen.findByRole('button', { name: t('settings.unlink') }));
    await fireEvent.click(screen.getByRole('button', { name: t('settings.unlinkYes') }));
    expect(await screen.findByText(t('settings.unlinked'))).toBeInTheDocument();
    await waitFor(() =>
      expect(fetchMock.mock.calls.some(([, init]) => init?.method === 'DELETE')).toBe(true),
    );
    expect(screen.queryByRole('button', { name: t('settings.save') })).not.toBeInTheDocument();
  });

  it.each([
    [401, 'settings.signIn'],
    [403, 'settings.forbidden'],
    [404, 'settings.notLinked'],
    [500, 'settings.loadError'],
  ] as const)('shows what a %s means and no settings', async (status, key) => {
    stub({ [`GET ${SETTINGS}`]: () => json({ error: 'no' }, status) });
    render(SettingsPage, { clubId: '7' });
    expect(await screen.findByText(t(key))).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: t('settings.save') })).not.toBeInTheDocument();
  });

  it('sends a signed-out visitor to sign in and back to the settings', async () => {
    stub({ [`GET ${SETTINGS}`]: () => json({ error: 'not signed in' }, 401) });
    render(SettingsPage, { clubId: '7' });
    const link = await screen.findByRole('link', { name: t('applications.session.signIn') });
    expect(link).toHaveAttribute(
      'href',
      '/api/v1/auth/discord/login?next=%2Fapplications%2Fclub%2F7',
    );
  });
});
