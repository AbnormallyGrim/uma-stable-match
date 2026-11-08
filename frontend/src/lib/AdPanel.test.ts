// AdPanel tests. All data is made up.

import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { LANGUAGE_CODES, type MyAd } from './ads';
import AdPanel from './AdPanel.svelte';
import { t } from './i18n';

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

const mine: MyAd = {
  trainerName: 'Example Trainer',
  trainerId: '123456789',
  currentClub: 'Other Club',
  pace: 6_400_000,
  fansThisMonth: 48_000_000,
  rankMin: 3,
  rankMax: 8,
  discord: 'required',
  language: 'en',
  postedAt: '2026-11-01T00:00:00Z',
  visibleUntil: '2026-11-08T00:00:00Z',
  expiresAt: '2026-12-01T00:00:00Z',
  onBoard: true,
  invites: [{ clubId: '7', clubName: 'Example Club', sentAt: '2026-11-02T00:00:00Z' }],
};

/** Answers each request with the next response, remembering what was asked. */
function stubFetch(...responses: Response[]) {
  const queue = [...responses];
  const fetch = vi.fn(() => Promise.resolve(queue.shift() ?? json(null)));
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

function calls(fetch: ReturnType<typeof vi.fn>): [string, string | undefined][] {
  return (fetch.mock.calls as unknown as [string, RequestInit][]).map(([uri, init]) => [
    `${init?.method ?? 'GET'} ${uri}`,
    init?.body as string | undefined,
  ]);
}

/** Opens a dropdown and picks one of its options, as a visitor would. */
async function choose(label: string, option: string) {
  await fireEvent.click(await screen.findByRole('combobox', { name: label }));
  await fireEvent.click(screen.getByRole('option', { name: option }));
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('AdPanel', () => {
  it('explains the ad and offers the form when there is none', () => {
    render(AdPanel, { ad: null });
    expect(screen.getByText(t('ad.lede'))).toBeInTheDocument();
    expect(screen.getByRole('button', { name: t('ad.form.post') })).toBeInTheDocument();
  });

  it('posts the ad with the choices made, and says so', async () => {
    const fetch = stubFetch(json(mine));
    const onchange = vi.fn();
    render(AdPanel, { ad: null, onchange });
    await choose(t('ad.form.rankMax'), 'C');
    await choose(t('ad.form.rankMin'), 'C');
    await choose(t('ad.form.discord'), t('ad.discord.required'));
    await choose(t('ad.form.language'), t('ad.language.ko'));
    await fireEvent.click(screen.getByRole('button', { name: t('ad.form.post') }));
    expect(await screen.findByText('Example Trainer')).toBeInTheDocument();
    const [call, body] = calls(fetch)[0]!;
    expect(call).toBe('PUT /api/v1/discord/ads/me');
    expect(JSON.parse(body!)).toEqual({
      rankMin: 3,
      rankMax: 3,
      discord: 'required',
      language: 'ko',
    });
    expect(onchange).toHaveBeenCalledTimes(1);
  });

  it('offers every language, with any last', async () => {
    render(AdPanel, { ad: null });
    await fireEvent.click(screen.getByRole('combobox', { name: t('ad.form.language') }));
    const names = screen.getAllByRole('option').map((option) => option.textContent?.trim());
    expect(names).toEqual(LANGUAGE_CODES.map((code) => t(`ad.language.${code}`)));
  });

  it('wants the lowest club rank at or below the highest', async () => {
    const fetch = stubFetch();
    render(AdPanel, { ad: null });
    await choose(t('ad.form.rankMin'), 'S');
    await choose(t('ad.form.rankMax'), 'C+');
    expect(screen.getByText(t('ad.form.rankError'))).toBeInTheDocument();
    expect(screen.getByRole('button', { name: t('ad.form.post') })).toBeDisabled();
    expect(fetch).not.toHaveBeenCalled();
  });

  it('starts on the three lowest club ranks', () => {
    render(AdPanel, { ad: null });
    expect(screen.getByRole('combobox', { name: t('ad.form.rankMin') })).toHaveTextContent('D');
    expect(screen.getByRole('combobox', { name: t('ad.form.rankMax') })).toHaveTextContent('C');
  });

  it('allows three club ranks at most', async () => {
    const fetch = stubFetch(json({}));
    render(AdPanel, { ad: null });
    await choose(t('ad.form.rankMin'), 'C');
    await choose(t('ad.form.rankMax'), 'B');
    expect(screen.queryByText(t('ad.form.rankSpanError'))).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: t('ad.form.post') })).toBeEnabled();
    await choose(t('ad.form.rankMax'), 'B+');
    expect(screen.getByText(t('ad.form.rankSpanError'))).toBeInTheDocument();
    expect(screen.getByRole('button', { name: t('ad.form.post') })).toBeDisabled();
    expect(fetch).not.toHaveBeenCalled();
  });

  it('sends a trainer without a link to Settings', async () => {
    stubFetch(json({ error: 'link your trainer ID in Settings first' }, 403));
    render(AdPanel, { ad: null });
    await fireEvent.click(screen.getByRole('button', { name: t('ad.form.post') }));
    expect(await screen.findByText(t('ad.form.noTrainer'))).toBeInTheDocument();
    expect(screen.getByRole('link', { name: t('ad.noTrainerLink') })).toHaveAttribute(
      'href',
      '/settings',
    );
  });

  it('shows the ad, its numbers and how long it stays', () => {
    render(AdPanel, { ad: mine });
    expect(screen.getByText('Example Trainer')).toBeInTheDocument();
    expect(screen.getByText('Other Club')).toBeInTheDocument();
    expect(screen.getByText('6.4M/day')).toBeInTheDocument();
    expect(screen.getByText('C to A+')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent(/On the board until/);
  });

  it('tells a trainer whose ad left the board to renew it', () => {
    render(AdPanel, { ad: { ...mine, onBoard: false } });
    expect(screen.getByText(/Renew it to show it again/)).toBeInTheDocument();
  });

  it('renews, edits and deletes the ad', async () => {
    const fetch = stubFetch(
      json({ ...mine, pace: 7_000_000 }),
      new Response(null, { status: 204 }),
    );
    const onchange = vi.fn();
    render(AdPanel, { ad: mine, onchange });
    await fireEvent.click(screen.getByRole('button', { name: t('ad.renew') }));
    expect(await screen.findByText('7.0M/day')).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: t('ad.edit') }));
    expect(screen.getByRole('button', { name: t('ad.form.save') })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: t('ad.form.cancel') }));

    await fireEvent.click(screen.getByRole('button', { name: t('ad.delete') }));
    expect(await screen.findByRole('button', { name: t('ad.form.post') })).toBeInTheDocument();
    expect(calls(fetch).map(([call]) => call)).toEqual([
      'POST /api/v1/discord/ads/me/renew',
      'DELETE /api/v1/discord/ads/me',
    ]);
    expect(onchange).toHaveBeenCalledTimes(2);
  });

  it('lists invitations to apply to or dismiss', async () => {
    const fetch = stubFetch(new Response(null, { status: 204 }));
    render(AdPanel, { ad: mine });
    expect(
      screen.getByRole('link', { name: t('ad.invites.applyTo', { club: 'Example Club' }) }),
    ).toHaveAttribute('href', '/api/v1/discord/apply?club=7');
    await fireEvent.click(
      screen.getByRole('button', { name: t('ad.invites.dismissFrom', { club: 'Example Club' }) }),
    );
    await waitFor(() => expect(screen.getByText(t('ad.invites.empty'))).toBeInTheDocument());
    expect(calls(fetch)[0]![0]).toBe('DELETE /api/v1/discord/ads/me/invites/7');
  });

  it('says when an action fails', async () => {
    stubFetch(json({ error: 'down' }, 502));
    render(AdPanel, { ad: mine });
    await fireEvent.click(screen.getByRole('button', { name: t('ad.renew') }));
    expect(await screen.findByText(t('ad.actionError'))).toBeInTheDocument();
  });
});
