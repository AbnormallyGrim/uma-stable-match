// Results component tests. All data is made up.

import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import Results from './Results.svelte';
import { club, recommendations } from './test-data';

const now = new Date('2026-03-18T12:00:00Z');

describe('Results', () => {
  it('summarises the search', () => {
    render(Results, { result: recommendations({ total: 42 }), now });
    const summary = screen.getByTestId('result-summary');
    expect(within(summary).getByText('7.6M/day')).toBeInTheDocument();
    expect(within(summary).getByText('42')).toBeInTheDocument();
    expect(within(summary).getByText('3 hours ago')).toHaveAttribute(
      'datetime',
      '2026-03-18T09:00:00Z',
    );
    expect(within(summary).getByText('Off')).toBeInTheDocument();
  });

  it('shows the balance weight when balance is on', () => {
    render(Results, {
      result: recommendations({
        options: { balance_weight: 0.5, include_closed: false, limit: 50 },
      }),
      now,
    });
    expect(screen.getByText('Weight 0.50')).toBeInTheDocument();
  });

  it('lists up to ten top picks in order', () => {
    const clubs = Array.from({ length: 12 }, (_, i) =>
      club({ circle_id: i + 1, name: `Club ${i + 1}`, url: `https://uma.moe/circles/${i + 1}` }),
    );
    render(Results, { result: recommendations({ total: 12, clubs }), now });
    const picks = screen.getByRole('list');
    const items = within(picks).getAllByRole('listitem');
    expect(items).toHaveLength(10);
    expect(within(items[0]!).getByRole('link')).toHaveTextContent('Club 1');
    expect(within(items[9]!).getByRole('link')).toHaveTextContent('Club 10');
    expect(within(picks).queryByText('Club 11')).not.toBeInTheDocument();
    expect(within(screen.getByRole('table')).getAllByRole('link')).toHaveLength(12);
  });

  it('says when nothing matched', () => {
    render(Results, { result: recommendations(), now });
    expect(screen.getByText('No clubs matched your search.')).toBeInTheDocument();
  });
});
