// App tests with a stubbed fetch. All data is made up.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import type { Recommendations } from './lib/api';

function recommendations(total: number, pace = 7_600_000): Recommendations {
  return {
    trainer: { trainer_id: null, pace_per_day: pace },
    options: { balance_weight: null, include_closed: false, limit: 50 },
    updated_at: '2026-03-18T09:00:00Z',
    total,
    clubs: [],
  };
}

function stubFetch(response: () => Response) {
  const fetch = vi.fn<typeof globalThis.fetch>(
    () => new Promise((resolve) => setTimeout(() => resolve(response()), 10)),
  );
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

function setUrl(search: string) {
  window.history.replaceState(null, '', `/${search}`);
}

const trainerInput = () => screen.getByLabelText('Trainer ID', { selector: 'input#trainer-id' });

beforeEach(() => {
  setUrl('');
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('App', () => {
  it('searches and puts the search in the URL', async () => {
    const fetch = stubFetch(() => json(recommendations(3)));
    render(App);
    await fireEvent.input(trainerInput(), { target: { value: '123456789012' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));

    expect(await screen.findByTestId('result-summary')).toHaveTextContent('7.6M/day');
    expect(screen.getByText('Found 3 clubs.')).toBeInTheDocument();
    expect(fetch.mock.calls[0]?.[0]).toBe(
      '/api/v1/recommendations?trainer_id=123456789012&limit=100',
    );
    expect(window.location.search).toBe('?trainer_id=123456789012');
  });

  it('runs a search from the URL on load', async () => {
    setUrl('?pace=5000000&include_closed=true');
    const fetch = stubFetch(() => json(recommendations(1, 5_000_000)));
    render(App);
    expect(await screen.findByTestId('result-summary')).toHaveTextContent('5.0M/day');
    expect(screen.getByText('Found 1 club.')).toBeInTheDocument();
    expect(fetch.mock.calls[0]?.[0]).toBe(
      '/api/v1/recommendations?pace=5000000&include_closed=true&limit=100',
    );
    expect(screen.getByLabelText('Daily fan gain (millions)')).toHaveValue('5');
  });

  it('does not search on load without a valid search in the URL', () => {
    setUrl('?trainer_id=abc');
    const fetch = stubFetch(() => json(recommendations(0)));
    render(App);
    expect(fetch).not.toHaveBeenCalled();
  });

  it('shows the server error message', async () => {
    stubFetch(() => json({ error: 'trainer not found' }, 404));
    render(App);
    await fireEvent.input(trainerInput(), { target: { value: '123456789' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('trainer not found');
  });

  it('remembers the trainer ID only when asked', async () => {
    stubFetch(() => json(recommendations(0)));
    const { unmount } = render(App);
    await fireEvent.input(trainerInput(), { target: { value: '123456789' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(localStorage.length).toBe(0);
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
    stubFetch(() => json(recommendations(0)));
    render(App);
    await fireEvent.click(screen.getByLabelText('Remember my trainer ID on this device'));
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(localStorage.getItem('uma-stable-match:trainer-id')).toBeNull();
  });
});
