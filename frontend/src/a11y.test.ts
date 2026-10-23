// axe-core accessibility checks for the main views and states. All data is made up.

import { render, screen, fireEvent } from '@testing-library/svelte';
import axe from 'axe-core';
import { afterEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import type { Recommendations } from './lib/api';
import HowToUse from './lib/HowToUse.svelte';
import ResultsTable from './lib/ResultsTable.svelte';
import SearchForm from './lib/SearchForm.svelte';
import SearchStatus from './lib/SearchStatus.svelte';
import { DEFAULT_SEARCH } from './lib/search';
import { club, recommendations } from './lib/test-data';
import ThemeToggle from './lib/ThemeToggle.svelte';

// axe's colour-contrast rule needs real rendering, so contrast.test.ts covers it instead.
// The landmark (region) rule only applies to whole-page checks.
async function violations(node: Element, wholePage = false) {
  const result = await axe.run(node, {
    rules: {
      'color-contrast': { enabled: false },
      region: { enabled: wholePage },
    },
  });
  return result.violations.map(
    (v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`,
  );
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

function stubApi(recommendationsResponse: () => Response) {
  vi.stubGlobal(
    'fetch',
    vi.fn(() => Promise.resolve(recommendationsResponse())),
  );
}

const balancedClubs = [1, 2, 3].map((id) =>
  club({
    circle_id: id,
    name: `Club ${id}`,
    url: `https://uma.moe/circles/${id}`,
    join_style: id === 3 ? 'closed' : 'open',
    balance: {
      score: 80,
      gini: 0.2,
      passengers: 1,
      counted: 20,
      bottom_quarter_per_day: 3_000_000,
      top3_share: 0.3,
    },
  }),
);

const withResults: Recommendations = recommendations({
  total: 3,
  clubs: balancedClubs,
  options: { balance_weight: 0.5, include_closed: true, limit: 100 },
});

afterEach(() => {
  vi.unstubAllGlobals();
  window.history.replaceState(null, '', '/');
});

describe('accessibility', () => {
  it('has no violations on the empty page', async () => {
    stubApi(() => json(withResults));
    render(App);
    await screen.findByRole('heading', { name: 'Uma Stable Match' });
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations with results, cards and the table', async () => {
    window.history.replaceState(null, '', '/?pace=7600000&balance_weight=0.5&include_closed=true');
    stubApi(() => json(withResults));
    render(App);
    await screen.findByRole('table');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations with coverage and last-month notices', async () => {
    window.history.replaceState(null, '', '/?pace=7600000');
    stubApi(() =>
      json({
        ...withResults,
        updated_at: '2026-03-02T15:00:00Z',
        month: '2026-02',
        coverage: { shortlisted: 9, scored: 3, pending: 6 },
      }),
    );
    render(App);
    await screen.findAllByTestId('result-notice');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations when a search fails', async () => {
    window.history.replaceState(null, '', '/?trainer_id=123456789');
    stubApi(() => json({ error: 'trainer not found' }, 404));
    render(App);
    await screen.findByRole('alert');
    expect(await violations(document.body, true)).toEqual([]);
  });

  it('has no violations in the form with a validation error', async () => {
    const { container } = render(SearchForm, {
      initial: { ...DEFAULT_SEARCH, trainerId: '123' },
      remember: false,
      onsearch: vi.fn(),
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Find clubs' }));
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the pace form with balance on', async () => {
    const { container } = render(SearchForm, {
      initial: { ...DEFAULT_SEARCH, mode: 'pace', paceMillions: '7.6', balance: true },
      remember: false,
      onsearch: vi.fn(),
    });
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the sorted table', async () => {
    const { container } = render(ResultsTable, {
      clubs: balancedClubs,
      balanceOn: true,
      includeClosed: true,
    });
    await fireEvent.click(screen.getByRole('button', { name: /^Median/ }));
    expect(await violations(container)).toEqual([]);
  });

  it.each([
    { phase: 'loading' as const },
    { phase: 'waiting-for-scan' as const, retryIn: 5 },
    { phase: 'rate-limited' as const, retryIn: 3 },
    {
      phase: 'error' as const,
      message: { title: "Couldn't reach uma.moe", detail: 'Try again.', canRetry: true },
    },
  ])('has no violations in the $phase status', async (status) => {
    const { container } = render(SearchStatus, { status, onretry: vi.fn() });
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the open instructions', async () => {
    const { container } = render(HowToUse);
    container.querySelector('details')!.open = true;
    expect(await violations(container)).toEqual([]);
  });

  it('has no violations in the theme switch', async () => {
    const { container } = render(ThemeToggle, { storage: localStorage });
    expect(await violations(container)).toEqual([]);
  });
});
