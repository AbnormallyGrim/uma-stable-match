// App tests with a stubbed fetch. All data is made up.

import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import type { Recommendations } from './lib/api';
import { TOUR_KEY } from './lib/tour';

function recommendations(
  total: number,
  pace = 7_600_000,
  extra: Partial<Recommendations> = {},
): Recommendations {
  return {
    trainer: { trainer_id: null, pace_per_day: pace },
    options: {
      balance_weight: null,
      include_closed: false,
      include_full: false,
      join_styles: null,
      playstyles: null,
      has_discord: false,
      limit: 50,
    },
    updated_at: '2026-03-18T09:00:00Z',
    month: '2026-03',
    coverage: { considered: 340, shortlisted: total, scored: total, pending: 0 },
    total,
    clubs: [],
    your_club: null,
    ...extra,
  };
}

function stubApi(recommendations: () => Response) {
  const fetch = vi.fn<typeof globalThis.fetch>(
    () => new Promise((resolve) => setTimeout(() => resolve(recommendations()), 10)),
  );
  vi.stubGlobal('fetch', fetch);
  const recommendationCalls = () =>
    fetch.mock.calls
      .map(([input]) => String(input))
      .filter((url) => url.startsWith('/api/v1/recommendations'));
  return { fetch, recommendationCalls };
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

function setUrl(search: string) {
  window.history.replaceState(null, '', `/${search}`);
}

const trainerInput = () => screen.getByLabelText('Trainer ID', { selector: 'input#trainer-id' });

// Returning visitors by default, so the tour doesn't cover the page.
beforeEach(() => {
  setUrl('');
  localStorage.clear();
  localStorage.setItem(TOUR_KEY, '1');
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('App', () => {
  it("announces clubs still loading and last month's data", async () => {
    const result = recommendations(2, 7_600_000, {
      updated_at: '2026-03-02T15:00:00Z',
      month: '2026-02',
      coverage: { considered: 340, shortlisted: 7, scored: 2, pending: 5 },
    });
    stubApi(() => json(result));
    render(App);
    await fireEvent.input(trainerInput(), { target: { value: '123456789012' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));

    await screen.findByTestId('result-summary');
    const live = document.querySelector('[aria-live="polite"]');
    expect(live).toHaveTextContent(
      "Found 2 clubs. These results are based on last month's data (February 2026), because the new game month has only just started. 5 more clubs are still loading. Search again in a minute for complete results.",
    );
  });

  it('searches and puts the search in the URL', async () => {
    const { recommendationCalls } = stubApi(() => json(recommendations(3)));
    render(App);
    await fireEvent.input(trainerInput(), { target: { value: '123456789012' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));

    expect(await screen.findByTestId('result-summary')).toHaveTextContent('7.6M/day');
    expect(screen.getByText('Found 3 clubs.')).toBeInTheDocument();
    expect(recommendationCalls()).toEqual([
      '/api/v1/recommendations?trainer_id=123456789012&limit=100',
    ]);
    expect(window.location.search).toBe('?trainer_id=123456789012');
  });

  it('runs a search from the URL on load', async () => {
    setUrl('?pace=5000000&join_style=closed');
    const { recommendationCalls } = stubApi(() => json(recommendations(1, 5_000_000)));
    render(App);
    expect(await screen.findByTestId('result-summary')).toHaveTextContent('5.0M/day');
    expect(screen.getByText('Found 1 club.')).toBeInTheDocument();
    expect(recommendationCalls()).toEqual([
      '/api/v1/recommendations?pace=5000000&join_style=closed&limit=100',
    ]);
    expect(screen.getByLabelText('Daily fan gain (millions)')).toHaveValue('5');
  });

  it('does not search on load without a valid search in the URL', () => {
    setUrl('?trainer_id=abc');
    const { recommendationCalls } = stubApi(() => json(recommendations(0)));
    render(App);
    expect(recommendationCalls()).toEqual([]);
  });

  it('shows the server error message', async () => {
    stubApi(() => json({ error: 'trainer not found' }, 404));
    render(App);
    await fireEvent.input(trainerInput(), { target: { value: '123456789' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(await screen.findByRole('alert')).toHaveTextContent("We couldn't find that trainer");
  });

  it('remembers the trainer ID only when asked', async () => {
    stubApi(() => json(recommendations(0)));
    const { unmount } = render(App);
    await fireEvent.input(trainerInput(), { target: { value: '123456789' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(localStorage.getItem('uma-stable-match:trainer-id')).toBeNull();
    await screen.findByRole('button', { name: 'Find clubs' });

    await fireEvent.click(screen.getByLabelText('Remember my trainer ID on this device'));
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(localStorage.getItem('uma-stable-match:trainer-id')).toBe('123456789');
    await screen.findByRole('button', { name: 'Find clubs' });
    unmount();

    setUrl('');
    render(App);
    expect(trainerInput()).toHaveValue('123456789');
    expect(screen.getByLabelText('Remember my trainer ID on this device')).toBeChecked();
  });

  it('forgets the trainer ID when unticked', async () => {
    localStorage.setItem('uma-stable-match:trainer-id', '123456789');
    stubApi(() => json(recommendations(0)));
    render(App);
    await fireEvent.click(screen.getByLabelText('Remember my trainer ID on this device'));
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(localStorage.getItem('uma-stable-match:trainer-id')).toBeNull();
  });
});

describe('App states', () => {
  it('waits for the first scan and retries', async () => {
    let calls = 0;
    stubApi(() => {
      calls += 1;
      return calls === 1
        ? json({ error: 'no scan results yet; try again shortly' }, 503)
        : json(recommendations(2));
    });
    setUrl('?pace=5000000');
    render(App);
    expect(await screen.findByText('Club data is still loading')).toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Try now' }));
    expect(await screen.findByText('Found 2 clubs.')).toBeInTheDocument();
    expect(calls).toBe(2);
  });

  it('shows when the club data was updated', async () => {
    stubApi(() => json(recommendations(3)));
    setUrl('?pace=5000000');
    render(App);
    expect(await screen.findByTestId('data-age')).toHaveTextContent('Club data updated');
  });

  it('shows a rate limit countdown', async () => {
    stubApi(
      () =>
        new Response(JSON.stringify({ error: 'too many requests; try again in 7 seconds' }), {
          status: 429,
          headers: { 'Content-Type': 'application/json', 'Retry-After': '7' },
        }),
    );
    setUrl('?pace=5000000');
    render(App);
    expect(await screen.findByText('Too many searches')).toBeInTheDocument();
    expect(screen.getByText('Please wait 7 seconds before searching again.')).toBeInTheDocument();
  });

  it('starts the tour on a first visit, and not again once skipped', async () => {
    localStorage.removeItem(TOUR_KEY);
    stubApi(() => json(recommendations(0)));
    const { unmount } = render(App);
    const dialog = screen.getByRole('dialog', { name: 'Welcome to Uma Stable Match' });
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Skip tour' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(localStorage.getItem(TOUR_KEY)).toBe('1');
    unmount();

    render(App);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('waits with the tour when a visitor arrives with a search', () => {
    localStorage.removeItem(TOUR_KEY);
    setUrl('?pace=5000000');
    stubApi(() => json(recommendations(1, 5_000_000)));
    render(App);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(localStorage.getItem(TOUR_KEY)).toBeNull();
  });

  it('brings the tour back from the help button', async () => {
    stubApi(() => json(recommendations(0)));
    render(App);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await fireEvent.click(screen.getByRole('button', { name: 'Take the tour' }));
    expect(screen.getByRole('dialog', { name: 'Welcome to Uma Stable Match' })).toBeInTheDocument();
  });
});
